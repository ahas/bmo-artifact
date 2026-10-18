//! Stand-ins for the units a search calls besides bmo-agency (`docs/api.md`, "What this repository calls"):
//! the queue an ending is handed back through, bmo-receiver's messages, bmo-gate's knowledge, bmo-voice's
//! briefings and the tool host's page reading, all served on one local port from a book the test fills and
//! reads back. The service's own clients reach it, so a run goes through the same calls the service makes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use bmo_artifact::clients::job::HandBack;
use serde_json::{Value, json};

/// What the stand-ins hold, and every call they were asked to act on.
#[derive(Debug, Default)]
pub struct Book {
  /// Each ending handed back to a room's session.
  pub handed: Vec<HandBack>,
  /// The messages bmo-receiver holds, by room and message.
  pub messages: HashMap<(i64, i64), Value>,
  /// What bmo-receiver says each room is called.
  pub rooms: Vec<(i64, String)>,
  /// The knowledge bmo-gate picks for any piece of work.
  pub pieces: Vec<Value>,
  /// The lines each piece of work's knowledge was asked for.
  pub picked_for: Vec<Vec<String>>,
  /// Each briefing bmo-voice was asked to start.
  pub briefings: Vec<Value>,
  /// What the tool host answers `explorer/import_page` with.
  pub page: Option<Value>,
  /// What the tool host answers `explorer/search_engines` with.
  pub engines: Option<Value>,
}

/// The stand-ins as a test reads and fills them.
#[derive(Clone, Debug, Default)]
pub struct Units(Arc<Mutex<Book>>);

impl Units {
  pub fn book(&self) -> MutexGuard<'_, Book> {
    self.0.lock().unwrap_or_else(PoisonError::into_inner)
  }
}

/// The stand-ins listening on a port of their own, with its address, which every one of them answers on.
pub async fn serve(book: Book) -> (Units, String) {
  let units = Units(Arc::new(Mutex::new(book)));
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port");
  let address = format!("http://{}", listener.local_addr().expect("an address"));
  let app = axum::Router::new()
    .route("/instructions", post(hand_back))
    .route("/rooms", get(rooms))
    .route("/rooms/{room}/messages/{message}", get(message))
    .route("/knowledge/work", post(knowledge))
    .route("/briefings", post(briefing))
    .route("/calls", post(call))
    .with_state(units.clone());

  tokio::spawn(async move { axum::serve(listener, app).await });

  (units, address)
}

/// The settings that point every unit a search calls at the stand-ins, as environment variables.
pub fn pointed_at(address: &str, agency: &str) -> Vec<(&'static str, String)> {
  vec![
    ("BMO_ARTIFACT_UNITS__JOB", address.to_owned()),
    ("BMO_ARTIFACT_UNITS__RECEIVER", address.to_owned()),
    ("BMO_ARTIFACT_UNITS__GATE", address.to_owned()),
    ("BMO_ARTIFACT_UNITS__VOICE", address.to_owned()),
    ("BMO_ARTIFACT_UNITS__TOOL_HOST", address.to_owned()),
    ("BMO_ARTIFACT_UNITS__AGENCY", agency.to_owned()),
  ]
}

fn absent(what: &str) -> Response {
  (
    StatusCode::NOT_FOUND,
    Json(json!({ "error": format!("no such {what}"), "code": "not_found" })),
  )
    .into_response()
}

async fn hand_back(State(units): State<Units>, Json(back): Json<HandBack>) -> Response {
  let mut book = units.book();

  book.handed.push(back);

  (StatusCode::CREATED, Json(json!({ "id": book.handed.len() }))).into_response()
}

async fn rooms(State(units): State<Units>) -> Json<Value> {
  let rooms: Vec<Value> = units
    .book()
    .rooms
    .iter()
    .map(|(chat_id, title)| json!({ "chat_id": chat_id, "title": title }))
    .collect();

  Json(json!({ "rooms": rooms }))
}

async fn message(State(units): State<Units>, Path((room, message)): Path<(i64, i64)>) -> Response {
  match units.book().messages.get(&(room, message)) {
    Some(held) => Json(held.clone()).into_response(),
    None => absent("message"),
  }
}

async fn knowledge(State(units): State<Units>, Json(asked): Json<Value>) -> Json<Value> {
  let mut book = units.book();
  let lines = asked["lines"]
    .as_array()
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
    .map(str::to_owned);

  book.picked_for.push(lines.collect());

  Json(json!({ "pieces": book.pieces }))
}

async fn briefing(State(units): State<Units>, Json(asked): Json<Value>) -> Response {
  let mut book = units.book();

  book.briefings.push(asked);

  (StatusCode::CREATED, Json(json!({ "unit": format!("voice-{}", book.briefings.len()) }))).into_response()
}

async fn call(State(units): State<Units>, Json(asked): Json<Value>) -> Json<Value> {
  let book = units.book();
  let served = match asked["tool"].as_str() {
    Some("explorer/import_page") => book.page.clone(),
    Some("explorer/search_engines") => book.engines.clone(),
    _ => None,
  };

  Json(match served {
    Some(content) => json!({ "content": [], "structuredContent": content }),
    None => json!({ "isError": true, "content": [{ "type": "text", "text": "nothing is served there" }] }),
  })
}
