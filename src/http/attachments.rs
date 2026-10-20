//! A document's attachments (`docs/api.md`, "Attachments"): listed, attached, and each one's bytes sent.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use schemars::generate::SchemaGenerator;

use super::documents::document;
use super::files::{Asking, KeptFile, PRIVATE_DAY, inside_root, send};
use crate::Refusal;
use crate::documents::keeper::{Handed, keep};
use crate::events::EventKind;
use crate::plumbing::http::{Call, Refused, Route, answer};
use crate::plumbing::openapi::{
  Operation, bytes_answer, header, id_parameter, json_answer, json_body, query,
};
use crate::store::{attachments, page_limit};
use crate::wire::{Attachment, AttachmentInput, AttachmentPage, ContentQuery, PageQuery};

pub struct ListAttachments;

impl Route for ListAttachments {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}/attachments";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = call.path_id("documentId").await?;
    let asked: PageQuery = call.query()?;
    let limit = page_limit(asked.limit).ok_or_else(|| Refused::bad("limit is a whole number from 1"))?;

    document(call.standing(), id).await?;

    let (kept, more) = attachments::list(call.standing().db().await?, id, asked.after, limit).await?;

    Ok(answer(
      StatusCode::OK,
      &AttachmentPage {
        items: kept.into_iter().map(|kept| kept.attachment).collect(),
        more,
      },
    ))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listDocumentsAttachmentsByDocumentId",
      tag: "documents",
      summary: "A document's attachments.",
      parameters: [vec![id_parameter("documentId")], query::<PageQuery>(generator)].concat(),
      body: None,
      answers: vec![(200, json_answer::<AttachmentPage>(generator))],
      codes: &["unauthorized", "bad-request", "not-found"],
    }
  }
}

crate::route!(ListAttachments);

pub struct Attach;

impl Route for Attach {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/documents/{documentId}/attachments";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = call.path_id("documentId").await?;
    let input: AttachmentInput = call.json().await?;
    let standing = call.standing();
    let owner = document(standing, id).await?.document;
    let bytes = STANDARD
      .decode(input.content.trim())
      .map_err(|why| Refused::bad(format!("content is not base64: {why}")))?;
    let db = standing.db().await?;
    let kept = keep(
      db,
      &standing.settings.files.root,
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
      codes: &["unauthorized", "bad-request", "not-found", "too-large"],
    }
  }
}

crate::route!(Attach);

pub struct ReadAttachment;

impl Route for ReadAttachment {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}/attachments/{attachmentId}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = call.path_id("documentId").await?;
    let attachment = call.path_id("attachmentId").await?;
    let asking = Asking::of(&call)?;
    let standing = call.shared();
    let kept = attachments::read(standing.db().await?, id, attachment)
      .await?
      .ok_or_else(|| Refusal::absent(format!("document {id} keeps no attachment {attachment}")))?;
    if !inside_root(&standing.settings.files.root, &kept.path) {
      return Err(
        Refusal::absent(format!("document {id} keeps no attachment {attachment} on the disk")).into(),
      );
    }

    let file = KeptFile {
      path: kept.path,
      media_type: kept.attachment.media_type,
      name: kept.attachment.name,
    };

    send(&asking, &file, PRIVATE_DAY).await
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
      codes: &["unauthorized", "bad-request", "not-found"],
    }
  }
}

crate::route!(ReadAttachment);
