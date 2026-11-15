//! A stand-in account service, listening on a port of the test's own: the two reads this service makes of it
//! (`docs/api.md`, "What this service calls"), answered from the units and users the test places, behind the
//! account service's bearer token.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use bmo_web_client::{Unit, UnitRef, UserUnit};
use serde_json::json;

/// The token the stand-in takes as its bearer.
pub const ACCOUNTS_TOKEN: &str = "accounts-token";

/// The units and users the stand-in answers, which a test places and moves.
#[derive(Default)]
pub struct Directory {
  /// Each unit's name and parent, none for a company.
  units: Mutex<HashMap<i64, (String, Option<i64>)>>,
  /// The unit each user stands in.
  users: Mutex<HashMap<String, i64>>,
}

impl Directory {
  /// A unit placed under `parent`, or a company where none is given.
  pub fn unit(&self, id: i64, name: &str, parent: Option<i64>) {
    self.units.lock().expect("the units").insert(id, (name.to_owned(), parent));
  }

  /// A user standing in `unit`, moved there where they stood elsewhere.
  pub fn place(&self, user: &str, unit: i64) {
    self.users.lock().expect("the users").insert(user.to_owned(), unit);
  }

  // The units from the company down to `unit`, none where it does not stand.
  fn path(&self, unit: i64) -> Option<Vec<UnitRef>> {
    let units = self.units.lock().expect("the units");
    let mut path = Vec::new();
    let mut at = Some(unit);

    while let Some(id) = at {
      let (name, parent) = units.get(&id)?;

      path.push(UnitRef {
        id,
        name: name.clone(),
      });
      at = *parent;
    }

    path.reverse();
    Some(path)
  }

  /// The stand-in listening on a port of its own until the test ends; its address.
  pub async fn listening(self: &Arc<Self>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = format!("http://{}", listener.local_addr().expect("an address"));
    let router = Router::new()
      .route("/users/{user}/unit", get(user_unit))
      .route("/units/{unit}", get(unit))
      .with_state(Arc::clone(self));

    tokio::spawn(async move { axum::serve(listener, router).await });
    address
  }
}

fn refused(status: StatusCode, code: &str) -> Response {
  (status, Json(json!({ "code": code, "error": code }))).into_response()
}

fn admitted(headers: &HeaderMap) -> bool {
  headers.get("authorization").and_then(|value| value.to_str().ok())
    == Some(format!("Bearer {ACCOUNTS_TOKEN}").as_str())
}

async fn user_unit(
  State(directory): State<Arc<Directory>>,
  headers: HeaderMap,
  Path(user): Path<String>,
) -> Response {
  if !admitted(&headers) {
    return refused(StatusCode::UNAUTHORIZED, "unauthorized");
  }

  let standing = directory.users.lock().expect("the users").get(&user).copied();
  let Some(path) = standing.and_then(|unit| directory.path(unit)) else {
    return refused(StatusCode::NOT_FOUND, "not-found");
  };

  Json(UserUnit {
    user,
    unit: path.last().cloned().expect("a unit"),
    company: path.first().cloned().expect("a company"),
    path,
  })
  .into_response()
}

async fn unit(
  State(directory): State<Arc<Directory>>,
  headers: HeaderMap,
  Path(unit): Path<i64>,
) -> Response {
  if !admitted(&headers) {
    return refused(StatusCode::UNAUTHORIZED, "unauthorized");
  }

  let Some(path) = directory.path(unit) else {
    return refused(StatusCode::NOT_FOUND, "not-found");
  };
  let parent = directory.units.lock().expect("the units").get(&unit).and_then(|(_, parent)| *parent);

  Json(Unit {
    id: unit,
    name: path.last().map(|unit| unit.name.clone()).unwrap_or_default(),
    parent,
    company: path.first().cloned().expect("a company"),
    path,
    children: Vec::new(),
    heads: Vec::new(),
    members: Vec::new(),
  })
  .into_response()
}
