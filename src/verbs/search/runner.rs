//! `bmo runner search` (`docs/research-runs.md`, "The launch and the runner"): one search carried to its end
//! apart from whoever launched it. It reads the question its launcher wrote and deletes the file, carries one
//! research run apart on the search line — or, for a page asked for whole, has the page read with no model at
//! all — and hands what it found back to the chat's own session as a quiet instruction queued in
//! bmo-receiver, so the agent in that room answers in its own voice. It never speaks to the room itself.
//!
//! Every ending is written once: the run's row closed with how it ended, the search closed with the same
//! word and what it found put away beside it, and the room told. A stop from outside salvages the last thing
//! the run wrote in the result's own structure and hands it over the way an answer is; a run that wrote
//! nothing is told as stopped, never as broken.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use clap::Args;
use sea_orm::DatabaseConnection;

use super::document::leaves_no_document;
use super::gallery::{looked_at, only_looked_at};
use super::images::{CapturedFigure, KEEPING_DEADLINE, Keeping, with_kept};
use super::import::{figure_address, imported_result, title_of};
use super::record::{Ending as RecordEnding, record_run};
use super::request::{Depth, Request, depth, depth_names, read_request, search_key, task};
use super::result::{
  Ending, failure_instruction, follow_up_line, imported_instruction, is_answered, is_search_result,
  lift_narration, result_instruction, salvaged_result, stopped_instruction,
};
use super::share::{is_long, room_link};
use super::web::WebLayer;
use crate::clients::agency::Instructions;
use crate::clients::receiver::{HandBack, Said};
use crate::clients::voice::{Briefing, launched};
use crate::helpers::lines::Trees;
use crate::now_ms;
use crate::plumbing::signal::Stop;
use crate::plumbing::verb::{Answer, Shell, Verb, VerbError};
use crate::verbs::archive_docs::Documents;
use crate::verbs::research::admission::{
  Branches, Demand, PLACE_POLL, await_place, parse_demand, unit_running,
};
use crate::verbs::research::run::{Research, Road, assistant_texts};
use crate::verbs::research::{
  Profile, Told, instruction_line, profile as read_profile, reclaim_leases, reserve_places, work_knowledge,
};

/// How the runner is called, said where it is called wrong.
pub fn usage() -> String {
  format!(
    "usage: bmo runner search <chatId> <queryFile> [{}] [runId] [unit] [startedBy] [searchId] [demand] [profile]",
    depth_names()
  )
}

/// What a run stopped from outside is closed with.
pub const RUN_STOPPED: &str = "stopped";

// How a run that wrote a result saying itself it did not answer is closed.
const RUN_UNANSWERED: &str = "unanswered";

// The instructions bmo-agency composes for the search line.
const INSTRUCTIONS: &str = "search";

// The kind of ending a search hands its room's session.
const SORT: &str = "search";

// How long work under way is given to come to rest once a stop is heard: a run apart is stopped through
// bmo-agency, and a run not yet started never is.
const STOP_GRACE: Duration = Duration::from_secs(30);

/// The names of the work a run left running, the first `limit` of them, with how many more there were.
pub fn live_task_names(tasks: &[String], limit: usize) -> String {
  let shown: Vec<&str> = tasks.iter().take(limit).map(|task| task.trim()).collect();
  let more = if tasks.len() > shown.len() {
    format!(" and {} more", tasks.len() - shown.len())
  } else {
    String::new()
  };

  format!("{}{more}", shown.join(", "))
}

/// One run as its launcher started it.
pub struct Started {
  pub chat_id: i64,
  pub query_path: PathBuf,
  pub depth: Depth,
  /// The row the launcher opened; a run with none records nothing and opens no branch.
  pub run_id: Option<i64>,
  /// The unit it runs as, its log's name; a run with none writes no log.
  pub unit: String,
  /// The search it answers; a run with none records nothing of it.
  pub search_id: Option<i64>,
  pub demand: Demand,
  /// The settings it launched on, as the launcher's copy; none reads them now.
  pub profile: Option<String>,
  /// The road the question came in on, which it is answered on.
  pub via: Option<String>,
  /// The registration its research is kept on.
  pub kept_on: Option<String>,
  /// The agent whose turn asked it, which what it comes to goes back to.
  pub asked_by: Option<String>,
}

// How the work came out where it did not hand anything back itself.
enum Ended {
  Done(Vec<String>),
  Stopped,
}

