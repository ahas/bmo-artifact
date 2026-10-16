//! bmo-agency, which keeps the providers, tiers and routes a research run resolves on, the common
//! co-worker pool, the run ledger, the agent log and the registrations, and carries every run apart
//! (`docs/api.md`, "bmo-agency"). `Agency` declares every call this repository makes of it, and
//! `AgencyClient` makes them over HTTP.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

use axum::http::StatusCode;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::plumbing::http::Refused;

/// What a call to bmo-agency comes to.
pub type Asked<'a, T> = Pin<Box<dyn Future<Output = Result<T, AgencyFault>> + Send + 'a>>;

/// Why a call to bmo-agency gave nothing.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AgencyFault {
  /// bmo-agency refused it, with its status, its code and its sentence.
  #[error("{error}")]
  Refused { status: u16, code: String, error: String },
  /// It did not reach bmo-agency, or the answer did not read as its shape.
  #[error("{0}")]
  Failed(String),
}

impl From<AgencyFault> for Refused {
  fn from(fault: AgencyFault) -> Self {
    match fault {
      AgencyFault::Refused { status, code, error } => Refused::new(
        StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
        code,
        error,
      ),
      AgencyFault::Failed(error) => Refused::new(StatusCode::BAD_GATEWAY, "agency_failed", error),
    }
  }
}

/// One provider, in the provider order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provider {
  pub name: String,
  pub label: String,
  pub enabled: bool,
  pub registered: bool,
  /// The work lines its program carries.
  #[serde(default)]
  pub lines: Vec<String>,
  /// Whether it lends co-workers.
  pub coworkers: bool,
}

impl Provider {
  /// Whether its program carries a line.
  pub fn carries(&self, line: &str) -> bool {
    self.lines.iter().any(|one| one == line)
  }
}

/// One model tier.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tier {
  pub name: String,
  /// Whether a co-worker may run at it.
  pub coworker: bool,
  /// A co-worker tier's default pool capacity.
  #[serde(default)]
  pub default_capacity: Option<i64>,
}

/// The tier a work key is assigned.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignment {
  pub key: String,
  pub tier: String,
}

/// What a route is asked for: a work key's tier on a provider, or a tier on its own.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteAsked {
  pub work_key: Option<String>,
  pub provider: Option<String>,
  pub tier: String,
}

/// The model and effort a route resolves to.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mapping {
  pub provider: String,
  pub tier: String,
  pub model: String,
  #[serde(default)]
  pub effort: Option<String>,
  #[serde(default)]
  pub answer_tokens: Option<i64>,
}

/// One role of the common co-worker pool.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
  pub name: String,
  #[serde(default)]
  pub description: String,
  pub enabled: bool,
  pub pooled: bool,
  /// Whether it is the worker's own control role, never lent.
  pub control: bool,
}

/// A model a place held when its capacity was last taken.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotModel {
  pub model: String,
  #[serde(default)]
  pub effort: Option<String>,
}

/// One provider and tier's places in the pool.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
  pub provider: String,
  pub tier: String,
  pub capacity: i64,
  pub default_capacity: i64,
  #[serde(default)]
  pub snapshot_capacities: Vec<i64>,
  #[serde(default)]
  pub snapshot_models: Vec<SnapshotModel>,
  pub active: i64,
  pub reserved: i64,
  pub waiting: i64,
  pub yielding_active: i64,
  pub yielding_waiting: i64,
}

/// A request for a place.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asking {
  pub request_key: String,
  pub owner: String,
  pub source: String,
  pub role: String,
  pub work_key: String,
  pub provider: String,
  pub model: String,
  #[serde(default)]
  pub effort: Option<String>,
  pub tier: String,
  pub yielding: bool,
  #[serde(default)]
  pub place_key: Option<String>,
  #[serde(default)]
  pub turn: Option<i64>,
  #[serde(default)]
  pub reservation: Option<String>,
}

