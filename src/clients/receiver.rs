//! bmo-receiver (`docs/api.md`, "bmo-receiver"): who sent the message a search answers, when, and the road
//! it came in on; and what each room is called, which the archive's list shows and is searched by.

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
