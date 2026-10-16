//! A stand-in bmo-agency (`docs/api.md`, "bmo-agency"): every path the client calls, served on a local
//! port from a book the test fills and reads back. The service's own `AgencyClient` reaches it, so a test
//! of a run, a branch or a settings route goes through the same calls the service makes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use axum::Json;
use axum::extract::{Path, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use bmo_artifact::clients::agency::{
  Agency, AgencyClient, Apart, ApartAsking, ApartReply, Asking, Assignment, Mapping, Note, Place,
  PlaceWrite, PoolRequest, Provider, Registration, Reservation, ReservationAsking, Role, RouteAsked, Seat,
  Tier,
};
use serde_json::{Value, json};

/// What the stand-in holds, and every call it was asked to act on.
#[derive(Debug, Default)]
pub struct Book {
  pub providers: Vec<Provider>,
  pub tiers: Vec<Tier>,
  pub assignments: Vec<Assignment>,
  /// The routes, found by provider and tier.
  pub mappings: Vec<Mapping>,
  /// The efforts every model refuses.
  pub refused_efforts: Vec<String>,
  pub roles: Vec<Role>,
  pub status: Vec<Place>,
  pub requests: Vec<PoolRequest>,
  /// Whether a waiting request is given a place when it tries for one.
  pub leasing: bool,
  pub reservations: Vec<Reservation>,
  /// The state every seat of a new reservation stands in.
  pub seat_state: String,
  pub seats: HashMap<String, Vec<Seat>>,
  pub pings: HashMap<String, Vec<bmo_artifact::clients::agency::Ping>>,
  pub promoted: usize,
  pub writes: Vec<(String, PlaceWrite)>,
  pub limits: Vec<(String, String, i64)>,
  pub limits_reset: usize,
  pub counted: Vec<String>,
  pub notes: Vec<Note>,
  /// Each home and the registration that owns it.
  pub registrations: Vec<(String, Registration)>,
  pub started: Vec<ApartAsking>,
  /// How every run apart stands when it is read; a stop leaves it stopped.
  pub apart: Apart,
  pub stopped: Vec<String>,
  /// Each request finished: its token, its outcome and whether it was cancelled.
  pub finished: Vec<(String, String, bool)>,
  /// Each reservation given back: its token and its reason.
  pub released: Vec<(String, String)>,
  /// Every call, as its method and its address past the host.
  pub calls: Vec<String>,
}

impl Book {
  /// One provider lending co-workers and carrying both research lines, a co-worker tier and a run tier
  /// with a route each, a pooled role, a role that is not, the control role, two free places, and every
  /// run apart answering at once.
  pub fn lending() -> Self {
    let provider = |name: &str, enabled: bool| Provider {
      name: name.to_owned(),
      label: name.to_uppercase(),
      enabled,
      registered: true,
      lines: vec!["search".to_owned(), "browse".to_owned()],
      coworkers: true,
    };
    let assigned = |key: &str, tier: &str| Assignment {
      key: key.to_owned(),
      tier: tier.to_owned(),
    };
    let route = |tier: &str, model: &str, effort: &str| Mapping {
      provider: "prov-a".to_owned(),
      tier: tier.to_owned(),
      model: model.to_owned(),
      effort: Some(effort.to_owned()),
      answer_tokens: None,
    };
    let role = |name: &str, pooled: bool, control: bool| Role {
      name: name.to_owned(),
      description: format!("the {name} role"),
      enabled: true,
      pooled,
      control,
    };

    Self {
      providers: vec![provider("prov-a", true), provider("prov-b", false)],
      tiers: vec![
        Tier {
          name: "small".to_owned(),
          coworker: true,
          default_capacity: Some(4),
        },
        Tier {
          name: "large".to_owned(),
          coworker: false,
          default_capacity: None,
        },
      ],
      assignments: vec![
        assigned("role:scout", "small"),
        assigned("role:helper", "small"),
        assigned("search:basic", "large"),
        assigned("search:thorough", "large"),
        assigned("search:exhaustive", "large"),
        assigned("browse", "large"),
      ],
      mappings: vec![route("small", "model-a", "low"), route("large", "model-b", "high")],
      roles: vec![role("scout", true, false), role("helper", false, false), role("keeper", true, true)],
      status: vec![Place {
        provider: "prov-a".to_owned(),
        tier: "small".to_owned(),
        capacity: 2,
        default_capacity: 4,
        ..Place::default()
      }],
      leasing: true,
      seat_state: "reserved".to_owned(),
      apart: answered("the answer"),
      ..Self::default()
    }
  }
}

/// A run apart that answered `text`.
pub fn answered(text: &str) -> Apart {
  Apart {
    state: "answered".to_owned(),
    streamed: vec![said(text)],
    reply: Some(ApartReply {
      text: text.to_owned(),
      model: Some("model-b".to_owned()),
      effort: Some("high".to_owned()),
      context_tokens: Some(1200),
      live_tasks: Vec::new(),
    }),
    ..Apart::default()
  }
}

/// A record line in which the model wrote `text`.
pub fn said(text: &str) -> String {
  json!({ "type": "assistant", "message": { "content": [{ "type": "text", "text": text }] } }).to_string()
}

/// The stand-in as a test reads and fills it.
#[derive(Clone, Debug, Default)]
pub struct Stand(Arc<Mutex<Book>>);

impl Stand {
  pub fn book(&self) -> MutexGuard<'_, Book> {
    self.0.lock().unwrap_or_else(PoisonError::into_inner)
  }
}