/// A request for a place as the pool holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolRequest {
  #[serde(flatten)]
  pub asking: Asking,
  pub token: String,
  /// `waiting`, `active` or `ended`.
  pub state: String,
  pub enqueued_at: i64,
  #[serde(default)]
  pub leased_at: Option<i64>,
  #[serde(default)]
  pub child: Option<String>,
  #[serde(default)]
  pub ended_at: Option<i64>,
  #[serde(default)]
  pub outcome: Option<String>,
}

/// Seats asked to be held for one owner's branches.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReservationAsking {
  pub reservation_key: String,
  pub owner: String,
  pub source: String,
  pub role: String,
  pub provider: String,
  pub model: String,
  #[serde(default)]
  pub effort: Option<String>,
  pub tier: String,
  pub wanted: i64,
}

/// Seats as the pool holds them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reservation {
  #[serde(flatten)]
  pub asking: ReservationAsking,
  pub token: String,
  /// `held` or `released`.
  pub state: String,
  pub created_at: i64,
}

/// One seat of a reservation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seat {
  /// `reserved` or `waiting`.
  pub state: String,
}

/// One time the pool kept a reserved place warm.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping {
  pub place: String,
  pub at: i64,
  #[serde(default)]
  pub outcome: Option<String>,
  #[serde(default)]
  pub answered_at: Option<i64>,
}

/// What a place a branch answered on was last written with.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceWrite {
  pub provider: String,
  pub model: String,
  pub effort: Option<String>,
  pub upstream: Option<String>,
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

/// A note in a run's log.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
  pub agent: String,
  pub run: String,
  pub chat_id: Option<i64>,
  pub text: String,
}

/// The registration that owns a home.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registration {
  pub name: String,
  pub keeps_research: bool,
}

/// What a run apart goes out on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Road {
  /// A tier's model on a provider.
  Key {
    provider: String,
    model: String,
    effort: Option<String>,
    tier: String,
  },
  /// A registration, with its own model.
  Registration { name: String },
}

/// The instructions bmo-agency composes for a run apart's line, and the caller's own where it wrote them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instructions {
  pub name: String,
  pub text: Option<String>,
}

/// The run log every line a run apart writes is fed to.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApartLog {
  pub run: String,
  pub chat_id: Option<i64>,
}

/// A run apart asked for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApartAsking {
  /// The agent it is counted under.
  pub agent: String,
  pub line: String,
  pub title: String,
  pub work_key: String,
  pub road: Road,
  pub instructions: Instructions,
  /// The variables its web tools are told.
  pub environment: BTreeMap<String, String>,
  pub prompt: String,
  pub log: Option<ApartLog>,
}

/// What a run apart answered.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApartReply {
  pub text: String,
  #[serde(default)]
  pub model: Option<String>,
  #[serde(default)]
  pub effort: Option<String>,
  #[serde(default)]
  pub context_tokens: Option<i64>,
  /// The names of its own work still running as it ended.
  #[serde(default)]
  pub live_tasks: Vec<String>,
}

/// A run apart as it stands.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Apart {
  pub id: String,
  /// `running`, `answered`, `failed` or `stopped`.
  pub state: String,
  /// Every line its record has gained so far, in order.
  #[serde(default)]
  pub streamed: Vec<String>,
  #[serde(default)]
  pub reply: Option<ApartReply>,
  #[serde(default)]
  pub failure: Option<String>,
}

impl Apart {
  /// Whether it is still going.
  pub fn running(&self) -> bool {
    self.state == "running"
  }
}

#[derive(Deserialize)]
struct Started {
  id: String,
}

#[derive(Deserialize)]
struct Finished {
  finished: bool,
}

#[derive(Deserialize)]
struct Released {
  released: bool,
}

