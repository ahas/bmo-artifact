//! One morning news page, written, filed and sent (`docs/news-brief.md`): `bmo runner news-brief`, the run
//! bmo-agency launches.
//!
//! `--chat` is the room the page is made for, and its share link lives `--share-hours`. `--send-to` sends the
//! message somewhere else instead, a trial shown before a room gets it, and the page is made exactly as that
//! room would get it. `--send-at` holds the message until that time where the page is ready early. The
//! writer's answer is kept beside the page; `--answer` draws the page from one already written, and
//! `--no-send` stops once the page file is drawn, filing and sending nothing. `--run` is the ledger row the run
//! closes, and `--unit` the name its log is kept under.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use clap::Args;

use super::brief::{Brief, read_brief};
use super::calendar::{date_of, day_of, moment};
use super::compose::{compose, pacing};
use super::page::{message, page_title};
use crate::clients::agency::Instructions;
use crate::clients::tool_host::ToolHost;
use crate::helpers::lines::{Trees, read_required, render};
use crate::plumbing::verb::{Answer, Shell, Verb, VerbError};
use crate::verbs::archive_docs::{DocumentInput, Documents, ShareLife};
use crate::verbs::research::run::{Research, Road};
use crate::verbs::research::{LINE, Told, profile as read_profile};
use crate::verbs::search::request::asked_at_text;

/// The research key whose profile the writer runs on.
pub const RESEARCH_KEY: &str = "search:basic";

/// The instructions bmo-agency composes for the writer on the search line.
pub const INSTRUCTIONS: &str = "news-brief";

/// The family of the quiet instructions the owner is told by.
pub const SORT: &str = "news_brief";

// How many hours a page's share link lives where the call names none.
const SHARE_HOURS: f64 = 24.0;

/// How the runner is called, said with every refusal of what it was asked.
pub const USAGE: &str = "usage: bmo runner news-brief --chat <chatId> [--send-to <chatId>] [--day YYYY-MM-DD] [--share-hours <n>] [--send-at HH:MM] [--run <runId>] [--unit <unit>] [--answer <file>] [--no-send]";

/// The clock a run reads the time off and waits on; a test hands one of its own.
pub trait Clock: Send + Sync {
  /// Now, in epoch milliseconds.
  fn now_ms(&self) -> i64;
  /// Done once the moment `at_ms` has come.
  fn wait_until(&self, at_ms: i64) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

/// The machine's own clock.
pub struct Wall;

impl Clock for Wall {
  fn now_ms(&self) -> i64 {
    crate::now_ms()
  }

  fn wait_until(&self, at_ms: i64) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
    let wait = (at_ms - crate::now_ms()).max(0).unsigned_abs();

    Box::pin(tokio::time::sleep(std::time::Duration::from_millis(wait)))
  }
}

/// One page's run, as the binary takes it.
#[derive(Args, Clone, Debug, Default)]
#[command(allow_negative_numbers = true)]
pub struct RunnerArgs {
  /// The room the page is made for.
  #[arg(long)]
  pub chat: Option<String>,
  /// The room the message goes to instead.
  #[arg(long)]
  pub send_to: Option<String>,
  /// The day the page is for, today's where none is named.
  #[arg(long)]
  pub day: Option<String>,
  /// How many hours the page's share link lives.
  #[arg(long)]
  pub share_hours: Option<String>,
  /// The time the message is held until, HH:MM.
  #[arg(long)]
  pub send_at: Option<String>,
  /// The row in bmo-agency's run ledger the run closes.
  #[arg(long)]
  pub run: Option<String>,
  /// The name the run's log is kept under.
  #[arg(long)]
  pub unit: Option<String>,
  /// A writer's answer already written, which the page is drawn from instead.
  #[arg(long)]
  pub answer: Option<PathBuf>,
  /// Stops once the page file is drawn.
  #[arg(long)]
  pub no_send: bool,
}

/// What a run was asked, read and checked.
#[derive(Clone, Debug, PartialEq)]
pub struct Asked {
  pub chat_id: i64,
  pub send_to: i64,
  pub day: String,
  pub share_hours: f64,
  /// The time the message waits for, as it was written and as the moment it is.
  pub send_at: Option<(String, i64)>,
  pub run_id: Option<i64>,
  pub unit: String,
  pub answer: Option<PathBuf>,
  pub no_send: bool,
}

