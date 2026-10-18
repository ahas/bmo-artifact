//! bmo-job (`docs/api.md`, "bmo-job"): the queue a run's ending is handed back to its room's session
//! through, as a quiet instruction.

use serde::{Deserialize, Serialize};

use super::json::Unit;

/// A run's ending handed to its room's session as a quiet instruction: the run, the room, the agent whose
/// turn asked, the text, the kind of ending it is and the road the question came in on.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandBack {
  pub run_id: Option<i64>,
  pub chat_id: i64,
  /// The conversational agent whose turn set the run going, which what it comes to goes back to.
  #[serde(default)]
  pub agent: Option<String>,
  pub text: String,
  pub sort: String,
  pub via: Option<String>,
}

#[derive(Deserialize)]
struct Queued {
  id: i64,
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

  /// A run's ending queued for its room's session.
  pub async fn hand_back(&self, back: &HandBack) -> Result<i64, String> {
    let queued: Queued = self.unit.post("/instructions", back).await?;

    Ok(queued.id)
  }
}