/// Every call this repository makes of bmo-agency (`docs/api.md`, "bmo-agency"). A read of one thing by
/// its id answers nothing where it does not exist.
pub trait Agency: Send + Sync + Debug {
  /// The providers, in the provider order.
  fn providers(&self) -> Asked<'_, Vec<Provider>>;
  /// The model tiers.
  fn tiers(&self) -> Asked<'_, Vec<Tier>>;
  /// Every work key's tier.
  fn assignments(&self) -> Asked<'_, Vec<Assignment>>;
  /// The route that serves a work key's tier, or a tier on its own, on a provider; none where none does.
  fn resolve<'a>(&'a self, asked: &'a RouteAsked) -> Asked<'a, Option<Mapping>>;
  /// Whether a provider and a model accept an effort; refused with the sentence where they do not.
  fn check_effort<'a>(&'a self, provider: &'a str, model: &'a str, effort: &'a str) -> Asked<'a, ()>;

  /// The pool's roles.
  fn roles(&self) -> Asked<'_, Vec<Role>>;
  /// The pool's places, one per provider and tier.
  fn status(&self) -> Asked<'_, Vec<Place>>;
  /// A request for a place, queued.
  fn ask_place<'a>(&'a self, asking: &'a Asking) -> Asked<'a, PoolRequest>;
  /// A request as it stands.
  fn request<'a>(&'a self, token: &'a str) -> Asked<'a, Option<PoolRequest>>;
  /// The requests waiting or active, of one owner or of every one.
  fn requests<'a>(&'a self, owner: Option<&'a str>) -> Asked<'a, Vec<PoolRequest>>;
  /// A request tried for a place: `active` where one was leased.
  fn lease<'a>(&'a self, token: &'a str) -> Asked<'a, PoolRequest>;
  /// A leased request bound to the work it carries.
  fn bind<'a>(&'a self, token: &'a str, child: &'a str) -> Asked<'a, PoolRequest>;
  /// A request ended, cancelled where it never ran; whether it was this call that ended it.
  fn finish<'a>(&'a self, token: &'a str, outcome: &'a str, cancel: bool) -> Asked<'a, bool>;

  /// Seats held for one owner's branches.
  fn reserve<'a>(&'a self, asking: &'a ReservationAsking) -> Asked<'a, Reservation>;
  /// The reservations held, of one owner or of every one.
  fn reservations<'a>(&'a self, owner: Option<&'a str>) -> Asked<'a, Vec<Reservation>>;
  /// A reservation's seats.
  fn seats<'a>(&'a self, token: &'a str) -> Asked<'a, Vec<Seat>>;
  /// The times a reservation's places were kept warm.
  fn pings<'a>(&'a self, token: &'a str) -> Asked<'a, Vec<Ping>>;
  /// A reservation given back; whether it was this call that gave it back.
  fn release<'a>(&'a self, token: &'a str, reason: &'a str) -> Asked<'a, bool>;
  /// Every reservation's waiting seats let in where places are free.
  fn promote(&self) -> Asked<'_, ()>;
  /// What a place a branch answered on was written with.
  fn write_place<'a>(&'a self, place: &'a str, write: &'a PlaceWrite) -> Asked<'a, ()>;
  /// A co-worker tier's limit on a provider set.
  fn set_limit<'a>(&'a self, provider: &'a str, tier: &'a str, capacity: i64) -> Asked<'a, ()>;
  /// Every limit put back to its default.
  fn reset_limits(&self) -> Asked<'_, ()>;

  /// A run of the ledger.
  fn run(&self, id: i64) -> Asked<'_, Option<Run>>;
  /// A run closed; false where it was already closed.
  fn close_run<'a>(&'a self, id: i64, close: &'a RunClose) -> Asked<'a, bool>;
  /// A run counted on its agent.
  fn count_run<'a>(&'a self, agent: &'a str) -> Asked<'a, ()>;
  /// A note written in a run's log.
  fn note<'a>(&'a self, note: &'a Note) -> Asked<'a, ()>;
  /// The registration that owns a home, none where no registration does.
  fn registration_by_home<'a>(&'a self, home: &'a str) -> Asked<'a, Option<Registration>>;

  /// A run apart started; its id.
  fn start_apart<'a>(&'a self, asking: &'a ApartAsking) -> Asked<'a, String>;
  /// A run apart as it stands.
  fn apart<'a>(&'a self, id: &'a str) -> Asked<'a, Apart>;
  /// A run apart stopped.
  fn stop_apart<'a>(&'a self, id: &'a str) -> Asked<'a, ()>;
}