/// The stand-in listening on a port of its own, with its address, written with a trailing slash as a
/// setting may be.
pub async fn serve(book: Book) -> (Stand, String) {
  let stand = Stand(Arc::new(Mutex::new(book)));
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port");
  let address = format!("http://{}/", listener.local_addr().expect("an address"));
  let app = axum::Router::new()
    .route("/providers", get(providers))
    .route("/tiers", get(tiers))
    .route("/tiers/assignments", get(assignments))
    .route("/routes/resolve", post(resolve))
    .route("/routes/check-effort", post(check_effort))
    .route("/pool/roles", get(roles))
    .route("/pool/status", get(status))
    .route("/pool/requests", post(ask_place).get(requests))
    .route("/pool/requests/{token}", get(request))
    .route("/pool/requests/{token}/lease", post(lease))
    .route("/pool/requests/{token}/bind", post(bind))
    .route("/pool/requests/{token}/finish", post(finish))
    .route("/pool/reservations", post(reserve).get(reservations))
    .route("/pool/reservations/promote", post(promote))
    .route("/pool/reservations/{token}/seats", get(seats))
    .route("/pool/reservations/{token}/pings", get(pings))
    .route("/pool/reservations/{token}/release", post(release))
    .route("/pool/places/{place}/write", put(write_place))
    .route("/pool/limits/{provider}/{tier}", put(set_limit))
    .route("/pool/limits", delete(reset_limits))
    .route("/agents/{agent}/runs", post(count_run))
    .route("/log/notes", post(note))
    .route("/registrations/by-home", get(registration))
    .route("/apart", post(start_apart))
    .route("/apart/{id}", get(apart))
    .route("/apart/{id}/stop", post(stop_apart))
    .layer(middleware::from_fn_with_state(stand.clone(), noted))
    .with_state(stand.clone());

  tokio::spawn(async move { axum::serve(listener, app).await });

  (stand, address)
}

/// The service's own client of the stand-in at `address`.
pub fn client(address: &str) -> Arc<dyn Agency> {
  Arc::new(AgencyClient::new(Some(address.to_owned())))
}

/// A stand-in filled with `book`, and the client that reaches it.
pub async fn standing_in(book: Book) -> (Stand, Arc<dyn Agency>) {
  let (stand, address) = serve(book).await;

  (stand, client(&address))
}

async fn noted(State(stand): State<Stand>, request: Request, next: Next) -> Response {
  stand.book().calls.push(format!("{} {}", request.method(), request.uri()));

  next.run(request).await
}

fn refuse(status: StatusCode, code: &str, error: &str) -> Response {
  (status, Json(json!({ "error": error, "code": code }))).into_response()
}

fn absent(what: &str) -> Response {
  refuse(StatusCode::NOT_FOUND, "not_found", &format!("no {what} is held under that name"))
}