// What the run holds while it works.
struct Run<'g> {
  shell: &'g Shell,
  db: DatabaseConnection,
  started: &'g Started,
  research: Research,
  query: String,
  /// The documents a room is handed the address of; none where the settings name no console address.
  documents: Option<Documents>,
  owner_room: bool,
  artifacts: PathBuf,
  web: WebLayer,
  handed_back: bool,
  // What it found, in the words it wrote them in, once it has written anything worth keeping.
  found: Option<String>,
}

impl Run<'_> {
  async fn hand_back(&mut self, text: &str) {
    self.handed_back = true;

    let back = HandBack {
      agent: self.started.asked_by.clone(),
      chat: self.started.chat_id,
      said: Said {
        order: text.to_owned(),
      },
      sort: SORT.to_owned(),
      via: self.started.via.clone(),
      run: self.started.run_id,
    };

    if let Err(why) = self.shell.receiver.hand_back(&back).await {
      eprintln!("the search's ending could not be handed to the session: {why}");
    }
  }

  // The run's ending, which is the search's too: what it found put away in the same breath.
  async fn close(&mut self, outcome: &str) {
    if !self.research.close(outcome).await {
      return;
    }

    let Some(search_id) = self.started.search_id else { return };
    let runs = match self.shell.agency.runs_of_search(search_id).await {
      Ok(runs) => runs,
      Err(why) => {
        eprintln!("the runs that carried search {search_id} could not be listed: {why}");
        self.started.run_id.into_iter().collect()
      }
    };
    let stream = self.research.streamed();
    let ending = RecordEnding {
      search_id,
      run_id: self.started.run_id,
      runs: &runs,
      result: self.found.as_deref(),
      stream: &stream,
      outcome,
      artifacts: &self.artifacts,
      wording: &self.shell.wording,
      zone: self.shell.settings.time.zone,
    };

    if let Err(why) = record_run(&self.db, &ending).await {
      eprintln!("search {search_id} could not be written down: {}", why.detail);
    }
  }

  // Every way the run ends short of an answer: the reason written in the run's log, and the room told it
  // failed and that nothing more is coming.
  async fn failed(&mut self, reason: &str) -> VerbError {
    let said = format!("search run failed: {reason}");

    self.research.note(said.clone()).await;
    self.close("failed").await;

    if !self.handed_back {
      self.hand_back(&failure_instruction(&self.query)).await;
    }

    VerbError::Ended { code: 1, said }
  }

  // This repository's own copies of the pictures a result names, taken before the room is handed anything;
  // a failure costs the pictures their permanence and the answer nothing.
  async fn kept_pictures(
    &self,
    result: &str,
    already: BTreeMap<String, String>,
  ) -> BTreeMap<String, String> {
    let Some(search_id) = self.started.search_id else { return already };
    let keeping = Keeping {
      db: &self.db,
      search_id,
      artifacts: &self.artifacts,
      deadline: KEEPING_DEADLINE,
    };

    keeping.result_pictures(&self.web, result, already).await
  }

  // What the run wrote with every picture it never looked at taken out, and its log told how many went; a
  // stream that cannot be read leaves what it wrote as it was.
  async fn looked_at_only(&mut self, written: String) -> String {
    let Some(looked) = looked_at(&self.research.streamed()) else { return written };
    let (result, dropped) = only_looked_at(&written, &looked);

    if dropped > 0 {
      self
        .research
        .note(format!("{dropped} pictures the run never looked at taken out of its result"))
        .await;
    }

    result
  }

  // A result's instruction with the search's number after it, which a follow-up on the same things names.
  fn numbered(&self, instruction: String) -> String {
    match self.started.search_id {
      Some(search_id) => format!("{instruction}\n{}", follow_up_line(search_id)),
      None => instruction,
    }
  }

  // The address the room is handed for the search's document, where it left one.
  async fn document_link(&self) -> Option<String> {
    room_link(self.documents.as_ref()?, self.started.search_id?, self.owner_room).await
  }

  // A page asked for whole: read, not researched. Its figures are kept beside the pictures it names, and the
  // room is told the page is in, with the link to it.
  async fn import(&mut self, source: &str) -> Result<Vec<String>, VerbError> {
    let page = match self.web.page(source).await {
      Ok(page) => page,
      Err(why) => return Err(self.failed(&format!("the page could not be imported: {why}")).await),
    };
    let words = &self.shell.wording.search;
    let missing: Vec<_> = page.figures.iter().filter(|figure| figure.png.is_none()).collect();
    let result = imported_result(source, &page, &missing, &words.import);
    let captured: Vec<CapturedFigure> = page
      .figures
      .iter()
      .filter_map(|figure| {
        figure.png.clone().map(|png| CapturedFigure {
          address: figure_address(&page, figure),
          png,
          svg: figure.svg.clone(),
        })
      })
      .collect();
    let figures_captured = captured.len();

    self.found = Some(result.clone());
    self
      .research
      .note(format!(
        "imported {source} whole: {} characters, {figures_captured} figures captured",
        result.encode_utf16().count()
      ))
      .await;

    let figures = match self.started.search_id {
      Some(search_id) => {
        Keeping {
          db: &self.db,
          search_id,
          artifacts: &self.artifacts,
          deadline: KEEPING_DEADLINE,
        }
        .captured(captured)
        .await
      }
      None => BTreeMap::new(),
    };
    let kept = self.kept_pictures(&result, figures).await;

    self.close("done").await;

    let link = self.document_link().await;
    let text =
      imported_instruction(&self.query, &title_of(&page), kept.len(), link.as_deref(), &words.archive_name);

    self.hand_back(&text).await;

    Ok(vec![format!(
      "page {source} imported for chat {}; {} pictures kept",
      self.started.chat_id,
      kept.len()
    )])
  }

  // The research run itself, from the knowledge it is handed to the answer handed back.
  async fn research(&mut self, profile: &Profile, request: &Request) -> Result<Ended, VerbError> {
    let shell = self.shell;
    let started = self.started;
    let agency = Arc::clone(&shell.agency);

    // Places a research run left behind when its process died are given back before this run asks.
    if let Err(why) = reclaim_leases(&*agency, unit_running).await {
      eprintln!("the pool's abandoned places could not be reclaimed: {why}");
    }

    let knowledge =
      work_knowledge(&shell.gate, started.chat_id, &[instruction_line(&self.query)], "search").await;

    if !knowledge.paths.is_empty() {
      self.research.note(format!("knowledge attached: {}", knowledge.paths.join(", "))).await;
    }

    let code = shell.settings.prompts.code.clone().ok_or_else(|| {
      VerbError::Refused("the prompt tree is not set: set code under [prompts] in the settings".to_owned())
    })?;
    let asked = task(
      &Trees { code },
      request,
      started.depth.name,
      profile.max_children(),
      now_ms() / 1000,
      shell.settings.time.zone,
    )
    .map_err(|why| VerbError::Refused(why.detail))?;
    let prompt = [Some(asked), knowledge.block].into_iter().flatten().collect::<Vec<_>>().join("\n\n");

    self.research.count_run().await;

    let road = match &started.kept_on {
      Some(registration) => Road::Kept(registration.clone()),
      None => Road::Key(search_key(started.depth.name)),
    };

    // A run that may open branches reserves their seats before the model is launched; an explicit deeper
    // search waits for its first seat before the model starts at all.
    if let Some(owner) = self.research.owner().map(str::to_owned)
      && profile.max_children() > 0
    {
      let branches = Branches {
        owner: &owner,
        source: "search",
        role: profile.child_role(),
        count: profile.max_children(),
      };

      if started.demand == Demand::Explicit {
        let flag = self.research.stop_flag();
        let place = await_place(&*agency, &branches, profile.wait_limit_ms(), PLACE_POLL, &|| {
          flag.load(Ordering::SeqCst)
        })
        .await;

        if self.research.stopping() {
          return Ok(Ended::Stopped);
        }

        if place.reservation.is_none() {
          let why = place.reason.unwrap_or_default();

          return Err(
            self
              .failed(&format!(
                "the explicit {} search got no place in the common co-worker pool: {why}",
                started.depth.name
              ))
              .await,
          );
        }

        self
          .research
          .note(format!(
            "pool seat held after {}s before the run was launched",
            (place.waited_ms as f64 / 1000.0).round()
          ))
          .await;
      } else if let Err(why) =
        reserve_places(&*agency, branches.owner, branches.source, branches.role, branches.count).await
      {
        self.research.note(format!("branch seats were not reserved: {why}")).await;
      }
    }

    // What the search line's web tools are told for every run of it: the branch tool, the picture viewer and
    // the rulebooks read on request.
    let standing = Told {
      branch_tool: true,
      image_tool: true,
      rulebooks: shell.settings.research.search.on_request.clone(),
      ..Told::default()
    };
    let instructions = Instructions {
      name: INSTRUCTIONS.to_owned(),
      text: None,
    };
    let reply = match self.research.ask(standing, &road, profile, instructions, &prompt).await {
      Ok(reply) => reply,
      Err(_) if self.research.stopping() => return Ok(Ended::Stopped),
      Err(why) => return Err(self.failed(&why).await),
    };

    // Whether the run finished is settled by what it left running, not by what it says.
    if !reply.live_tasks.is_empty() {
      self.close("live-tasks").await;

      return Err(
        self
          .failed(&format!(
            "the run ended with its own work still running: {}",
            live_task_names(&reply.live_tasks, 4)
          ))
          .await,
      );
    }

    // A deep dive's narration is no finding of the document and nothing the session is handed.
    let (result, narration) = lift_narration(reply.text.trim());

    if result.is_empty() || !is_search_result(&result) {
      let head: String = result.chars().take(300).collect();

      return Err(self.failed(&format!("the run ended without a result: {head}")).await);
    }

    let result = self.looked_at_only(result).await;

    self.research.count_run().await;
    self.found = Some(result.clone());

    let kept = self.kept_pictures(&result, BTreeMap::new()).await;
    // The run's own word on its own answer, the one reader of its ANSWERED line.
    let answered = is_answered(&result);

    self.close(if answered { "done" } else { RUN_UNANSWERED }).await;

    let long = is_long(&result);
    let link = if long { self.document_link().await } else { None };
    let ending = if answered { Ending::Answered } else { Ending::Unanswered };
    let instruction =
      result_instruction(&self.query, &with_kept(&result, &kept), ending, long, link.as_deref());

    self.hand_back(&self.numbered(instruction)).await;

    let mut lines = vec![format!(
      "search for chat {} handed back to its session{}",
      started.chat_id,
      if answered { "" } else { " as an unanswered result" }
    )];

    // Once the room has its answer and the document is written, a deep dive's narration is spoken apart and
    // kept as the document's sound; a briefing bmo-voice refuses leaves the answer and the document as they
    // are.
    if let (Some(narration), Some(search_id)) = (narration, started.search_id)
      && started.depth.deep_dive
      && !leaves_no_document(&result)
    {
      lines.push(self.narrate(search_id, &narration).await);
    }

    Ok(Ended::Done(lines))
  }

  // A deep dive's narration handed to bmo-voice, which keeps it as the document's sound.
  async fn narrate(&self, search_id: i64, narration: &str) -> String {
    let chat_id = self.started.chat_id;
    let briefing = Briefing {
      chat_id,
      text: narration.to_owned(),
      started_by: format!("search:{search_id}"),
      document: Some(search_id),
    };

    match self.shell.voice.brief(&briefing).await {
      Ok(Ok(unit)) => {
        format!("narration of document {search_id} is being spoken: {}", launched(&unit, chat_id))
      }
      Ok(Err(refused)) => format!("narration of document {search_id} not spoken: voice: {refused}"),
      Err(why) => {
        eprintln!("narration of document {search_id} could not be launched: {why}");
        format!("narration of document {search_id} not spoken")
      }
    }
  }

  // A stop from outside: what the run had written in the result's own structure handed to the room the way an
  // answer is, or, where it wrote nothing, the stop itself.
  async fn stopped(&mut self, reason: &str) -> Vec<String> {
    let chat_id = self.started.chat_id;
    let salvaged = match salvaged_result(&assistant_texts(&self.research.streamed())) {
      Some(written) => Some(self.looked_at_only(lift_narration(&written).0).await),
      None => None,
    };

    self.found = salvaged.clone();

    let Some(salvaged) = salvaged else {
      self.close(RUN_STOPPED).await;

      if !self.handed_back {
        self.hand_back(&stopped_instruction(&self.query)).await;
      }

      return vec![format!("search for chat {chat_id} was {reason} before it had written anything")];
    };
    let kept = self.kept_pictures(&salvaged, BTreeMap::new()).await;

    self.close(RUN_STOPPED).await;

    let instruction =
      result_instruction(&self.query, &with_kept(&salvaged, &kept), Ending::Stopped, false, None);

    self.hand_back(&self.numbered(instruction)).await;

    vec![format!("search for chat {chat_id} was {reason}; what it had written was handed back")]
  }
}