impl RunnerArgs {
  /// The run these words ask for at `now_ms`, its day and its time read in `zone`; none where a word does
  /// not read.
  pub fn read(&self, now_ms: i64, zone: chrono_tz::Tz) -> Option<Asked> {
    let whole = |text: &str| text.trim().parse::<i64>().ok();
    let chat_id = whole(self.chat.as_deref()?)?;
    let send_to = match &self.send_to {
      Some(named) => whole(named)?,
      None => chat_id,
    };
    let day = self.day.clone().unwrap_or_else(|| day_of(now_ms, zone));
    let share_hours = match &self.share_hours {
      Some(named) => named.trim().parse::<f64>().ok().filter(|hours| *hours > 0.0)?,
      None => SHARE_HOURS,
    };

    date_of(&day)?;

    let send_at = match &self.send_at {
      Some(clock) => Some((clock.clone(), moment(&day, clock, zone)?)),
      None => None,
    };
    let run_id = match &self.run {
      Some(named) => Some(whole(named)?),
      None => None,
    };

    Some(Asked {
      chat_id,
      send_to,
      day,
      share_hours,
      send_at,
      run_id,
      unit: self.unit.clone().unwrap_or_default(),
      answer: self.answer.clone(),
      no_send: self.no_send,
    })
  }
}

fn say(line: &str) {
  eprintln!("news-brief: {line}");
}

/// The folder each day's page and its writer's answer are kept in: `news-brief/` in the file root.
pub fn folder(shell: &Shell) -> PathBuf {
  shell.settings.files.root.join("news-brief")
}

// One run as it works: what it stands on, what it was asked, the research run its row is closed by.
struct Run<'a> {
  shell: &'a Shell,
  asked: &'a Asked,
  clock: &'a dyn Clock,
  research: Research,
}

