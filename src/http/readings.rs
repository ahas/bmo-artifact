//! The reading routes (`docs/artifact.md`, "Readings"; `docs/api.md`, "Readings"): the addresses a page the
//! console's own reader opens loads its sounds from, opened by a reading token, and the player's script those
//! pages and a share link's load.

use axum::body::Body;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::Response;
use schemars::generate::SchemaGenerator;
use serde_json::json;

use super::attachments::attachment_bytes;
use super::files::{Asking, Opened, Reach, any_origin};
use crate::Refusal;
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route};
use crate::plumbing::openapi::{
  Operation, bytes_answer, header as header_parameter, id_parameter, query, text_parameter,
};
use crate::readings::now_seconds;
use crate::wire::ContentQuery;

// What the player's script is kept for by whoever loaded it: a year, since its name changes with its bytes.
const KEPT: &str = "public, max-age=31536000, immutable";

fn token_parameter() -> serde_json::Value {
  text_parameter("token", "The reading token.")
}

pub struct ReadReadingAttachment;

impl Route for ReadReadingAttachment {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/readings/{token}/attachments/{attachmentId}";
  // A page the console's reader opens asks for it with no session, from a sandbox of its own origin.
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let token = call.path_text("token").await?;
    let attachment = call.path_id("attachmentId").await?;
    let asking = Asking::of(&call)?;
    let id = call
      .standing()
      .readings
      .opened(&token, now_seconds())
      .ok_or_else(|| Refused::from(Refusal::absent("the reading token opens nothing")))?;
    let reach = Reach::new(Opened::Reading(token), Self::PATH);

    Ok(any_origin(attachment_bytes(call.standing(), &asking, id, attachment, &reach).await?))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readReadingsAttachmentsByTokenAttachmentId",
      tag: "readings",
      summary: "One attachment's bytes of the document a reading opens, with Range.",
      parameters: [
        vec![token_parameter(), id_parameter("attachmentId")],
        query::<ContentQuery>(generator),
        vec![header_parameter("Range", "One byte range of the file.")],
      ]
      .concat(),
      body: None,
      answers: vec![(200, bytes_answer()), (206, bytes_answer())],
      codes: &["unauthorized", "bad-request", "not-found"],
    }
  }
}

crate::route!(ReadReadingAttachment);

pub struct ReadPlayer;

impl Route for ReadPlayer {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/player/{token}";
  // A hosted page loads it with no session, and it carries no reader's data.
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let name = call.path_text("token").await?;
    let file = call.standing().console.player(&name).ok_or_else(|| {
      Refused::from(Refusal::absent(format!("the console bundle has no player named {name}")))
    })?;
    let mut response = Response::new(Body::from(file.bytes.clone()));
    let headers = response.headers_mut();

    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(file.media_type));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(KEPT));
    *response.status_mut() = StatusCode::OK;

    Ok(response)
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readPlayer",
      tag: "readings",
      summary: "bmo-ui's audio player script, named by the digest of its bytes.",
      parameters: vec![text_parameter(
        "token",
        "The script's file name, carrying the digest of its bytes.",
      )],
      body: None,
      answers: vec![(200, Some(json!({ "text/javascript": { "schema": { "type": "string" } } })))],
      codes: &["unauthorized", "not-found"],
    }
  }
}

crate::route!(ReadPlayer);
