//! bmo-receiver (`docs/api.md`, "bmo-receiver"): who sent the message a search answers, when, and the road
//! it came in on; what each room is called, which the archive's list shows and is searched by; the queue a
//! run's ending is handed back to its room's session through, and the owner told, as a quiet instruction; and
//! a morning news page's message said into its room.

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

/// An order queued for the deployment's agent in the owner's private chat: no agent and no room named.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct OwnerOrder {
  agent: Option<String>,
  chat: Option<i64>,
  said: Said,
  sort: String,
}

/// A line said into a room by no agent's turn and no job: who asks it, and the line.
#[derive(Serialize)]
struct Send<'a> {
  asker: Asker,
  tool: &'static str,
  input: SayInput<'a>,
}

#[derive(Serialize)]
struct Asker {
  chat: i64,
  job: bool,
}

#[derive(Serialize)]
struct SayInput<'a> {
  chat: i64,
  text: &'a str,
  reply: bool,
}

#[derive(Deserialize)]
struct Sent {
  #[serde(default)]
  lines: Vec<String>,
  #[serde(default)]
  refusal: Option<String>,
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

  /// An English line for the deployment's agent in the owner's private chat, queued as a quiet instruction of
  /// `sort`: its number.
  pub async fn tell_owner(&self, sort: &str, order: &str) -> Result<i64, String> {
    let order = OwnerOrder {
      agent: None,
      chat: None,
      said: Said {
        order: order.to_owned(),
      },
      sort: sort.to_owned(),
    };
    let queued: Queued = self.unit.post("/instructions", &order).await?;

    Ok(queued.id)
  }

  /// A message said into a room as it stands, by no agent's turn and no job: the lines the receiver said of
  /// it, or the refusal it ended in.
  pub async fn say(&self, chat_id: i64, text: &str) -> Result<Vec<String>, String> {
    let send = Send {
      asker: Asker {
        chat: chat_id,
        job: false,
      },
      tool: "say",
      input: SayInput {
        chat: chat_id,
        text,
        reply: false,
      },
    };
    let sent: Sent = self.unit.post("/sends", &send).await?;

    match sent.refusal {
      Some(refusal) => Err(refusal),
      None => Ok(sent.lines),
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