impl Run<'_> {
  // The writer: a run apart on the search line under the news task bmo-agency composes, handed the day. It
  // opens no branch.
  async fn write(&mut self) -> Result<String, String> {
    let shell = self.shell;
    let code = shell
      .settings
      .prompts
      .code
      .clone()
      .ok_or("the prompt tree is not set: set code under [prompts] in the settings")?;
    let trees = Trees { code };
    let template =
      read_required(&trees.agent_dir(LINE).join("news-brief-request.md")).map_err(|why| why.detail)?;
    let asked_at = asked_at_text(self.clock.now_ms() / 1000, shell.settings.time.zone);
    let prompt = render(&template, &[("day", &self.asked.day), ("askedAt", &asked_at)]);
    let db = shell.documents().await.map_err(|why| why.to_string())?;
    let profile = read_profile(db, &shell.settings.research, RESEARCH_KEY)
      .await
      .map_err(|why| why.detail)?;
    let instructions = Instructions {
      name: INSTRUCTIONS.to_owned(),
      text: None,
    };

    say(&format!("writing the page for {}", self.asked.day));
    self.research.count_run().await;

    let reply = self
      .research
      .ask(Told::default(), &Road::Key(RESEARCH_KEY.to_owned()), &profile, instructions, &prompt)
      .await?;

    Ok(reply.text)
  }

  // The page filed as a page document of the room it is for, with a share link living the run's hours: the
  // link.
  async fn file(&self, page_path: &Path) -> Result<String, String> {
    let shell = self.shell;
    let console = shell
      .settings
      .console
      .address
      .clone()
      .ok_or("the console's address is not set: set address under [console] in the settings")?;
    let db = shell.documents().await.map_err(|why| why.to_string())?.clone();
    let documents = Documents::of_shell(shell, db, console);
    let written = documents
      .write(&DocumentInput {
        chat_id: self.asked.chat_id,
        asker_id: None,
        title: String::new(),
        kind: "page".to_owned(),
        body: String::new(),
        attach: vec![],
        html: Some(page_path.to_string_lossy().into_owned()),
        replace: None,
      })
      .await?;
    let seconds = (self.asked.share_hours * 3600.0).round() as i64;
    let handover = documents
      .hand_over(written.id, false, ShareLife::Seconds(seconds))
      .await
      .map_err(|why| why.detail)?;
    let Some((link, _)) = handover.share else {
      return Err(format!("document {} was filed but no share link could be made", written.id));
    };

    say(&format!("filed as document {}: {}", written.id, handover.console));

    Ok(link)
  }

  // The day's page made, and filed and sent unless it was asked to stop once drawn: the lines said of it.
  async fn make(&mut self) -> Result<Vec<String>, String> {
    let shell = self.shell;
    let asked = self.asked;
    let (day, chat_id) = (asked.day.as_str(), asked.chat_id);
    let folder = folder(shell);
    let page_path = folder.join(format!("news-{day}-{chat_id}.html"));
    let answer_path = folder.join(format!("answer-{day}-{chat_id}.txt"));

    std::fs::create_dir_all(&folder).map_err(|why| format!("{}: {why}", folder.display()))?;

    let answer = match &asked.answer {
      Some(file) => {
        std::fs::read_to_string(file).map_err(|why| format!("the writer did not answer: {why}"))?
      }
      None => {
        let written = self.write().await.map_err(|why| format!("the writer did not answer: {why}"))?;

        std::fs::write(&answer_path, &written)
          .map_err(|why| format!("{}: {why}", answer_path.display()))?;
        written
      }
    };
    let kept = asked.answer.as_ref().unwrap_or(&answer_path).display().to_string();
    let brief: Brief = read_brief(&answer).map_err(|why| format!("{why}; the answer is kept in {kept}"))?;
    let words = &shell.wording.news_page;
    let host = ToolHost::new(shell.settings.units.tool_host.clone(), pacing(day, chat_id));
    let composed = compose(words, day, &brief, &host).await;

    for line in &composed.said {
      say(line);
    }

    std::fs::write(&page_path, &composed.html).map_err(|why| format!("{}: {why}", page_path.display()))?;

    if asked.no_send {
      return Ok(vec![format!("the page is drawn in {}; nothing was filed or sent", page_path.display())]);
    }

    let link = self
      .file(&page_path)
      .await
      .map_err(|why| format!("the page could not be filed: {why}"))?;

    // A page ready before its time waits for it, so the room gets it when it was promised.
    if let Some((clock, at)) = &asked.send_at
      && *at > self.clock.now_ms()
    {
      say(&format!("holding the message until {clock}"));
      self.clock.wait_until(*at).await;
    }

    shell
      .receiver
      .say(asked.send_to, &message(words, day, &brief, &link))
      .await
      .map_err(|why| format!("the room {} was not sent the message: {why}", asked.send_to))?;

    Ok(vec![format!("{} sent to {}: {link}", page_title(words, day), asked.send_to)])
  }

  // A run that failed tells the owner which room went without its page and why, and ends with failure.
  async fn fail(&mut self, reason: &str) -> VerbError {
    let asked = self.asked;
    let order = format!(
      "The morning news page for {} could not be made, so room {} was sent nothing today. Reason: {reason}. \
       The run's log: {}.",
      asked.day,
      asked.chat_id,
      if asked.unit.is_empty() { "none was kept" } else { &asked.unit }
    );

    self.research.close("failed").await;

    if let Err(why) = self.shell.receiver.tell_owner(SORT, &order).await {
      say(&format!("the owner could not be told: {why}"));
    }

    VerbError::Ended {
      code: 1,
      said: format!("news-brief: FAILED — {reason}"),
    }
  }
}

/// One page made, filed and sent on what the shell stands on, at the time `clock` tells: the lines said of
/// it, or the failure, which the owner has been told of.
pub async fn make(shell: &Shell, asked: &Asked, clock: &dyn Clock) -> Result<Vec<String>, VerbError> {
  let mut run = Run {
    shell,
    asked,
    clock,
    research: Research::new(Arc::clone(&shell.agency), LINE, asked.chat_id, asked.run_id, &asked.unit),
  };

  match run.make().await {
    Ok(lines) => {
      run.research.close("done").await;
      Ok(lines)
    }
    Err(reason) => Err(run.fail(&reason).await),
  }
}

/// `runner news-brief`.
pub struct NewsBriefRunner;

impl Verb for NewsBriefRunner {
  const NAME: &'static str = "runner news-brief";
  const ALIASES: &'static [&'static str] = &["news-brief-runner"];
  const ABOUT: &'static str = "One morning news page written, filed as a page document and sent to its room \
                               as a link.";

  type Args = RunnerArgs;

  async fn run(&self, args: RunnerArgs, shell: &Shell) -> Result<Answer, VerbError> {
    let asked = args.read(Wall.now_ms(), shell.settings.time.zone).ok_or_else(|| VerbError::Ended {
      code: 2,
      said: USAGE.to_owned(),
    })?;

    make(shell, &asked, &Wall).await.map(Answer::from)
  }
}

crate::verb!(NewsBriefRunner);
