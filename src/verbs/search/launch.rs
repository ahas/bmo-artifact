//! `bmo search` (`docs/research-runs.md`, "The launch and the runner"): a question set off as a search run of
//! its own, which answers the room by itself. The launcher checks what it was asked, asks bmo-receiver who
//! asked, settles the depth the pool lets it run at, writes the question to a file only the owner can read,
//! opens the search's row here and the run's in bmo-job's ledger, and has bmo-job launch `bmo runner search`.
//! What was asked never travels in an argument, which the process list shows.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::Args;

use super::record::{NewSearch, finish_search, join_search, reopen_search, start_search};
use super::request::{
  Asker, DEFAULT_DEPTH, REGIONS, Request, SHAPES, depth_names, is_depth, options, search_key,
};
use crate::clients::job::{Command, Launch, NewRun, RunClose};
use crate::helpers::marks::fill_marks;
use crate::now_ms;
use crate::plumbing::verb::{Answer, Shell, Verb, VerbError};
use crate::settings::ENVIRONMENT_PREFIX;
use crate::verbs::research::admission::{Demand, admit_depth, parse_demand, pool_room, unit_running};
use crate::verbs::research::{Profile, kept_research_on, profile as read_profile, reclaim_leases};

/// The agent a search run belongs to.
pub const AGENT: &str = "search";

/// The variable a launching process's program names its own home in, which says the registration it runs
/// on (`docs/research-runs.md`, "A research run set off on a registration").
pub const HOME_VARIABLE: &str = "CLAUDE_CONFIG_DIR";

/// The variable the agent whose turn set a search off is named in.
pub const TURN_AGENT_VARIABLE: &str = "BMO_TURN_AGENT";

// What a run that never started is closed with: it never reached the work.
const LAUNCH_ERROR: &str = "error";

/// How the verb is called, said with every refusal of what it was asked.
pub fn usage() -> String {
  format!(
    "usage: bmo search --chat <chatId> --query <what to find out> [--message <messageId>] [--depth {}] [--demand explicit|optional] [--period <span>] [--region {}] [--exclude <sources>] [--shape {}] [--into <searchId>] | --import <url> [--query <what was asked>]",
    depth_names(),
    REGIONS.join("|"),
    SHAPES.join("|")
  )
}

/// What the verb was asked, as its words gave it.
#[derive(Args, Debug, Default)]
pub struct Asked {
  #[arg(long, allow_hyphen_values = true)]
  pub chat: Option<String>,
  #[arg(long)]
  pub query: Option<String>,
  /// A page to bring in whole rather than a question to research.
  #[arg(long)]
  pub import: Option<String>,
  #[arg(long)]
  pub depth: Option<String>,
  /// Whether the person insisted on the depth: explicit or optional.
  #[arg(long)]
  pub demand: Option<String>,
  /// The message the search answers, whose sender is who asked.
  #[arg(long, allow_hyphen_values = true)]
  pub message: Option<String>,
  #[arg(long)]
  pub period: Option<String>,
  #[arg(long)]
  pub region: Option<String>,
  #[arg(long)]
  pub exclude: Option<String>,
  #[arg(long)]
  pub shape: Option<String>,
  /// An earlier search of the room this one carries on, writing into its record and document.
  #[arg(long)]
  pub into: Option<String>,
  /// Who set it off: the console names itself, and a chat session's launch names nobody.
  #[arg(long, allow_hyphen_values = true)]
  pub by: Option<String>,
  /// The road the caller was told its turn is read on, which the result is answered on.
  #[arg(long)]
  pub via: Option<String>,
  /// The agent whose turn asked it, which what it comes to goes back to; read from the environment.
  #[arg(skip)]
  pub asked_by: Option<String>,
  /// The home the launching process's program was handed; read from the environment.
  #[arg(skip)]
  pub home: Option<String>,
}

/// A whole number read off the front of a text, as the verbs read one.
pub use crate::helpers::js::parse_int as whole;

