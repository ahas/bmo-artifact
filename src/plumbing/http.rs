//! Routes, the calls they answer, their refusals, and the router the server stands them on, behind the
//! bearer token every call presents and the reach each route gives the console's (`docs/api.md`, "The wire"
//! and "The console").

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::Router;
use axum::extract::{FromRequestParts, Query, RawPathParams, Request, State};
use axum::http::{Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{MethodFilter, MethodRouter};
use schemars::generate::SchemaGenerator;
use sea_orm::DatabaseConnection;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use tokio::sync::OnceCell;

use crate::console::{ConsoleBundle, screen_admits};
use crate::documents::preview::PreviewCache;
use crate::events::Publisher;
use crate::plumbing::openapi::Operation;
use crate::plumbing::refusal::Refusal;
use crate::pool::Archive;
use crate::reader::{ExplorerReader, PageReader};
use crate::settings::Settings;
use crate::store::documents::settle;
use crate::wording::{Wording, WordingError};

pub use inventory;

/// How far the console's token reaches a route (`docs/api.md`, "The console").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleReach {
  /// Not at all: the screens do not use the route.
  Closed,
  /// With no role and no owner key: what a console reads for no reader.
  Open,
  /// As a screen's call, under the role and the owner key the screens need.
  Screen,
}

impl ConsoleReach {
  /// The word the OpenAPI document states it by.
  pub fn word(self) -> &'static str {
    match self {
      Self::Closed => "closed",
      Self::Open => "open",
      Self::Screen => "screen",
    }
  }
}

/// Whose token a call presents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Credential {
  /// The service's, which reaches every route.
  Service,
  /// The console's, relaying for a reader.
  Console,
}

/// One route the server answers, registered beside its answer with `bmo_artifact::route!`.
pub trait Route: Send + Sync + 'static {
  const METHOD: Method;
  /// The path, in the router's own syntax: `/documents/{documentId}`.
  const PATH: &'static str;
  /// How far the console's token reaches it; a route the screens do not use is closed to it.
  const CONSOLE: ConsoleReach = ConsoleReach::Closed;

  fn answer(&self, call: Call) -> impl Future<Output = Result<Response, Refused>> + Send;

  /// The route as the OpenAPI document states it.
  fn operation(&self, generator: &mut SchemaGenerator) -> Operation;
}

type Answering<'a> = Pin<Box<dyn Future<Output = Result<Response, Refused>> + Send + 'a>>;

/// A route as the router holds it, whatever its type.
pub trait Answers: Send + Sync {
  fn method(&self) -> Method;
  fn path(&self) -> &'static str;
  fn console(&self) -> ConsoleReach;
  fn answer(&self, call: Call) -> Answering<'_>;
  fn operation(&self, generator: &mut SchemaGenerator) -> Operation;
}

impl<R: Route> Answers for R {
  fn method(&self) -> Method {
    R::METHOD
  }

  fn path(&self) -> &'static str {
    R::PATH
  }

  fn console(&self) -> ConsoleReach {
    R::CONSOLE
  }

  fn answer(&self, call: Call) -> Answering<'_> {
    Box::pin(Route::answer(self, call))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Route::operation(self, generator)
  }
}

