//! One room's page started as a run of its own (`docs/news-brief.md`, "When it runs"; `docs/api.md`, "The run
//! ledger"): its row opened in bmo-agency's run ledger and `runner news-brief` launched by bmo-agency, which
//! keeps it standing however long it writes. `news-brief --run` and the minute loop start a page the same way.

use std::path::Path;

use crate::clients::agency::{Agency, Launch, NewRun, RunClose};
use crate::plumbing::verb::VerbError;
use crate::store::news_briefs::Brief;
use crate::verbs::research::LINE;
use crate::verbs::search::launch::runner_command;

/// What every page's run is called by, before the moment it started.
pub const UNIT_PREFIX: &str = "bmo-news-brief-";

/// What a page's run is in the words a person watching reads.
pub const TITLE: &str = "morning news page";

// What a run that never came up is closed with: it never reached the work.
const LAUNCH_ERROR: &str = "error";

/// One page's start: the room's row, the day it is for, the time its message waits for where it waits,
/// and the room it goes to instead where it goes elsewhere.
#[derive(Clone, Debug)]
pub struct Start<'a> {
  pub row: &'a Brief,
  pub day: &'a str,
  pub send_at: Option<&'a str>,
  pub send_to: Option<i64>,
}

/// A run launched: its row in the ledger and its unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launched {
  pub run_id: i64,
  pub unit: String,
}

/// The runner's words, after the binary.
pub fn runner_args(start: &Start<'_>, run_id: i64, unit: &str) -> Vec<String> {
  let mut args = vec![
    "news-brief-runner".to_owned(),
    "--chat".to_owned(),
    start.row.chat_id.to_string(),
    "--day".to_owned(),
    start.day.to_owned(),
    "--share-hours".to_owned(),
    start.row.share_hours.to_string(),
  ];

  if let Some(send_at) = start.send_at {
    args.extend(["--send-at".to_owned(), send_at.to_owned()]);
  }

  if let Some(send_to) = start.send_to {
    args.extend(["--send-to".to_owned(), send_to.to_string()]);
  }

  args.extend(["--run".to_owned(), run_id.to_string(), "--unit".to_owned(), unit.to_owned()]);
  args
}

/// This repository's CLI binary, which the runner is: the program given where it is the CLI, else the one
/// standing beside it, since the server ships with it and launches the runner too.
pub fn cli_program(program: &str) -> String {
  let path = Path::new(program);

  if path.file_stem().is_some_and(|stem| stem == CLI) {
    return program.to_owned();
  }

  path.with_file_name(CLI).to_string_lossy().into_owned()
}

const CLI: &str = "bmo-artifact-cli";

/// One page's run opened in bmo-agency's ledger and launched at `now_ms`; refused with the sentence where
/// either did not happen, a run that did not come up closed as one that never started.
pub async fn launch(agency: &dyn Agency, start: &Start<'_>, now_ms: i64) -> Result<Launched, VerbError> {
  let unit = format!("{UNIT_PREFIX}{now_ms}");
  let run_id = agency
    .open_run(&NewRun {
      agent: LINE.to_owned(),
      chat_id: Some(start.row.chat_id),
      unit: unit.clone(),
      depth: 0,
      cap_ms: None,
      started_at: now_ms / 1000,
      started_by: Some(format!("news-brief:{}", start.row.id)),
      search_id: None,
    })
    .await
    .map_err(|why| VerbError::Refused(format!("the run's row could not be opened in bmo-agency: {why}")))?;
  let mut command = runner_command(runner_args(start, run_id, &unit), std::env::vars())?;

  command.program = cli_program(&command.program);

  let launch = Launch {
    title: TITLE.to_owned(),
    agent: LINE.to_owned(),
    command,
  };

  if let Err(reason) = agency.launch(run_id, &launch).await {
    let close = RunClose {
      finished_at: now_ms / 1000,
      outcome: LAUNCH_ERROR.to_owned(),
      context_tokens: None,
    };

    if let Err(why) = agency.close_run(run_id, &close).await {
      eprintln!("{unit}: the run's row could not be closed: {why}");
    }

    return Err(VerbError::Refused(format!("{unit} did not come up: bmo-agency refused ({reason})")));
  }

  Ok(Launched { run_id, unit })
}
