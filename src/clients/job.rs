//! bmo-job (`docs/api.md`, "bmo-job"): the ledger of runs, where a search run's row is opened, its runner
//! launched, read and closed; and the queue a run's ending is handed back to its room's session through, as a
//! quiet instruction.

use std::collections::BTreeMap;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use super::json::Unit;

/// A run's row as it is opened: whose run it is, the room it reports to, the unit it runs as, how long it
/// may take, who set it off, the agent whose turn asked it, its depth and the search it carries.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewRun {
  pub agent: String,
  pub chat_id: i64,
  pub unit: String,
  /// How long it may take, or none for a run that follows its work to the end however long it takes.
  pub cap_ms: Option<i64>,
  pub started_by: Option<String>,
  /// The conversational agent whose turn set it going, which what it comes to goes back to.
  pub asked_by: Option<String>,
  pub depth: Option<String>,
  pub search_id: Option<i64>,
}

/// One run of the ledger.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
  pub id: i64,
  pub agent: String,
  #[serde(default)]
  pub chat_id: Option<i64>,
  pub unit: String,
  #[serde(default)]
  pub depth: Option<String>,
  #[serde(default)]
  pub search_id: Option<i64>,
  pub started_at: i64,
  #[serde(default)]
  pub finished_at: Option<i64>,
  #[serde(default)]
  pub outcome: Option<String>,
  #[serde(default)]
  pub context_tokens: Option<i64>,
}

/// How a run ended.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunClose {
  pub finished_at: i64,
  pub outcome: String,
  pub context_tokens: Option<i64>,
}

/// The process a run is: the program, its words, the folder it starts in and the variables it is handed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
  pub program: String,
  pub args: Vec<String>,
  pub cwd: String,
  pub environment: BTreeMap<String, String>,
}

/// A run's runner launched: what it is in the words a person watching reads, the agent it belongs to, and
/// its process.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Launch {
  pub title: String,
  pub agent: String,
  pub command: Command,
}

/// A run's ending handed to its room's session as a quiet instruction: the run, the room, the text, the
/// kind of ending it is and the road the question came in on.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandBack {
  pub run_id: Option<i64>,
  pub chat_id: i64,
  pub text: String,
  pub sort: String,
  pub via: Option<String>,
}

#[derive(Deserialize)]
struct Opened {
  id: i64,
}

#[derive(Deserialize)]
struct Carried {
  runs: Vec<i64>,
}

/// bmo-job at the address the settings name, where they name one.
#[derive(Clone, Debug)]
pub struct Job {
  unit: Unit,
}

impl Job {
  pub fn new(address: Option<String>) -> Self {
    Self {
      unit: Unit::new("bmo-job", "job", address),
    }
  }

  /// A run's row opened before the run is, so it is in the ledger from the moment it exists: its number.
  pub async fn open_run(&self, run: &NewRun) -> Result<i64, String> {
    let opened: Opened = self.unit.post("/runs", run).await?;

    Ok(opened.id)
  }

  /// A run's runner launched; bmo-job's sentence where it did not come up.
  pub async fn launch(&self, run_id: i64, launch: &Launch) -> Result<(), String> {
    self.unit.act(&format!("/runs/{run_id}/launch"), launch, None).await.map(|_| ())
  }

  /// One run, none where the ledger holds no such run.
  pub async fn run(&self, run_id: i64) -> Result<Option<Run>, String> {
    self.unit.found(&format!("/runs/{run_id}")).await
  }

  /// A run's row closed with how it ended: whether this was the ending that closed it, `false` where it was
  /// closed already.
  pub async fn close_run(&self, run_id: i64, close: &RunClose) -> Result<bool, String> {
    self.unit.act(&format!("/runs/{run_id}/close"), close, Some(StatusCode::CONFLICT)).await
  }

  /// Every run that has carried a search, in the order they were opened.
  pub async fn runs_of_search(&self, search_id: i64) -> Result<Vec<i64>, String> {
    let carried: Carried = self.unit.read("/runs", &[("search", search_id.to_string())]).await?;

    Ok(carried.runs)
  }

  /// A run's ending queued for its room's session.
  pub async fn hand_back(&self, back: &HandBack) -> Result<i64, String> {
    let queued: Opened = self.unit.post("/instructions", back).await?;

    Ok(queued.id)
  }
}
