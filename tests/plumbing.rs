//! The plumbing: how a verb's answer ends the CLI, and how the router answers, refuses and reads a call
//! (`docs/layout.md`, "Plumbing"; `docs/api.md`, "CLI answers" and "Errors").

mod support;

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;

use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use axum::response::Response;
use bmo_artifact::plumbing::http::{Call, Refused, Route, Standing, answer, route_faults, router};
use bmo_artifact::plumbing::verb::{
  Answer, FAILURE, Finished, VerbError, finished, joined, verb_faults, verbs,
};
use bmo_artifact::settings::Settings;
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
fn no_verb_name_is_claimed_twice_and_each_says_what_it_does_in_its_own_words() {
  assert_eq!(verb_faults(), Vec::<String>::new());

  let filing = verbs().find(|verb| verb.name() == "archive doc").expect("archive doc is registered");
  let about = filing.command().get_about().map(ToString::to_string).unwrap_or_default();

  assert_eq!(filing.aliases(), ["archive-doc"]);
  assert!(about.starts_with("Writes a document into the archive"), "{about}");
}

#[test]
fn a_verbs_two_words_are_read_as_its_one_name_and_any_other_line_is_left_as_it_is() {
  let line = |words: &[&str]| words.iter().map(OsString::from).collect::<Vec<_>>();
  let names = ["archive doc", "version"];

  assert_eq!(
    joined(line(&["cli", "archive", "doc", "--title", "x"]), &names),
    line(&["cli", "archive doc", "--title", "x"])
  );
  assert_eq!(
    joined(line(&["cli", "archive-doc", "--title"]), &names),
    line(&["cli", "archive-doc", "--title"])
  );
  assert_eq!(joined(line(&["cli", "version", "doc"]), &names), line(&["cli", "version", "doc"]));
  assert_eq!(joined(line(&["cli", "archive"]), &names), line(&["cli", "archive"]));
}

/// Answers with the query parameter it is called with and the address the server stands on.
struct Echo;

impl Route for Echo {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/echo";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let listen = call.standing().settings.server.listen.to_string();

    Ok(answer(StatusCode::OK, json!({ "name": call.query("name"), "listen": listen })))
  }
}

bmo_artifact::route!(Echo);

/// Answers with the JSON body it is sent, on the same path as `Echo` under another method.
struct Mirror;

impl Route for Mirror {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/echo";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    Ok(answer(StatusCode::CREATED, call.body().await?))
  }
}

bmo_artifact::route!(Mirror);

/// Refuses every call.
struct Taken;

impl Route for Taken {
  const METHOD: Method = Method::PUT;
  const PATH: &'static str = "/names/{name}";

  async fn answer(&self, _call: Call) -> Result<Response, Refused> {
    Err(Refused::new(StatusCode::CONFLICT, "name_taken", "the name is taken"))
  }
}

bmo_artifact::route!(Taken);

async fn call(method: Method, uri: &str, body: &str) -> (StatusCode, Value) {
  let settings = Settings::read(Path::new(ROOT), Some(HashMap::new())).expect("the settings read");
  let router = router(Standing::new(settings).expect("the server stands")).expect("the routes stand");
  let request = Request::builder()
    .method(method)
    .uri(uri)
    .body(Body::from(body.to_owned()))
    .expect("a request");
  let response = router.oneshot(request).await.expect("the router answers");
  let status = response.status();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("the body reads");

  (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[test]
fn no_route_is_registered_twice() {
  assert_eq!(route_faults(), Vec::<String>::new());
}

#[tokio::test]
async fn a_route_reads_its_query_and_its_standing() {
  let (status, body) = call(Method::GET, "/echo?name=first&name=second", "").await;

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
async fn a_body_that_is_not_json_is_refused() {
  let (status, body) = call(Method::POST, "/echo", "not json").await;

  assert_eq!(status, StatusCode::BAD_REQUEST);
  assert_eq!(body, json!({ "error": "the body is not JSON", "code": "not_json" }));
}

#[tokio::test]
async fn a_refusal_carries_its_status_code_and_sentence() {
  let (status, body) = call(Method::PUT, "/names/report", "").await;

  assert_eq!(status, StatusCode::CONFLICT);
  assert_eq!(body, json!({ "error": "the name is taken", "code": "name_taken" }));
}

#[tokio::test]
async fn an_unknown_route_is_not_found() {
  let (status, _) = call(Method::GET, "/nowhere", "").await;

  assert_eq!(status, StatusCode::NOT_FOUND);
}

#[test]
fn an_unexpected_failure_is_refused_as_failed() {
  let refused = Refused::failed("the disk is full");

  assert_eq!((refused.status, refused.code.as_str()), (StatusCode::BAD_REQUEST, "failed"));
}