/// The run carried to its end: the lines it says of how it went, or the failure it ended with once the room
/// was told. `stop` is the stop from outside, which names the signal that came.
pub async fn search_runner(
  shell: &Shell,
  started: &Started,
  stop: impl Future<Output = &'static str>,
) -> Result<Vec<String>, VerbError> {
  let db = shell.documents().await?.clone();
  let research = &shell.settings.research;
  let key = search_key(started.depth.name);
  let profile = match &started.profile {
    Some(snapshot) => Profile::parse(snapshot, research, &key),
    None => read_profile(&db, research, &key).await.map_err(|why| why.detail),
  }
  .map_err(VerbError::Refused)?;
  let owner_room = shell.settings.documents.room == Some(started.chat_id);
  let request = read_request(&std::fs::read_to_string(&started.query_path).unwrap_or_default());

  let _ = std::fs::remove_file(&started.query_path);

  let documents = shell
    .settings
    .console
    .address
    .clone()
    .map(|address| Documents::of_shell(shell, db.clone(), address));
  let mut run = Run {
    shell,
    db: db.clone(),
    started,
    research: Research::new(
      Arc::clone(&shell.agency),
      "search",
      started.chat_id,
      started.run_id,
      &started.unit,
    ),
    query: request.query.clone(),
    documents,
    owner_room,
    artifacts: shell.settings.files.root.join("searches"),
    web: WebLayer::new(Arc::new(shell.tool_host.clone())),
    handed_back: false,
    found: None,
  };
  let flag = run.research.stop_flag();
  let (ended, signal) = {
    let work = async {
      match request.options.source.clone() {
        Some(source) => run.import(&source).await.map(Ended::Done),
        None => run.research(&profile, &request).await,
      }
    };

    tokio::pin!(work);

    tokio::select! {
      ended = &mut work => (Some(ended), None),
      signal = stop => {
        flag.store(true, Ordering::SeqCst);

        (tokio::time::timeout(STOP_GRACE, &mut work).await.ok(), Some(signal))
      }
    }
  };

  match (ended, signal) {
    (Some(Ok(Ended::Done(lines))), _) => Ok(lines),
    (Some(Err(why)), _) => Err(why),
    (_, signal) => Ok(run.stopped(&format!("stopped by {}", signal.unwrap_or("SIGTERM"))).await),
  }
}

