//! bmo-voice: a finished document's narration started as a briefing (`docs/api.md`, "Start a briefing of
//! a finished document").

use serde::{Deserialize, Serialize};

/// A briefing to start: the room it is spoken to, the script, what started it and the document whose
/// sound it is kept as.
#[derive(Clone, Debug, Serialize)]
pub struct Briefing {
  pub chat_id: i64,
  pub text: String,
  pub started_by: String,
  pub document: Option<i64>,
}

#[derive(Deserialize)]
struct Started {
  unit: String,
}

#[derive(Deserialize)]
struct Refused {
  error: String,
}

/// bmo-voice at the address the settings name, where they name one.
#[derive(Clone, Debug)]
pub struct Voice {
  http: reqwest::Client,
  address: Option<String>,
}

impl Voice {
  pub fn new(address: Option<String>) -> Self {
    Self {
      http: reqwest::Client::new(),
      address: address.map(|address| address.trim_end_matches('/').to_owned()),
    }
  }

  /// A briefing started: the run that speaks it, or bmo-voice's own sentence refusing it; the failure's
  /// own words where no answer came.
  pub async fn brief(&self, briefing: &Briefing) -> Result<Result<String, String>, String> {
    let Some(address) = &self.address else {
      return Err("the bmo-voice address is not set: set voice under [units] in the settings".to_owned());
    };
    let answer = self
      .http
      .post(format!("{address}/briefings"))
      .json(briefing)
      .send()
      .await
      .map_err(|why| why.to_string())?;
    let status = answer.status();
    let body = answer.bytes().await.map_err(|why| why.to_string())?;

    if status.is_success() {
      return serde_json::from_slice::<Started>(&body)
        .map(|started| Ok(started.unit))
        .map_err(|why| format!("bmo-voice answered {status} with no unit: {why}"));
    }

    Ok(Err(
      serde_json::from_slice::<Refused>(&body)
        .map(|refused| refused.error)
        .unwrap_or_else(|_| format!("bmo-voice answered {status}")),
    ))
  }
}