/// bmo-agency at the address the settings name, where they name one.
#[derive(Clone, Debug)]
pub struct AgencyClient {
  http: reqwest::Client,
  address: Option<String>,
}

#[derive(Deserialize)]
struct Refusal {
  error: String,
  #[serde(default)]
  code: Option<String>,
}

// One call as it goes out: what it is called in a failure, its method, its path's segments, its query and
// its body.
struct Call<'a> {
  what: &'a str,
  method: Method,
  path: &'a [&'a str],
  query: Vec<(&'a str, &'a str)>,
  body: Option<Value>,
}

impl<'a> Call<'a> {
  fn new(what: &'a str, method: Method, path: &'a [&'a str]) -> Self {
    Self {
      what,
      method,
      path,
      query: Vec::new(),
      body: None,
    }
  }

  fn query(mut self, name: &'a str, value: Option<&'a str>) -> Self {
    if let Some(value) = value {
      self.query.push((name, value));
    }

    self
  }

  fn body(mut self, body: impl Serialize) -> Self {
    self.body = Some(json!(body));
    self
  }
}

impl AgencyClient {
  pub fn new(address: Option<String>) -> Self {
    Self {
      http: reqwest::Client::new(),
      address,
    }
  }

  // The answer's body, none where it answered `absent`; every other refusal handed on, and a call that
  // went wrong named.
  async fn send(&self, call: Call<'_>, absent: Option<StatusCode>) -> Result<Option<Vec<u8>>, AgencyFault> {
    let failed = |why: String| AgencyFault::Failed(format!("bmo-agency could not {}: {why}", call.what));
    let Some(address) = &self.address else {
      return Err(AgencyFault::Failed(
        "the agency's address is not set: set agency under [units] in the settings".to_owned(),
      ));
    };
    let mut url = url::Url::parse(address).map_err(|why| failed(why.to_string()))?;

    url
      .path_segments_mut()
      .map_err(|()| failed(format!("{address} cannot stand under a path")))?
      .pop_if_empty()
      .extend(call.path);

    if !call.query.is_empty() {
      url.query_pairs_mut().extend_pairs(&call.query);
    }

    let mut request = self.http.request(call.method.clone(), url);

    if let Some(body) = &call.body {
      request = request.json(body);
    }

    let answer = request.send().await.map_err(|why| failed(why.to_string()))?;
    let status = answer.status();
    let body = answer.bytes().await.map_err(|why| failed(why.to_string()))?;

    if status.is_success() {
      return Ok(Some(body.to_vec()));
    }

    if absent.is_some_and(|absent| absent.as_u16() == status.as_u16()) {
      return Ok(None);
    }

    match serde_json::from_slice::<Refusal>(&body) {
      Ok(refusal) => Err(AgencyFault::Refused {
        status: status.as_u16(),
        code: refusal.code.unwrap_or_else(|| "agency_refused".to_owned()),
        error: refusal.error,
      }),
      Err(_) => Err(failed(format!("it answered {status}"))),
    }
  }

  // The answer read as its shape.
  async fn read<T: DeserializeOwned>(&self, call: Call<'_>) -> Result<T, AgencyFault> {
    let what = call.what;
    let body = self.send(call, None).await?.unwrap_or_default();

    shaped(what, &body)
  }

  // The answer read as its shape, none where it answered 404.
  async fn found<T: DeserializeOwned>(&self, call: Call<'_>) -> Result<Option<T>, AgencyFault> {
    let what = call.what;

    match self.send(call, Some(StatusCode::NOT_FOUND)).await? {
      Some(body) => shaped(what, &body).map(Some),
      None => Ok(None),
    }
  }