/// What `bmo runner search` is given, in the order its launcher writes them.
#[derive(Args, Debug)]
pub struct RunnerArgs {
  #[arg(allow_hyphen_values = true)]
  chat_id: String,
  query_path: PathBuf,
  depth: Option<String>,
  run_id: Option<String>,
  unit: Option<String>,
  started_by: Option<String>,
  search_id: Option<String>,
  demand: Option<String>,
  profile: Option<String>,
  /// The road the question came in on, which the result is answered on.
  #[arg(long)]
  via: Option<String>,
  /// The registration the run's research is kept on.
  #[arg(long)]
  kept_on: Option<String>,
  /// The agent whose turn asked it, which what it comes to goes back to.
  #[arg(long)]
  asked_by: Option<String>,
}

impl RunnerArgs {
  /// The run these words start, refused where they name no room.
  pub fn started(self) -> Result<Started, VerbError> {
    let number = |text: Option<&str>| text.and_then(|text| text.trim().parse::<i64>().ok());
    let chat_id = number(Some(&self.chat_id)).ok_or_else(|| VerbError::Refused(usage()))?;

    Ok(Started {
      chat_id,
      query_path: self.query_path,
      depth: depth(self.depth.as_deref()),
      run_id: number(self.run_id.as_deref()),
      unit: self.unit.unwrap_or_default(),
      search_id: number(self.search_id.as_deref()),
      demand: parse_demand(self.demand.as_deref()).unwrap_or(Demand::Optional),
      profile: self.profile.filter(|profile| !profile.is_empty()),
      via: self.via.filter(|via| !via.trim().is_empty()),
      kept_on: self.kept_on.filter(|kept_on| !kept_on.trim().is_empty()),
      asked_by: self.asked_by.filter(|asked_by| !asked_by.trim().is_empty()),
    })
  }
}

/// `runner search`.
pub struct SearchRunner;

impl Verb for SearchRunner {
  const NAME: &'static str = "runner search";
  const ALIASES: &'static [&'static str] = &["search-runner"];
  const ABOUT: &'static str = "The run carrying one search to its end and handing what it found back to the \
                               chat's session.";

  type Args = RunnerArgs;

  async fn run(&self, args: RunnerArgs, shell: &Shell) -> Result<Answer, VerbError> {
    let started = args.started()?;
    // The stop is listened for before anything else, so a stop that comes while the run is set up is heard.
    let mut stop = Stop::listen()?;

    search_runner(shell, &started, async move { stop.next().await }).await.map(Answer::from)
  }
}

crate::verb!(SearchRunner);