/// One registered route.
pub struct Registered(pub &'static dyn Answers);

inventory::collect!(Registered);

/// Registers a route with the server: `bmo_artifact::route!(ReadDocument);`
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

/// A refusal: a status, and a body of `code` and `error` (`docs/api.md`, "Codes").
#[derive(Debug, PartialEq, Eq)]
pub struct Refused {
  pub status: StatusCode,
  /// The kebab-case code a caller matches on.
  pub code: String,
  /// The English sentence a person reads.
  pub error: String,
}

impl Refused {
  /// A refusal under the status and the code the route gives it.
  pub fn new(status: StatusCode, code: impl Into<String>, error: impl Into<String>) -> Self {
    Self {
      status,
      code: code.into(),
      error: error.into(),
    }
  }

  /// A request not in the shape the route takes.
  pub fn bad(error: impl Into<String>) -> Self {
    Refusal::bad(error).into()
  }
}

impl IntoResponse for Refused {
  fn into_response(self) -> Response {
    (self.status, Json(json!({ "code": self.code, "error": self.error }))).into_response()
  }
}

/// What the server stands on, which every call carries.
#[derive(Clone)]
pub struct Standing {
  pub settings: Settings,
  /// The database the settings name, opened on the first statement a route asks of it.
  pub archive: Archive,
  /// The document wording the settings name, read when the server starts.
  pub wording: Wording,
  /// Where the web is read: bmo-explorer's client; a test hands a reader of its own.
  pub reader: Arc<dyn PageReader>,
  /// The previews of the pages a document refers to, read through the reader and kept under the file root.
  pub previews: Arc<PreviewCache>,
  /// Where events are published.
  pub events: Publisher,
  /// The console bundle the settings' folder holds, read when the server starts.
  pub console: Arc<ConsoleBundle>,
  // Whether the document tables have been settled in this process.
  settled: Arc<OnceCell<()>>,
}

impl std::fmt::Debug for Standing {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter
      .debug_struct("Standing")
      .field("settings", &self.settings)
      .finish_non_exhaustive()
  }
}

impl Standing {
  /// What the settings name, the web read through bmo-explorer's client.
  pub fn new(settings: Settings) -> Result<Self, WordingError> {
    let reader = Arc::new(ExplorerReader::open(&settings.explorer.data_root));

    Self::with_reader(settings, reader)
  }

  /// What the settings name, the web read through `reader`.
  pub fn with_reader(settings: Settings, reader: Arc<dyn PageReader>) -> Result<Self, WordingError> {
    let archive = Archive::new(settings.database.url.clone());
    let wording = Wording::read(settings.wording.file.as_deref())?;
    let previews = Arc::new(PreviewCache::new(settings.files.root.join("web-cache"), Arc::clone(&reader)));
    let console = Arc::new(ConsoleBundle::read(&settings.console.bundle));

    Ok(Self {
      settings,
      archive,
      wording,
      reader,
      previews,
      events: Publisher::default(),
      console,
      settled: Arc::default(),
    })
  }

  /// The database, its document tables settled before the first statement asked of them
  /// (`docs/layout.md`, "Storage").
  pub async fn db(&self) -> Result<&DatabaseConnection, Refused> {
    let db = self.archive.db().map_err(Refusal::from)?;
    let prefix = self.settings.documents.kindless_import_prefix.as_deref();

    self.settled.get_or_try_init(|| settle(db, prefix)).await.map_err(Refused::from)?;

    Ok(db)
  }
}

/// One request a route answers.
pub struct Call {
  pub request: Request,
  standing: Arc<Standing>,
}

// The most a body read whole may carry: an attachment's ceiling in base64, and room beside it.
const BODY_LIMIT: usize = 96 * 1024 * 1024;

impl Call {
  pub fn new(request: Request, standing: Arc<Standing>) -> Self {
    Self { request, standing }
  }

  /// What the server stands on.
  pub fn standing(&self) -> &Standing {
    &self.standing
  }

  /// What the server stands on, shared, for work that outlives the call.
  pub fn shared(&self) -> Arc<Standing> {
    Arc::clone(&self.standing)
  }

  /// The parameters the route's path names, by name.
  pub async fn path_params(&mut self) -> Result<HashMap<String, String>, Refused> {
    let (mut parts, body) = std::mem::take(&mut self.request).into_parts();
    let read = RawPathParams::from_request_parts(&mut parts, &()).await;

    self.request = Request::from_parts(parts, body);

    let params = read.map_err(|why| Refused::bad(why.to_string()))?;

    Ok(params.iter().map(|(name, value)| (name.to_owned(), value.to_owned())).collect())
  }

  /// One path parameter as text.
  pub async fn path_text(&mut self, name: &str) -> Result<String, Refused> {
    self
      .path_params()
      .await?
      .remove(name)
      .ok_or_else(|| Refused::bad(format!("the path names no {name}")))
  }

