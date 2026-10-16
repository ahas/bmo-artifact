//! Routes, the calls they answer, their refusals, and the router the server stands them on.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Query, Request, State};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{MethodFilter, MethodRouter};
use serde_json::{Value, json};

use crate::pool::Archive;
use crate::settings::Settings;

pub use inventory;

/// One route the server answers, registered beside its answer with `bmo_artifact::route!`.
pub trait Route: Send + Sync + 'static {
  const METHOD: Method;
  /// The path, in the router's own syntax: `/api/archive/{id}`.
  const PATH: &'static str;

  fn answer(&self, call: Call) -> impl Future<Output = Result<Response, Refused>> + Send;
}

type Answering<'a> = Pin<Box<dyn Future<Output = Result<Response, Refused>> + Send + 'a>>;

/// A route as the router holds it, whatever its type.
pub trait Answers: Send + Sync {
  fn method(&self) -> Method;
  fn path(&self) -> &'static str;
  fn answer(&self, call: Call) -> Answering<'_>;
}

impl<R: Route> Answers for R {
  fn method(&self) -> Method {
    R::METHOD
  }

  fn path(&self) -> &'static str {
    R::PATH
  }

  fn answer(&self, call: Call) -> Answering<'_> {
    Box::pin(Route::answer(self, call))
  }
}

/// One registered route.
pub struct Registered(pub &'static dyn Answers);

inventory::collect!(Registered);

/// Registers a route with the server: `bmo_artifact::route!(ResearchSettings);`
#[macro_export]
macro_rules! route {
  ($route:expr) => {
    $crate::plumbing::http::inventory::submit! { $crate::plumbing::http::Registered(&$route) }
  };
}

/// Every route linked into this binary.
pub fn routes() -> impl Iterator<Item = &'static dyn Answers> {
  inventory::iter::<Registered>.into_iter().map(|entry| entry.0)
}

/// A route's name as a caller writes it: its method and its path.
pub fn route_name(method: &Method, path: &str) -> String {
  format!("{method} {path}")
}

/// What is wrong with the routes linked in: two answers for one method and path would leave which one
/// answers to the order they were linked, so the server refuses to stand on any.
pub fn route_faults() -> Vec<String> {
  let mut seen = BTreeSet::new();

  routes()
    .map(|route| route_name(&route.method(), route.path()))
    .filter(|name| !seen.insert(name.clone()))
    .map(|name| format!("{name}: registered twice"))
    .collect()
}

/// A refusal: a status, and a body naming why under `error` and `code` (`docs/api.md`, "Errors").
#[derive(Debug, PartialEq, Eq)]
pub struct Refused {
  pub status: StatusCode,
  /// The snake_case code a caller matches on.
  pub code: String,
  /// The English sentence a person reads.
  pub error: String,
}

/// The code of a failure a route did not expect.
pub const FAILED: &str = "failed";
/// The code of a body that is not JSON.
pub const NOT_JSON: &str = "not_json";

impl Refused {
  /// A refusal under the status and the code the route gives it.
  pub fn new(status: StatusCode, code: impl Into<String>, error: impl Into<String>) -> Self {
    Self {
      status,
      code: code.into(),
      error: error.into(),
    }
  }

  /// What a route answers for a failure it did not expect: 400, with the failure's own words.
  pub fn failed(error: impl std::fmt::Display) -> Self {
    Self::new(StatusCode::BAD_REQUEST, FAILED, error.to_string())
  }
}

impl IntoResponse for Refused {
  fn into_response(self) -> Response {
    (self.status, Json(json!({ "error": self.error, "code": self.code }))).into_response()
  }
}

/// What the server stands on, which every call carries.
#[derive(Clone, Debug)]
pub struct Standing {
  pub settings: Settings,
  /// The database the settings name, opened on the first statement a route asks of it.
  pub archive: Archive,
}

impl Standing {
  pub fn new(settings: Settings) -> Self {
    let archive = Archive::new(settings.database.url.clone());

    Self { settings, archive }
  }
}

/// One request a route answers.
pub struct Call {
  pub request: Request,
  standing: Arc<Standing>,
}

// The most a body read whole may carry: 128 MiB.
const BODY_LIMIT: usize = 128 * 1024 * 1024;

impl Call {
  pub fn new(request: Request, standing: Arc<Standing>) -> Self {
    Self { request, standing }
  }

  /// What the server stands on.
  pub fn standing(&self) -> &Standing {
    &self.standing
  }

  /// One query parameter, the first where it is given more than once.
  pub fn query(&self, name: &str) -> Option<String> {
    let Query(pairs) = Query::<Vec<(String, String)>>::try_from_uri(self.request.uri()).ok()?;

    pairs.into_iter().find(|(key, _)| key == name).map(|(_, value)| value)
  }

  /// The body read as JSON, whatever its shape; a body that is not JSON is refused. A route reads it once.
  pub async fn body(&mut self) -> Result<Value, Refused> {
    let body = std::mem::take(self.request.body_mut());
    let bytes = axum::body::to_bytes(body, BODY_LIMIT).await.map_err(Refused::failed)?;

    serde_json::from_slice(&bytes)
      .map_err(|_| Refused::new(StatusCode::BAD_REQUEST, NOT_JSON, "the body is not JSON"))
  }
}

/// An answer: a JSON body under a status.
pub fn answer(status: StatusCode, body: Value) -> Response {
  (status, Json(body)).into_response()
}

/// The router answering every registered route, or what is wrong with the routes linked in.
pub fn router(standing: Standing) -> Result<Router, Vec<String>> {
  let faults = route_faults();

  if !faults.is_empty() {
    return Err(faults);
  }

  let mut by_path: BTreeMap<&'static str, MethodRouter<Arc<Standing>>> = BTreeMap::new();
  let mut faults = Vec::new();

  for route in routes() {
    let Ok(filter) = MethodFilter::try_from(route.method()) else {
      faults.push(format!("{}: the method cannot be routed", route_name(&route.method(), route.path())));
      continue;
    };
    let handler = move |State(standing): State<Arc<Standing>>, request: Request| async move {
      route
        .answer(Call::new(request, standing))
        .await
        .unwrap_or_else(IntoResponse::into_response)
    };
    let methods = by_path.remove(route.path()).unwrap_or_default();

    by_path.insert(route.path(), methods.on(filter, handler));
  }

  if !faults.is_empty() {
    return Err(faults);
  }

  let router = by_path
    .into_iter()
    .fold(Router::new(), |router, (path, methods)| router.route(path, methods));

  Ok(router.with_state(Arc::new(standing)))
}