fn done() -> Response {
  StatusCode::NO_CONTENT.into_response()
}

type Owner = Query<HashMap<String, String>>;

fn of_owner(query: &Owner, owner: &str) -> bool {
  query.get("owner").is_none_or(|asked| asked == owner)
}

async fn providers(State(stand): State<Stand>) -> Json<Vec<Provider>> {
  Json(stand.book().providers.clone())
}

async fn tiers(State(stand): State<Stand>) -> Json<Vec<Tier>> {
  Json(stand.book().tiers.clone())
}

async fn assignments(State(stand): State<Stand>) -> Json<Vec<Assignment>> {
  Json(stand.book().assignments.clone())
}

async fn resolve(State(stand): State<Stand>, Json(asked): Json<RouteAsked>) -> Response {
  let book = stand.book();
  let found = book.mappings.iter().find(|one| {
    asked.provider.as_ref().is_none_or(|provider| *provider == one.provider) && one.tier == asked.tier
  });

  match found {
    Some(mapping) => Json(mapping.clone()).into_response(),
    None => absent("route"),
  }
}

async fn check_effort(State(stand): State<Stand>, Json(asked): Json<Value>) -> Response {
  let effort = asked["effort"].as_str().unwrap_or_default();

  if stand.book().refused_efforts.iter().any(|one| one == effort) {
    let model = asked["model"].as_str().unwrap_or_default();

    return refuse(
      StatusCode::UNPROCESSABLE_ENTITY,
      "effort_refused",
      &format!("{model} does not take the {effort} effort"),
    );
  }

  done()
}

async fn roles(State(stand): State<Stand>) -> Json<Vec<Role>> {
  Json(stand.book().roles.clone())
}

async fn status(State(stand): State<Stand>) -> Json<Vec<Place>> {
  Json(stand.book().status.clone())
}

async fn ask_place(State(stand): State<Stand>, Json(asking): Json<Asking>) -> Json<PoolRequest> {
  let mut book = stand.book();
  let request = PoolRequest {
    asking,
    token: format!("req-{}", book.requests.len() + 1),
    state: "waiting".to_owned(),
    enqueued_at: 1,
    ..PoolRequest::default()
  };

  book.requests.push(request.clone());

  Json(request)
}

async fn request(State(stand): State<Stand>, Path(token): Path<String>) -> Response {
  match stand.book().requests.iter().find(|one| one.token == token) {
    Some(request) => Json(request.clone()).into_response(),
    None => absent("request"),
  }
}

async fn requests(State(stand): State<Stand>, query: Owner) -> Json<Vec<PoolRequest>> {
  Json(
    stand
      .book()
      .requests
      .iter()
      .filter(|one| one.state != "ended" && of_owner(&query, &one.asking.owner))
      .cloned()
      .collect(),
  )
}

// A request of the book, changed by `change`, as it then stands.
fn changed(stand: &Stand, token: &str, change: impl FnOnce(&mut PoolRequest, bool)) -> Response {
  let mut book = stand.book();
  let leasing = book.leasing;

  match book.requests.iter_mut().find(|one| one.token == token) {
    Some(request) => {
      change(request, leasing);
      Json(request.clone()).into_response()
    }
    None => absent("request"),
  }
}

async fn lease(State(stand): State<Stand>, Path(token): Path<String>) -> Response {
  changed(&stand, &token, |request, leasing| {
    if leasing && request.state == "waiting" {
      request.state = "active".to_owned();
      request.leased_at = Some(2);
    }
  })
}

async fn bind(State(stand): State<Stand>, Path(token): Path<String>, Json(body): Json<Value>) -> Response {
  changed(&stand, &token, |request, _| {
    request.child = body["child"].as_str().map(str::to_owned);
  })
}

async fn finish(
  State(stand): State<Stand>,
  Path(token): Path<String>,
  Json(body): Json<Value>,
) -> Response {
  let mut book = stand.book();
  let outcome = body["outcome"].as_str().unwrap_or_default().to_owned();
  let cancel = body["cancel"].as_bool().unwrap_or_default();
  let Some(request) = book.requests.iter_mut().find(|one| one.token == token) else {
    return absent("request");
  };
  let finished = request.state != "ended";

  request.state = "ended".to_owned();
  request.outcome = Some(outcome.clone());
  book.finished.push((token, outcome, cancel));

  Json(json!({ "finished": finished })).into_response()
}

