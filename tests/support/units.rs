//! Stand-ins for the units a search calls besides bmo-agency (`docs/api.md`, "What this repository calls"):
//! bmo-job's ledger and queue, bmo-receiver's messages, bmo-gate's knowledge, bmo-voice's briefings and the
//! tool host's page reading, all served on one local port from a book the test fills and reads back. The
//! service's own clients reach it, so a launch or a run goes through the same calls the service makes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use bmo_artifact::clients::job::{HandBack, Launch, NewRun, Run, RunClose};
use serde_json::{Value, json};

/// What the stand-ins hold, and every call they were asked to act on.
#[derive(Debug, Default)]
pub struct Book {
  /// The ledger: each run opened, as bmo-job holds it.
  pub runs: Vec<Run>,
  /// Each run opened, as it was asked for.
  pub opened: Vec<NewRun>,
  /// Each launch asked for, by its run.
  pub launches: Vec<(i64, Launch)>,
  /// The sentence a launch is refused with; none lets every launch come up.
  pub launch_refusal: Option<String>,
  pub closes: Vec<(i64, RunClose)>,
  /// Each ending handed back to a room's session.
  pub handed: Vec<HandBack>,
  /// The messages bmo-receiver holds, by room and message.
  pub messages: HashMap<(i64, i64), Value>,
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
    .route("/runs", post(open_run).get(runs_of_search))
    .route("/runs/{id}", get(run))
    .route("/runs/{id}/launch", post(launch))
    .route("/runs/{id}/close", post(close_run))
    .route("/instructions", post(hand_back))
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

async fn open_run(State(units): State<Units>, Json(asked): Json<NewRun>) -> Response {
  let mut book = units.book();
  let id = 100 + book.runs.len() as i64;

  book.runs.push(Run {
    id,
    agent: asked.agent.clone(),
    chat_id: Some(asked.chat_id),
    unit: asked.unit.clone(),
    depth: asked.depth.clone(),
    search_id: asked.search_id,
    started_at: 1,
    ..Run::default()
  });
  book.opened.push(asked);

  (StatusCode::CREATED, Json(json!({ "id": id }))).into_response()
}

async fn runs_of_search(
  State(units): State<Units>,
  Query(query): Query<HashMap<String, String>>,
) -> Json<Value> {
  let search: Option<i64> = query.get("search").and_then(|id| id.parse().ok());
  let runs: Vec<i64> = units
    .book()
    .runs
    .iter()
    .filter(|run| run.search_id == search)
    .map(|run| run.id)
    .collect();

  Json(json!({ "runs": runs }))
}

async fn run(State(units): State<Units>, Path(id): Path<i64>) -> Response {
  match units.book().runs.iter().find(|run| run.id == id) {
    Some(run) => Json(run.clone()).into_response(),
    None => absent("run"),
  }
}

async fn launch(State(units): State<Units>, Path(id): Path<i64>, Json(launch): Json<Launch>) -> Response {
  let mut book = units.book();

  book.launches.push((id, launch));

  match book.launch_refusal.clone() {
    None => StatusCode::NO_CONTENT.into_response(),
    Some(refused) => {
      (StatusCode::BAD_GATEWAY, Json(json!({ "error": refused, "code": "launch_failed" }))).into_response()
    }
  }
}

async fn close_run(
  State(units): State<Units>,
  Path(id): Path<i64>,
  Json(close): Json<RunClose>,
) -> Response {
  let mut book = units.book();
  let Some(run) = book.runs.iter_mut().find(|run| run.id == id) else {
    return absent("run");
  };

  if run.finished_at.is_some() {
    return (
      StatusCode::CONFLICT,
      Json(json!({ "error": "the run is already closed", "code": "run_closed" })),
    )
      .into_response();
  }

  run.finished_at = Some(close.finished_at);
  run.outcome = Some(close.outcome.clone());
  run.context_tokens = close.context_tokens;
  book.closes.push((id, close));

  StatusCode::NO_CONTENT.into_response()
}

async fn hand_back(State(units): State<Units>, Json(back): Json<HandBack>) -> Response {
  let mut book = units.book();

  book.handed.push(back);

  (StatusCode::CREATED, Json(json!({ "id": book.handed.len() }))).into_response()
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