/// What the runner is started with: the room, the file holding the question, the depth, the rows it carries
/// and its unit, who set it off, the demand, the settings it launches on, its road and its registration —
/// and never the question itself.
pub struct RunnerCall<'a> {
  pub chat_id: i64,
  pub query_path: &'a Path,
  pub depth: &'a str,
  pub run_id: i64,
  pub unit: &'a str,
  pub started_by: Option<&'a str>,
  pub search_id: i64,
  pub demand: Demand,
  pub profile: &'a Profile,
  pub via: Option<&'a str>,
  pub kept_on: Option<&'a str>,
}

/// The runner's words, after the binary.
pub fn runner_args(call: &RunnerCall<'_>) -> Vec<String> {
  let mut args = vec![
    "search-runner".to_owned(),
    call.chat_id.to_string(),
    call.query_path.to_string_lossy().into_owned(),
    call.depth.to_owned(),
    call.run_id.to_string(),
    call.unit.to_owned(),
    call.started_by.unwrap_or_default().to_owned(),
    call.search_id.to_string(),
    call.demand.word().to_owned(),
    call.profile.snapshot(),
  ];

  if let Some(via) = call.via {
    args.extend(["--via".to_owned(), via.to_owned()]);
  }

  if let Some(registration) = call.kept_on {
    args.extend(["--kept-on".to_owned(), registration.to_owned()]);
  }

  args
}

/// The question written owner-only from the moment the file exists; the runner deletes it once it is read.
pub fn write_request(path: &Path, request: &Request) -> std::io::Result<()> {
  use std::io::Write;
  use std::os::unix::fs::OpenOptionsExt;

  let mut file = std::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .mode(0o600)
    .open(path)?;

  file.write_all(serde_json::to_string(request).unwrap_or_default().as_bytes())
}

/// The process the runner is: this binary on the runner's words, started where this one was, handed the
/// path, the home and every setting this process was given in its environment, so it reads the same settings.
pub fn runner_command(
  args: Vec<String>,
  environment: impl IntoIterator<Item = (String, String)>,
) -> Result<Command, VerbError> {
  let program = std::env::current_exe().map_err(|why| {
    VerbError::Refused(format!("this binary could not be found to run the search: {why}"))
  })?;
  let cwd = std::env::current_dir()
    .map_err(|why| VerbError::Refused(format!("this process's folder could not be read: {why}")))?;
  let handed: BTreeMap<String, String> = environment
    .into_iter()
    .filter(|(name, _)| {
      name == "PATH" || name == "HOME" || name.starts_with(&format!("{ENVIRONMENT_PREFIX}_"))
    })
    .collect();

  Ok(Command {
    program: program.to_string_lossy().into_owned(),
    args,
    cwd: cwd.to_string_lossy().into_owned(),
    environment: handed,
  })
}

// Who sent the message a search answers, when in epoch seconds, and the road it came in on, as bmo-receiver
// keeps it.
async fn message_of(
  shell: &Shell,
  chat_id: i64,
  message_id: i64,
) -> Result<Option<(Option<Asker>, i64, String)>, VerbError> {
  let message = shell
    .receiver
    .message(chat_id, message_id)
    .await
    .map_err(|why| VerbError::Refused(format!("the asking message could not be read: {why}")))?;

  Ok(message.map(|message| {
    (
      message.sender_id.map(|id| Asker {
        id,
        name: message.sender_name.unwrap_or_default(),
      }),
      message.sent_at / 1000,
      message.via,
    )
  }))
}

