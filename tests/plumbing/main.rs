//! The plumbing: how a command's answer ends the binary, and how the router answers, refuses and reads a
//! call behind the bearer token (`docs/layout.md`, "Plumbing"; `docs/api.md`, "The wire" and "The command").

// A target of its own (bmo-chat's docs/conventions.md, 「Rust builds」): its stand-in verbs register themselves
// in the process's verb table, and in the main target they would stand among the service's own routes in every
// router and in the OpenAPI document the other tests generate.

#[allow(dead_code)]
#[path = "../support/mod.rs"]
mod support;

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;

use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use axum::response::Response;
use bmo_artifact::Refusal;
use bmo_artifact::plumbing::http::{Call, Refused, Route, Standing, answer, route_faults, router};
use bmo_artifact::plumbing::openapi::Operation;
use bmo_artifact::plumbing::verb::{
  Answer, FAILURE, Finished, VerbError, finished, joined, verb_faults, verbs,
};
use bmo_artifact::settings::Settings;
use schemars::generate::SchemaGenerator;
use serde::Deserialize;
use serde_json::{Value, json};
use support::ROOT;
use tower::ServiceExt;

fn lines(lines: &[&str]) -> Vec<String> {
  lines.iter().map(|line| (*line).to_owned()).collect()
}

#[test]
fn an_answer_prints_its_lines_and_succeeds() {
  assert_eq!(
    finished(Ok(lines(&["one", "two"]).into())),
    Finished {
      out: lines(&["one", "two"]),
      err: None,
      code: 0
    }
  );
}

#[test]
fn an_answer_may_end_with_its_own_code() {
  let answered = Answer {
    lines: lines(&["found nothing"]),
    code: 3,
  };

  assert_eq!(finished(Ok(answered)).code, 3);
}

#[test]
fn a_refusal_goes_to_standard_error_and_fails() {
  assert_eq!(
    finished(Err(VerbError::Refused("no such document".to_owned()))),
    Finished {
      out: Vec::new(),
      err: Some("no such document".to_owned()),
      code: FAILURE
    }
  );
}

#[test]
fn a_verb_ending_with_its_own_code_ends_with_it() {
  let ended = VerbError::Ended {
    code: 4,
    said: "the archive is busy".to_owned(),
  };

  assert_eq!(
    finished(Err(ended)),
    Finished {
      out: Vec::new(),
      err: Some("the archive is busy".to_owned()),
      code: 4
    }
  );
}

#[test]
fn a_partial_answer_prints_what_it_did_and_fails() {
  let partly = VerbError::Partly {
    lines: lines(&["saved 12"]),
    refusal: "13 is missing".to_owned(),
  };

  assert_eq!(
    finished(Err(partly)),
    Finished {
      out: lines(&["saved 12"]),
      err: Some("13 is missing".to_owned()),
      code: FAILURE
    }
  );

  let quiet = VerbError::Partly {
    lines: lines(&["saved 12"]),
    refusal: String::new(),
  };

  assert_eq!(finished(Err(quiet)).err, None);
}

#[test]
fn no_command_name_is_claimed_twice_and_each_says_what_it_does_in_its_own_words() {
  assert_eq!(verb_faults(), Vec::<String>::new());

  let names: Vec<&str> = verbs().map(|verb| verb.name()).collect();

  for name in ["serve", "migrate", "health", "openapi", "mcp", "html-read", "version"] {
    assert!(names.contains(&name), "{name} is not registered: {names:?}");
  }

  let mcp = verbs().find(|verb| verb.name() == "mcp").expect("mcp is registered");
  let about = mcp.command().get_about().map(ToString::to_string).unwrap_or_default();

  assert!(about.starts_with("Serves the harness tools over MCP"), "{about}");
}

#[test]
fn a_commands_two_words_are_read_as_its_one_name_and_any_other_line_is_left_as_it_is() {
  let line = |words: &[&str]| words.iter().map(OsString::from).collect::<Vec<_>>();
  let names = ["file share", "version"];

  assert_eq!(
    joined(line(&["cli", "file", "share", "--days", "1"]), &names),
    line(&["cli", "file share", "--days", "1"])
  );
  assert_eq!(joined(line(&["cli", "version", "share"]), &names), line(&["cli", "version", "share"]));
  assert_eq!(joined(line(&["cli", "file"]), &names), line(&["cli", "file"]));
}

fn plain(id: &'static str) -> Operation {
  Operation {
    id,
    tag: "tests",
    summary: "A route of the tests.",
    parameters: Vec::new(),
    body: None,
    answers: Vec::new(),
    codes: &[],
  }
}

/// Answers with the query it is called with and the address the server stands on.
struct Echo;