  /// One path parameter as a member's id: a whole number.
  pub async fn path_id(&mut self, name: &str) -> Result<i64, Refused> {
    let text = self.path_text(name).await?;

    text
      .parse()
      .map_err(|_| Refused::bad(format!("{name} is a whole number, not {text:?}")))
  }

  /// One header's value, where it is given and reads as text.
  pub fn header(&self, name: &str) -> Option<String> {
    self
      .request
      .headers()
      .get(name)
      .and_then(|value| value.to_str().ok())
      .map(str::to_owned)
  }

  /// The query read as its type; a query not of that shape is refused.
  pub fn query<Asked: DeserializeOwned>(&self) -> Result<Asked, Refused> {
    Query::<Asked>::try_from_uri(self.request.uri())
      .map(|Query(asked)| asked)
      .map_err(|why| Refused::bad(format!("the query does not read: {}", why.body_text())))
  }

  /// The body read as its type; a body that is not JSON of that shape is refused, and one over the ceiling
  /// is too large. A route reads it once.
  pub async fn json<Sent: DeserializeOwned>(&mut self) -> Result<Sent, Refused> {
    let body = std::mem::take(self.request.body_mut());
    let bytes = axum::body::to_bytes(body, BODY_LIMIT)
      .await
      .map_err(|_| Refused::from(Refusal::too_large(format!("the body is over {BODY_LIMIT} bytes"))))?;

    serde_json::from_slice(&bytes).map_err(|why| Refused::bad(format!("the body does not read: {why}")))
  }
}

/// An answer: a member or a page as JSON under a status.
pub fn answer(status: StatusCode, body: &impl Serialize) -> Response {
  (status, Json(body)).into_response()
}

// Every call presents one of the settings' tokens as a bearer, and is marked with whose it is; an empty
// token lets nobody in by it.
async fn bearer(State(standing): State<Arc<Standing>>, mut request: Request, next: Next) -> Response {
  let settings = &standing.settings;
  let presented = request
    .headers()
    .get(header::AUTHORIZATION)
    .and_then(|value| value.to_str().ok())
    .and_then(|value| value.strip_prefix("Bearer "));
  let credential = presented.and_then(|presented| {
    [
      (&settings.server.token, Credential::Service),
      (&settings.console.token, Credential::Console),
    ]
    .into_iter()
    .find(|(token, _)| !token.is_empty() && presented == token.as_str())
    .map(|(_, credential)| credential)
  });

  match credential {
    Some(credential) => {
      request.extensions_mut().insert(credential);
      next.run(request).await
    }
    None => Refused::new(StatusCode::UNAUTHORIZED, "unauthorized", "the bearer token is missing or wrong")
      .into_response(),
  }
}

// Whether a call reaches a route: the service's token reaches every one, the console's only as far as the
// route's reach.
fn admitted(reach: ConsoleReach, request: &Request) -> Result<(), Refused> {
  if request.extensions().get::<Credential>() != Some(&Credential::Console) {
    return Ok(());
  }

  match reach {
    ConsoleReach::Closed => Err(Refused::new(
      StatusCode::FORBIDDEN,
      "forbidden",
      "the console's token does not reach this route",
    )),
    ConsoleReach::Open => Ok(()),
    ConsoleReach::Screen => screen_admits(request.headers()).map_err(Refused::from),
  }
}

async fn no_route(request: Request) -> Response {
  Refused::new(
    StatusCode::NOT_FOUND,
    "no-route",
    format!("no route answers {} {}", request.method(), request.uri().path()),
  )
  .into_response()
}

/// The router answering every registered route behind the bearer token, or what is wrong with the routes
/// linked in.
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
      if let Err(refused) = admitted(route.console(), &request) {
        return refused.into_response();
      }

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

  let standing = Arc::new(standing);
  let router = by_path
    .into_iter()
    .fold(Router::new(), |router, (path, methods)| router.route(path, methods.fallback(no_route)))
    .fallback(no_route)
    .layer(middleware::from_fn_with_state(Arc::clone(&standing), bearer));

  Ok(router.with_state(standing))
}