  // An answer with no body.
  async fn done(&self, call: Call<'_>) -> Result<(), AgencyFault> {
    self.send(call, None).await.map(|_| ())
  }
}

fn shaped<T: DeserializeOwned>(what: &str, body: &[u8]) -> Result<T, AgencyFault> {
  serde_json::from_slice(body)
    .map_err(|why| AgencyFault::Failed(format!("bmo-agency could not {what}: its answer is out of shape: {why}")))
}

impl Agency for AgencyClient {
  fn providers(&self) -> Asked<'_, Vec<Provider>> {
    Box::pin(self.read(Call::new("list the providers", Method::GET, &["providers"])))
  }

  fn tiers(&self) -> Asked<'_, Vec<Tier>> {
    Box::pin(self.read(Call::new("list the tiers", Method::GET, &["tiers"])))
  }

  fn assignments(&self) -> Asked<'_, Vec<Assignment>> {
    Box::pin(self.read(Call::new("list the tier assignments", Method::GET, &["tiers", "assignments"])))
  }

  fn resolve<'a>(&'a self, asked: &'a RouteAsked) -> Asked<'a, Option<Mapping>> {
    Box::pin(self.found(Call::new("resolve a route", Method::POST, &["routes", "resolve"]).body(asked)))
  }

  fn check_effort<'a>(&'a self, provider: &'a str, model: &'a str, effort: &'a str) -> Asked<'a, ()> {
    Box::pin(self.done(
      Call::new("check an effort", Method::POST, &["routes", "check-effort"])
        .body(json!({ "provider": provider, "model": model, "effort": effort })),
    ))
  }

  fn roles(&self) -> Asked<'_, Vec<Role>> {
    Box::pin(self.read(Call::new("list the pool's roles", Method::GET, &["pool", "roles"])))
  }

  fn status(&self) -> Asked<'_, Vec<Place>> {
    Box::pin(self.read(Call::new("read the pool's status", Method::GET, &["pool", "status"])))
  }

  fn ask_place<'a>(&'a self, asking: &'a Asking) -> Asked<'a, PoolRequest> {
    Box::pin(self.read(Call::new("ask for a place", Method::POST, &["pool", "requests"]).body(asking)))
  }

  fn request<'a>(&'a self, token: &'a str) -> Asked<'a, Option<PoolRequest>> {
    Box::pin(async move {
      self.found(Call::new("read a pool request", Method::GET, &["pool", "requests", token])).await
    })
  }

  fn requests<'a>(&'a self, owner: Option<&'a str>) -> Asked<'a, Vec<PoolRequest>> {
    Box::pin(
      self.read(Call::new("list the open pool requests", Method::GET, &["pool", "requests"]).query("owner", owner)),
    )
  }

  fn lease<'a>(&'a self, token: &'a str) -> Asked<'a, PoolRequest> {
    Box::pin(async move {
      self.read(Call::new("try for a place", Method::POST, &["pool", "requests", token, "lease"])).await
    })
  }

  fn bind<'a>(&'a self, token: &'a str, child: &'a str) -> Asked<'a, PoolRequest> {
    Box::pin(async move {
      self
        .read(
          Call::new("bind a place", Method::POST, &["pool", "requests", token, "bind"]).body(json!({ "child": child })),
        )
        .await
    })
  }

  fn finish<'a>(&'a self, token: &'a str, outcome: &'a str, cancel: bool) -> Asked<'a, bool> {
    Box::pin(async move {
      let finished: Finished = self
        .read(
          Call::new("finish a pool request", Method::POST, &["pool", "requests", token, "finish"])
            .body(json!({ "outcome": outcome, "cancel": cancel })),
        )
        .await?;

      Ok(finished.finished)
    })
  }

  fn reserve<'a>(&'a self, asking: &'a ReservationAsking) -> Asked<'a, Reservation> {
    Box::pin(self.read(Call::new("reserve seats", Method::POST, &["pool", "reservations"]).body(asking)))
  }

  fn reservations<'a>(&'a self, owner: Option<&'a str>) -> Asked<'a, Vec<Reservation>> {
    Box::pin(
      self.read(
        Call::new("list the held reservations", Method::GET, &["pool", "reservations"]).query("owner", owner),
      ),
    )
  }

  fn seats<'a>(&'a self, token: &'a str) -> Asked<'a, Vec<Seat>> {
    Box::pin(async move {
      self
        .read(Call::new("read a reservation's seats", Method::GET, &["pool", "reservations", token, "seats"]))
        .await
    })
  }

  fn pings<'a>(&'a self, token: &'a str) -> Asked<'a, Vec<Ping>> {
    Box::pin(async move {
      self
        .read(Call::new("read a reservation's pings", Method::GET, &["pool", "reservations", token, "pings"]))
        .await
    })
  }

  fn release<'a>(&'a self, token: &'a str, reason: &'a str) -> Asked<'a, bool> {
    Box::pin(async move {
      let released: Released = self
        .read(
          Call::new("release a reservation", Method::POST, &["pool", "reservations", token, "release"])
            .body(json!({ "reason": reason })),
        )
        .await?;

      Ok(released.released)
    })
  }

  fn promote(&self) -> Asked<'_, ()> {
    Box::pin(self.done(Call::new("promote the reservations", Method::POST, &["pool", "reservations", "promote"])))
  }

  fn write_place<'a>(&'a self, place: &'a str, write: &'a PlaceWrite) -> Asked<'a, ()> {
    Box::pin(async move {
      self
        .done(Call::new("record a place's write", Method::PUT, &["pool", "places", place, "write"]).body(write))
        .await
    })
  }

  fn set_limit<'a>(&'a self, provider: &'a str, tier: &'a str, capacity: i64) -> Asked<'a, ()> {
    Box::pin(async move {
      self
        .done(
          Call::new("set a pool limit", Method::PUT, &["pool", "limits", provider, tier])
            .body(json!({ "capacity": capacity })),
        )
        .await
    })
  }

  fn reset_limits(&self) -> Asked<'_, ()> {
    Box::pin(self.done(Call::new("reset the pool limits", Method::DELETE, &["pool", "limits"])))
  }

  fn run(&self, id: i64) -> Asked<'_, Option<Run>> {
    Box::pin(async move {
      let id = id.to_string();

      self.found(Call::new("read a run", Method::GET, &["runs", &id])).await
    })
  }

  fn close_run<'a>(&'a self, id: i64, close: &'a RunClose) -> Asked<'a, bool> {
    Box::pin(async move {
      let id = id.to_string();
      let closed = self
        .send(Call::new("close a run", Method::POST, &["runs", &id, "close"]).body(close), Some(StatusCode::CONFLICT))
        .await?;

      Ok(closed.is_some())
    })
  }

  fn count_run<'a>(&'a self, agent: &'a str) -> Asked<'a, ()> {
    Box::pin(async move { self.done(Call::new("count a run", Method::POST, &["agents", agent, "runs"])).await })
  }

  fn note<'a>(&'a self, note: &'a Note) -> Asked<'a, ()> {
    Box::pin(self.done(Call::new("write a note", Method::POST, &["log", "notes"]).body(note)))
  }

  fn registration_by_home<'a>(&'a self, home: &'a str) -> Asked<'a, Option<Registration>> {
    Box::pin(
      self.found(
        Call::new("find a home's registration", Method::GET, &["registrations", "by-home"]).query("home", Some(home)),
      ),
    )
  }

  fn start_apart<'a>(&'a self, asking: &'a ApartAsking) -> Asked<'a, String> {
    Box::pin(async move {
      let started: Started = self.read(Call::new("start a run apart", Method::POST, &["apart"]).body(asking)).await?;

      Ok(started.id)
    })
  }

  fn apart<'a>(&'a self, id: &'a str) -> Asked<'a, Apart> {
    Box::pin(async move { self.read(Call::new("read a run apart", Method::GET, &["apart", id])).await })
  }

  fn stop_apart<'a>(&'a self, id: &'a str) -> Asked<'a, ()> {
    Box::pin(async move { self.done(Call::new("stop a run apart", Method::POST, &["apart", id, "stop"])).await })
  }
}
