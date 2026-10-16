//! bmo-receiver (`docs/api.md`, "bmo-receiver"): who sent the message a search answers, when, and the road
//! it came in on.

use serde::Deserialize;

use super::json::Unit;

/// The message a search answers, as the receiver keeps it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Message {
  #[serde(default)]
  pub sender_id: Option<i64>,
  #[serde(default)]
  pub sender_name: Option<String>,
  /// When it was sent, in epoch milliseconds.
  pub sent_at: i64,
  /// The road it came in on.
  pub via: String,
}

/// bmo-receiver at the address the settings name, where they name one.
#[derive(Clone, Debug)]
pub struct Receiver {
  unit: Unit,
}

impl Receiver {
  pub fn new(address: Option<String>) -> Self {
    Self {
      unit: Unit::new("bmo-receiver", "receiver", address),
    }
  }

  /// One message of a room, none where the receiver holds no such message.
  pub async fn message(&self, chat_id: i64, message_id: i64) -> Result<Option<Message>, String> {
    self.unit.found(&format!("/rooms/{chat_id}/messages/{message_id}")).await
  }
}
