//! bmo-receiver (`docs/api.md`, "bmo-receiver"): who sent the message a search answers, when, and the road
//! it came in on; what each room is called, which the archive's list shows and is searched by; and the queue
//! a run's ending is handed back to its room's session through, as a quiet instruction.

use serde::{Deserialize, Serialize};

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

/// A run's ending handed to its room's session as a quiet instruction: the agent whose turn asked, the room,
/// the words the session is handed, the kind of ending it is, the road the question came in on and the run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HandBack {
  /// The conversational agent whose turn set the run going, which what it comes to goes back to.
  pub agent: Option<String>,
  pub chat: i64,
  pub said: Said,
  pub sort: String,
  pub via: Option<String>,
  /// The run in bmo-agency's ledger whose result it is.
  pub run: Option<i64>,
}

/// What an ending hands the session: the words of an order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Said {
  pub order: String,
}

#[derive(Deserialize)]
struct Queued {
  id: i64,
}

#[derive(Deserialize)]
struct Named {
  rooms: Vec<Room>,
}

#[derive(Deserialize)]
struct Room {
  chat_id: i64,
  #[serde(default)]
  title: Option<String>,
}

/// bmo-receiver at the address the settings name, where they name one, carrying the token they name.
#[derive(Clone, Debug)]
pub struct Receiver {
  unit: Unit,
}

impl Receiver {
  pub fn new(address: Option<String>, token: Option<String>) -> Self {
    Self {
      unit: Unit::new("bmo-receiver", "receiver", address).with_token(token),
    }
  }

  /// A run's ending queued for its room's session.
  pub async fn hand_back(&self, back: &HandBack) -> Result<i64, String> {
    let queued: Queued = self.unit.post("/instructions", back).await?;

    Ok(queued.id)
  }

  /// Every room's name, by the room.
  pub async fn room_names(&self) -> Result<std::collections::HashMap<i64, String>, String> {
    let named: Named = self.unit.read("/rooms", &[]).await?;

    Ok(
      named
        .rooms
        .into_iter()
        .filter_map(|room| room.title.map(|title| (room.chat_id, title)))
        .collect(),
    )
  }

  /// One message of a room, none where the receiver holds no such message.
  pub async fn message(&self, chat_id: i64, message_id: i64) -> Result<Option<Message>, String> {
    self.unit.found(&format!("/rooms/{chat_id}/messages/{message_id}")).await
  }
}