#[derive(Deserialize)]
struct Named {
  name: Option<String>,
}

impl Route for Echo {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/echo";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let asked: Named = call.query()?;
    let listen = call.standing().settings.server.listen.to_string();

    Ok(answer(StatusCode::OK, &json!({ "name": asked.name, "listen": listen })))
  }

  fn operation(&self, _: &mut SchemaGenerator) -> Operation {
    plain("echo")
  }
}

bmo_artifact::route!(Echo);

/// Answers with the JSON object it is sent, on the same path as `Echo` under another method.
struct Mirror;

impl Route for Mirror {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/echo";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let sent: serde_json::Map<String, Value> = call.json().await?;

    Ok(answer(StatusCode::CREATED, &sent))
  }

  fn operation(&self, _: &mut SchemaGenerator) -> Operation {
    plain("mirror")
  }
}

bmo_artifact::route!(Mirror);

/// Refuses every call.
struct Taken;

impl Route for Taken {
  const METHOD: Method = Method::PUT;
  const PATH: &'static str = "/names/{name}";

  async fn answer(&self, _call: Call) -> Result<Response, Refused> {
    Err(Refusal::conflict("the name is taken").into())
  }

  fn operation(&self, _: &mut SchemaGenerator) -> Operation {
    plain("taken")
  }
}

bmo_artifact::route!(Taken);

const TOKEN: &str = "plumbing-token";

async fn call_with(method: Method, uri: &str, body: &str, token: Option<&str>) -> (StatusCode, Value) {
  let environment = HashMap::from([("BMO_ARTIFACT_SERVER__TOKEN".to_owned(), TOKEN.to_owned())]);
  let settings = Settings::read(Path::new(ROOT), Some(environment)).expect("the settings read");
  let router = router(Standing::new(settings).expect("the server stands")).expect("the routes stand");
  let mut request = Request::builder().method(method).uri(uri);

  if let Some(token) = token {
    request = request.header("authorization", format!("Bearer {token}"));
  }

  let response = router
    .oneshot(request.body(Body::from(body.to_owned())).expect("a request"))
    .await
    .expect("the router answers");
  let status = response.status();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("the body reads");

  (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

async fn call(method: Method, uri: &str, body: &str) -> (StatusCode, Value) {
  call_with(method, uri, body, Some(TOKEN)).await
}

#[test]
fn no_route_is_registered_twice() {
  assert_eq!(route_faults(), Vec::<String>::new());
}

#[tokio::test]
async fn a_route_reads_its_query_and_its_standing() {
  let (status, body) = call(Method::GET, "/echo?name=first", "").await;

  assert_eq!(status, StatusCode::OK);
  assert_eq!(body["name"], "first");
  assert!(body["listen"].as_str().is_some_and(|listen| !listen.is_empty()));
}

#[tokio::test]
async fn two_methods_share_one_path() {
  let (status, body) = call(Method::POST, "/echo", r#"{"kept": [1, 2]}"#).await;

  assert_eq!(status, StatusCode::CREATED);
  assert_eq!(body, json!({ "kept": [1, 2] }));
}

#[tokio::test]
async fn a_body_not_of_the_routes_shape_is_a_bad_request() {
  let (status, body) = call(Method::POST, "/echo", "not json").await;

  assert_eq!(status, StatusCode::BAD_REQUEST);
  assert_eq!(body["code"], "bad-request");
  assert!(
    body["error"].as_str().is_some_and(|error| error.starts_with("the body does not read")),
    "{body}"
  );
}

#[tokio::test]
async fn a_refusal_carries_its_status_its_code_and_its_sentence_alone() {
  let (status, body) = call(Method::PUT, "/names/report", "").await;

  assert_eq!(status, StatusCode::CONFLICT);
  assert_eq!(body, json!({ "code": "conflict", "error": "the name is taken" }));
}

#[tokio::test]
async fn no_route_answers_an_unknown_path_or_method() {
  let (status, body) = call(Method::GET, "/nowhere", "").await;

  assert_eq!((status, body["code"].as_str()), (StatusCode::NOT_FOUND, Some("no-route")));

  let (status, body) = call(Method::DELETE, "/echo", "").await;

  assert_eq!((status, body["code"].as_str()), (StatusCode::NOT_FOUND, Some("no-route")));
}

#[tokio::test]
async fn a_call_without_the_token_or_with_another_is_unauthorized() {
  for token in [None, Some("another")] {
    let (status, body) = call_with(Method::GET, "/echo", "", token).await;

    assert_eq!((status, body["code"].as_str()), (StatusCode::UNAUTHORIZED, Some("unauthorized")));
  }
}