async fn reserve(State(stand): State<Stand>, Json(asking): Json<ReservationAsking>) -> Json<Reservation> {
  let mut book = stand.book();
  let token = format!("res-{}", book.reservations.len() + 1);
  let seat = Seat {
    state: book.seat_state.clone(),
  };
  let held = Reservation {
    token: token.clone(),
    state: "held".to_owned(),
    created_at: 3,
    asking,
  };

  book.seats.insert(token, vec![seat; held.asking.wanted.max(0) as usize]);
  book.reservations.push(held.clone());

  Json(held)
}

async fn reservations(State(stand): State<Stand>, query: Owner) -> Json<Vec<Reservation>> {
  Json(
    stand
      .book()
      .reservations
      .iter()
      .filter(|one| one.state == "held" && of_owner(&query, &one.asking.owner))
      .cloned()
      .collect(),
  )
}

async fn promote(State(stand): State<Stand>) -> Response {
  stand.book().promoted += 1;

  done()
}

async fn seats(State(stand): State<Stand>, Path(token): Path<String>) -> Json<Vec<Seat>> {
  Json(stand.book().seats.get(&token).cloned().unwrap_or_default())
}

async fn pings(
  State(stand): State<Stand>,
  Path(token): Path<String>,
) -> Json<Vec<bmo_artifact::clients::agency::Ping>> {
  Json(stand.book().pings.get(&token).cloned().unwrap_or_default())
}

async fn release(
  State(stand): State<Stand>,
  Path(token): Path<String>,
  Json(body): Json<Value>,
) -> Response {
  let mut book = stand.book();
  let reason = body["reason"].as_str().unwrap_or_default().to_owned();
  let Some(held) = book.reservations.iter_mut().find(|one| one.token == token) else {
    return absent("reservation");
  };
  let released = held.state == "held";

  held.state = "released".to_owned();
  book.released.push((token, reason));

  Json(json!({ "released": released })).into_response()
}

async fn write_place(
  State(stand): State<Stand>,
  Path(place): Path<String>,
  Json(write): Json<PlaceWrite>,
) -> Response {
  stand.book().writes.push((place, write));

  done()
}

async fn set_limit(
  State(stand): State<Stand>,
  Path((provider, tier)): Path<(String, String)>,
  Json(body): Json<Value>,
) -> Response {
  let capacity = body["capacity"].as_i64().unwrap_or_default();

  stand.book().limits.push((provider, tier, capacity));

  done()
}

async fn reset_limits(State(stand): State<Stand>) -> Response {
  stand.book().limits_reset += 1;

  done()
}

async fn count_run(State(stand): State<Stand>, Path(agent): Path<String>) -> Response {
  stand.book().counted.push(agent);

  done()
}

async fn note(State(stand): State<Stand>, Json(note): Json<Note>) -> Response {
  stand.book().notes.push(note);

  done()
}

async fn registration(
  State(stand): State<Stand>,
  Query(query): Query<HashMap<String, String>>,
) -> Response {
  let home = query.get("home").cloned().unwrap_or_default();

  match stand.book().registrations.iter().find(|(owned, _)| *owned == home) {
    Some((_, held)) => Json(held.clone()).into_response(),
    None => absent("registration"),
  }
}

async fn start_apart(State(stand): State<Stand>, Json(asking): Json<ApartAsking>) -> Json<Value> {
  let mut book = stand.book();

  book.started.push(asking);

  Json(json!({ "id": format!("apart-{}", book.started.len()) }))
}

async fn apart(State(stand): State<Stand>, Path(id): Path<String>) -> Json<Apart> {
  Json(Apart {
    id,
    ..stand.book().apart.clone()
  })
}

async fn stop_apart(State(stand): State<Stand>, Path(id): Path<String>) -> Response {
  let mut book = stand.book();

  book.stopped.push(id);
  book.apart.state = "stopped".to_owned();

  done()
}
