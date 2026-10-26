//! `GET /console-manifest`, `GET /console/words` and `GET /console/{file}` (`docs/api.md`, "The console"):
//! the console bundle's manifest, its words, and its module and stylesheet at paths that follow their bytes.

use axum::body::Body;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::Response;
use schemars::generate::SchemaGenerator;
use serde_json::json;

use super::files::NO_STORE;
use crate::Refusal;
use crate::console::{FILES_PATH, MANIFEST_ROUTE, WORDS_PATH};
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, answer};
use crate::plumbing::openapi::{Operation, json_answer, text_parameter};
use crate::wire::ConsoleManifest;

// A module or a stylesheet never changes under its path, so whoever loaded it keeps it.
const KEPT: &str = "private, max-age=31536000, immutable";

fn unbuilt() -> Refused {
  Refusal::absent("the console bundle is not built").into()
}

fn bytes(media_type: &str, cache: &'static str, body: Vec<u8>) -> Result<Response, Refused> {
  let mut response = Response::new(Body::from(body));
  let headers = response.headers_mut();

  headers.insert(
    header::CONTENT_TYPE,
    HeaderValue::from_str(media_type).map_err(|why| Refused::from(Refusal::failed(why.to_string())))?,
  );
  headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));

  Ok(response)
}

pub struct ReadConsoleManifest;

impl Route for ReadConsoleManifest {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = MANIFEST_ROUTE;
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let manifest = call.standing().console.manifest().ok_or_else(unbuilt)?;
    let mut answered = answer(StatusCode::OK, &manifest);

    answered.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static(NO_STORE));

    Ok(answered)
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readConsoleManifest",
      tag: "console",
      summary: "The console bundle's manifest.",
      parameters: Vec::new(),
      body: None,
      answers: vec![(200, json_answer::<ConsoleManifest>(generator))],
      codes: &["unauthorized", "not-found"],
    }
  }
}

crate::route!(ReadConsoleManifest);

pub struct ReadConsoleWords;

impl Route for ReadConsoleWords {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = WORDS_PATH;
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let words = call.standing().console.words().ok_or_else(unbuilt)?.to_vec();

    bytes("application/json", NO_STORE, words)
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readConsoleWords",
      tag: "console",
      summary: "The console bundle's words: each locale's under its code.",
      parameters: Vec::new(),
      body: None,
      answers: vec![(
        200,
        Some(json!({ "application/json": { "schema": {
          "type": "object",
          "additionalProperties": { "type": "object" },
        } } })),
      )],
      codes: &["unauthorized", "not-found"],
    }
  }
}

crate::route!(ReadConsoleWords);

pub struct ReadConsoleFile;

impl Route for ReadConsoleFile {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/console/{file}";
  // A reader with no sign-in loads them to open a share page, and they carry no reader's data (bmo-ui's
  // `docs/console-bundles.md`, "Share pages").
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let name = call.path_text("file").await?;
    let file = call.standing().console.file(&name).ok_or_else(|| {
      Refused::from(Refusal::absent(format!("the console bundle has no file {FILES_PATH}/{name}")))
    })?;

    bytes(file.media_type, KEPT, file.bytes.clone())
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readConsoleFile",
      tag: "console",
      summary: "The console bundle's module or stylesheet, at the path the manifest names.",
      parameters: vec![text_parameter("file", "The file's name, carrying the digest of its bytes.")],
      body: None,
      answers: vec![(
        200,
        Some(json!({
          "text/javascript": { "schema": { "type": "string" } },
          "text/css": { "schema": { "type": "string" } },
        })),
      )],
      codes: &["unauthorized", "not-found"],
    }
  }
}

crate::route!(ReadConsoleFile);