/// The search launched: the lines said of it, ending with the one a caller reads it by.
pub async fn search(
  shell: &Shell,
  asked: Asked,
  stdin: impl FnOnce() -> String,
) -> Result<Vec<String>, VerbError> {
  let usage = usage();
  let refuse = || VerbError::Refused(usage.clone());
  let chat_id = asked.chat.as_deref().and_then(whole).ok_or_else(refuse)?;
  let source = asked.import.as_deref();
  let query = match (&asked.query, source) {
    (Some(query), _) => query.clone(),
    (None, Some(source)) => fill_marks(&shell.wording.search.import_query, &[("source", source)]),
    (None, None) => stdin(),
  };
  let depth = if source.is_some() {
    DEFAULT_DEPTH.to_owned()
  } else {
    asked.depth.clone().unwrap_or_else(|| DEFAULT_DEPTH.to_owned())
  };
  let demand = parse_demand(asked.demand.as_deref());
  let message_id = match &asked.message {
    None => None,
    Some(text) => Some(whole(text).ok_or_else(refuse)?),
  };
  let into = match &asked.into {
    None => None,
    Some(text) => Some(whole(text).ok_or_else(refuse)?),
  };
  let query = query.trim().to_owned();

  if into.is_some() && source.is_some() {
    return Err(VerbError::Refused(format!(
      "a page imported whole is a document of its own and carries no earlier search on\n{usage}"
    )));
  }

  // A depth not one of the three is refused rather than quietly run at the default: the depth is what
  // somebody asked for, and answering a request to dig deeper with the shallowest search would be a lie.
  let Some(demand) = demand.filter(|_| !query.is_empty() && is_depth(&depth)) else {
    return Err(refuse());
  };
  let options = options(
    asked.period.as_deref(),
    asked.region.as_deref(),
    asked.exclude.as_deref(),
    asked.shape.as_deref(),
    source,
  )
  .map_err(|why| VerbError::Refused(format!("{why}\n{usage}")))?;
  let db = shell.documents().await?;
  let research = &shell.settings.research;
  let agency = &*shell.agency;
  let mut lines = vec![];
  let message = match message_id {
    Some(id) => message_of(shell, chat_id, id).await?,
    None => None,
  };
  // The road the question came in on, which the result is answered on: the one the caller named, or the
  // asking message's own.
  let asked_on = asked.via.clone().or_else(|| message.as_ref().map(|(_, _, via)| via.clone()));
  let asker = message.as_ref().and_then(|(asker, _, _)| asker.clone());

  if let Some(id) = message_id
    && asker.is_none()
  {
    lines.push(format!(
      "message {id} in chat {chat_id} names no sender bmo-receiver knows; the search runs for an asker not known"
    ));
  }

  // Places a research run left in the pool when its process died are given back first, so a dead run never
  // reads as a full pool.
  if let Err(why) = reclaim_leases(agency, &shell.job, unit_running).await {
    eprintln!("the pool's abandoned places could not be reclaimed: {why}");
  }

  // A search set off on a registration that keeps its research runs there at the depth asked with no branch.
  let kept_on = kept_research_on(agency, asked.home.as_deref()).await.map_err(VerbError::Refused)?;
  let asked_profile = read_profile(db, research, &search_key(&depth)).await?.kept_on(kept_on.as_deref());
  let role = asked_profile.child_role().to_owned();
  let admission =
    admit_depth(&depth, DEFAULT_DEPTH, demand, asked_profile.max_children(), || pool_room(agency, &role))
      .await;

  if let Some(fallback) = admission.fallback {
    lines.push(fallback);
  }

  // A follow-up takes its earlier search up again before anything of its own is written, so a search it
  // cannot carry on is refused with nothing left behind.
  let earlier = match into {
    None => None,
    Some(id) => Some(reopen_search(db, id, chat_id).await?.ok_or_else(|| {
      VerbError::Refused(format!("search {id} is no finished search of chat {chat_id} to carry on"))
    })?),
  };
  // How the search is put back where its run never came up: an earlier search as it stood, a new one failed.
  let put_back = earlier.as_deref().unwrap_or("failed");

  let effective = admission.depth;
  // The settings this run launches on, handed to the runner as they are now.
  let profile = read_profile(db, research, &search_key(&effective)).await?.kept_on(kept_on.as_deref());
  let unit = format!("bmo-search-{}", now_ms());
  let query_path = std::env::temp_dir().join(format!("{unit}.txt"));
  let request = Request {
    query: query.clone(),
    asker: asker.clone(),
    asked_at: Some(message.as_ref().map_or_else(|| now_ms() / 1000, |(_, at, _)| *at)),
    options: options.clone(),
  };

  write_request(&query_path, &request)
    .map_err(|why| VerbError::Refused(format!("the question could not be written for the run: {why}")))?;

  let scratch: [PathBuf; 1] = [query_path.clone()];
  let started_by = asked.by.as_deref().filter(|by| !by.is_empty());
  let kind = if source.is_some() {
    Some("import")
  } else if options.shape.as_deref() == Some("learning") {
    Some("learning")
  } else {
    None
  };
  let search_id = match into {
    Some(id) => id,
    None => {
      start_search(
        db,
        &NewSearch {
          chat_id,
          query: &query,
          depth: &effective,
          asker_id: asker.as_ref().map(|asker| asker.id),
          options: &options,
          kind,
        },
      )
      .await?
    }
  };

  join_search(db, search_id, &effective).await?;

  let opened = shell
    .job
    .open_run(&NewRun {
      agent: AGENT.to_owned(),
      chat_id,
      unit: unit.clone(),
      cap_ms: None,
      started_by: started_by.map(str::to_owned),
      asked_by: asked.asked_by.clone(),
      depth: Some(effective.clone()),
      search_id: Some(search_id),
    })
    .await;
  let run_id = match opened {
    Ok(run_id) => run_id,
    Err(why) => {
      take_off(&scratch);
      finish_search(db, search_id, put_back).await?;

      return Err(VerbError::Partly {
        lines,
        refusal: format!("the run's row could not be opened in bmo-job: {why}"),
      });
    }
  };
  let args = runner_args(&RunnerCall {
    chat_id,
    query_path: &query_path,
    depth: &effective,
    run_id,
    unit: &unit,
    started_by,
    search_id,
    demand,
    profile: &profile,
    via: asked_on.as_deref(),
    kept_on: kept_on.as_deref(),
  });
  let launch = Launch {
    title: asked.query.clone().unwrap_or_else(|| query.clone()),
    agent: AGENT.to_owned(),
    command: runner_command(args, std::env::vars())?,
  };

  if let Err(reason) = shell.job.launch(run_id, &launch).await {
    // A run that never came up ends the search it was going to answer, this launch being the only one it has;
    // an earlier search it was to carry on is put back as it stood.
    take_off(&scratch);

    let close = RunClose {
      finished_at: now_ms(),
      outcome: LAUNCH_ERROR.to_owned(),
      context_tokens: None,
    };

    if let Err(why) = shell.job.close_run(run_id, &close).await {
      eprintln!("{unit}: the run's row could not be closed: {why}");
    }

    eprintln!("failed to launch {unit}: bmo-job refused ({reason})");
    finish_search(db, search_id, put_back).await?;

    return Err(VerbError::Partly {
      lines,
      refusal: format!("{unit} did not come up"),
    });
  }

  lines.push(format!(
    "search launched as {unit}; it answers chat {chat_id} by itself when it is done. Do not wait for it and do not answer the question from memory."
  ));

  Ok(lines)
}

// What the launcher wrote for a run that never started, taken off the disk with it.
fn take_off(scratch: &[PathBuf]) {
  for path in scratch {
    if std::fs::remove_file(path).is_err() {
      eprintln!("failed to delete {} after a failed launch", path.display());
    }
  }
}

/// `search`.
pub struct SearchLaunch;

impl Verb for SearchLaunch {
  const NAME: &'static str = "search";
  const ABOUT: &'static str = "Sets a question off as a web search of its own, which answers the room by \
                               itself; the question comes in on standard input where --query names none.";

  type Args = Asked;

  async fn run(&self, mut asked: Asked, shell: &Shell) -> Result<Answer, VerbError> {
    let named = |name: &str| {
      std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    };

    asked.asked_by = named(TURN_AGENT_VARIABLE);
    asked.home = named(HOME_VARIABLE);

    let read = || std::io::read_to_string(std::io::stdin()).unwrap_or_default();

    search(shell, asked, read).await.map(Answer::from)
  }
}

crate::verb!(SearchLaunch);
