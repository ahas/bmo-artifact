//! A document's attachments (`docs/api.md`, "Attachments"): listed, attached, and each one's bytes sent.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use schemars::generate::SchemaGenerator;

use super::documents::{document, locate};
use super::files::{Asking, KeptFile, Opened, PRIVATE_DAY, Reach, send};
use super::readers::{kept, reached};
use crate::Refusal;
use crate::documents::keeper::{Handed, keep};
use crate::events::EventKind;
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, Standing, answer};
use crate::plumbing::openapi::{
  Operation, bytes_answer, header, id_parameter, json_answer, json_body, query,
};
use crate::store::{attachments, page_limit};
use crate::wire::{Attachment, AttachmentInput, AttachmentPage, ContentQuery, PageQuery};

/// A page of a document's attachments, in the order they were kept.
pub async fn attachment_page(standing: &Standing, id: i64, asked: &PageQuery) -> Result<Response, Refused> {
  let limit = page_limit(asked.limit).ok_or_else(|| Refused::bad("limit is a whole number from 1"))?;

  document(standing, id).await?;

  let (kept, more) = attachments::list(standing.db().await?, id, asked.after, limit).await?;

  Ok(answer(
    StatusCode::OK,
    &AttachmentPage {
      items: kept.into_iter().map(|kept| kept.attachment).collect(),
      more,
    },
  ))
}

/// One attachment's bytes as they are asked for; an attachment of another document is absent.
pub async fn attachment_bytes(
  standing: &Standing,
  asking: &Asking,
  id: i64,
  attachment: i64,
  reach: &Reach,
) -> Result<Response, Refused> {
  let row = document(standing, id).await?;
  let kept = attachments::read(standing.db().await?, id, attachment)
    .await?
    .ok_or_else(|| Refusal::absent(format!("document {id} keeps no attachment {attachment}")))?;
  let at = locate(standing, &row, &kept.path)
    .map_err(|_| Refusal::absent(format!("document {id} keeps no attachment {attachment} at its place")))?;

  let file = KeptFile {
    at,
    media_type: kept.attachment.media_type,
    name: kept.attachment.name,
  };

  send(standing, asking, &file, PRIVATE_DAY, id, reach).await
}

pub struct ListAttachments;

impl Route for ListAttachments {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}/attachments";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = reached(&mut call).await?;
    let asked: PageQuery = call.query()?;

    attachment_page(call.standing(), id, &asked).await
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listDocumentsAttachmentsByDocumentId",
      tag: "documents",
      summary: "A document's attachments.",
      parameters: [vec![id_parameter("documentId")], query::<PageQuery>(generator)].concat(),
      body: None,
      answers: vec![(200, json_answer::<AttachmentPage>(generator))],
      codes: &["unauthorized", "bad-request", "not-found", "upstream-failed"],
    }
  }
}

crate::route!(ListAttachments);

pub struct Attach;

impl Route for Attach {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/documents/{documentId}/attachments";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = kept(&mut call).await?;
    let input: AttachmentInput = call.json().await?;
    let standing = call.standing();
    let owner = document(standing, id).await?.document;
    let bytes = STANDARD
      .decode(input.content.trim())
      .map_err(|why| Refused::bad(format!("content is not base64: {why}")))?;
    let db = standing.db().await?;
    let kept = keep(
      db,
      &standing.places,
      id,
      Handed {
        name: &input.name,
        bytes,
        media_type: Some(&input.media_type),
        role: input.role.as_deref(),
        url: None,
      },
    )
    .await?;

    standing
      .events
      .publish(db, EventKind::AttachmentCreated, owner.scope.as_deref(), &kept)
      .await?;

    Ok(answer(StatusCode::CREATED, &kept))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "createDocumentsAttachmentsByDocumentId",
      tag: "documents",
      summary: "Attach a file to a document, a sound to be heard at its head among them.",
      parameters: vec![id_parameter("documentId")],
      body: json_body::<AttachmentInput>(generator),
      answers: vec![(201, json_answer::<Attachment>(generator))],
      codes: &["unauthorized", "bad-request", "not-found", "too-large", "upstream-failed"],
    }
  }
}

crate::route!(Attach);

pub struct ReadAttachment;

impl Route for ReadAttachment {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}/attachments/{attachmentId}";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = reached(&mut call).await?;
    let attachment = call.path_id("attachmentId").await?;
    let asking = Asking::of(&call)?;

    attachment_bytes(call.standing(), &asking, id, attachment, &Reach::new(Opened::Console, Self::PATH))
      .await
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readDocumentsAttachmentsByDocumentIdAttachmentId",
      tag: "documents",
      summary: "One attachment's bytes, with Range.",
      parameters: [
        vec![id_parameter("documentId"), id_parameter("attachmentId")],
        query::<ContentQuery>(generator),
        vec![header("Range", "One byte range of the file.")],
      ]
      .concat(),
      body: None,
      answers: vec![(200, bytes_answer()), (206, bytes_answer())],
      codes: &["unauthorized", "bad-request", "not-found", "upstream-failed"],
    }
  }
}

crate::route!(ReadAttachment);
