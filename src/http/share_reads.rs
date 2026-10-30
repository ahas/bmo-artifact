//! The share reads (`docs/api.md`, "Share links"): what a live link opens of its one document, each read
//! carrying the token and answering what the route under `/documents/{documentId}` it stands for answers for
//! that document. The token is the whole of what each is held to, so the console's token reaches them with
//! no role and no owner key (bmo-ui's `docs/console-bundles.md`, "Share pages").

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;

use super::attachments::{attachment_bytes, attachment_page};
use super::documents::{content, document, picture};
use super::files::{Asking, Opened, Reach, any_origin};
use super::references::{cited_in, reference, reference_image, reference_page};
use super::shares::{shared_document, token_parameter};
use crate::kinds;
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, answer};
use crate::plumbing::openapi::{Operation, bytes_answer, header, id_parameter, json_answer, query};
use crate::wire::{
  AttachmentPage, ContentQuery, Document, DocumentKind, PageQuery, Reference, ReferencePage,
};

/// Every share read's path, as the manifest names them under `shareReads`, `GET /shares/{token}` among them,
/// and the reading's attachment read and the player's script, which a hosted page asks for with no session.
pub const SHARE_READS: [&str; 12] = [
  "/shares/{token}",
  "/shares/{token}/document",
  "/shares/{token}/kind",
  "/shares/{token}/content",
  "/shares/{token}/picture",
  "/shares/{token}/attachments",
  "/shares/{token}/attachments/{attachmentId}",
  "/shares/{token}/references",
  "/shares/{token}/references/{referenceId}",
  "/shares/{token}/references/{referenceId}/image",
  "/readings/{token}/attachments/{attachmentId}",
  "/player/{token}",
];

// The document the call's token opens.
async fn opened(call: &mut Call) -> Result<i64, Refused> {
  let token = call.path_text("token").await?;

  shared_document(call.standing(), &token).await
}

const CODES: &[&str] = &["unauthorized", "bad-request", "not-found", "gone"];

pub struct ReadSharedDocument;

impl Route for ReadSharedDocument {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/shares/{token}/document";
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = opened(&mut call).await?;

    Ok(answer(StatusCode::OK, &document(call.standing(), id).await?.document))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listSharesDocumentByToken",
      tag: "shares",
      summary: "The document a live link opens.",
      parameters: vec![token_parameter()],
      body: None,
      answers: vec![(200, json_answer::<Document>(generator))],
      codes: CODES,
    }
  }
}

crate::route!(ReadSharedDocument);

pub struct ReadSharedKind;

impl Route for ReadSharedKind {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/shares/{token}/kind";
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = opened(&mut call).await?;
    let row = document(call.standing(), id).await?;

    Ok(answer(StatusCode::OK, &kinds::kind(Some(&row.document.kind)).wire()))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listSharesKindByToken",
      tag: "shares",
      summary: "The kind of the document a live link opens.",
      parameters: vec![token_parameter()],
      body: None,
      answers: vec![(200, json_answer::<DocumentKind>(generator))],
      codes: CODES,
    }
  }
}

crate::route!(ReadSharedKind);

pub struct ReadSharedContent;

impl Route for ReadSharedContent {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/shares/{token}/content";
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = opened(&mut call).await?;
    let asking = Asking::of(&call)?;
    let reach = Reach::new(Opened::Share(call.path_text("token").await?), Self::PATH);

    content(call.standing(), &asking, id, &reach).await
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listSharesContentByToken",
      tag: "shares",
      summary: "The file of the document a live link opens.",
      parameters: [vec![token_parameter()], query::<ContentQuery>(generator)].concat(),
      body: None,
      answers: vec![(200, bytes_answer())],
      codes: CODES,
    }
  }
}

crate::route!(ReadSharedContent);

pub struct ReadSharedPicture;

impl Route for ReadSharedPicture {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/shares/{token}/picture";
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = opened(&mut call).await?;

    picture(call.standing(), id).await
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listSharesPictureByToken",
      tag: "shares",
      summary: "The cover picture of the document a live link opens.",
      parameters: vec![token_parameter()],
      body: None,
      answers: vec![(200, bytes_answer())],
      codes: CODES,
    }
  }
}

crate::route!(ReadSharedPicture);

pub struct ListSharedAttachments;

impl Route for ListSharedAttachments {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/shares/{token}/attachments";
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = opened(&mut call).await?;
    let asked: PageQuery = call.query()?;

    attachment_page(call.standing(), id, &asked).await
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listSharesAttachmentsByToken",
      tag: "shares",
      summary: "The attachments of the document a live link opens.",
      parameters: [vec![token_parameter()], query::<PageQuery>(generator)].concat(),
      body: None,
      answers: vec![(200, json_answer::<AttachmentPage>(generator))],
      codes: CODES,
    }
  }
}

crate::route!(ListSharedAttachments);

pub struct ReadSharedAttachment;

impl Route for ReadSharedAttachment {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/shares/{token}/attachments/{attachmentId}";
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = opened(&mut call).await?;
    let attachment = call.path_id("attachmentId").await?;
    let asking = Asking::of(&call)?;
    let reach = Reach::new(Opened::Share(call.path_text("token").await?), Self::PATH);

    Ok(any_origin(attachment_bytes(call.standing(), &asking, id, attachment, &reach).await?))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readSharesAttachmentsByTokenAttachmentId",
      tag: "shares",
      summary: "One attachment's bytes of the document a live link opens, with Range.",
      parameters: [
        vec![token_parameter(), id_parameter("attachmentId")],
        query::<ContentQuery>(generator),
        vec![header("Range", "One byte range of the file.")],
      ]
      .concat(),
      body: None,
      answers: vec![(200, bytes_answer()), (206, bytes_answer())],
      codes: CODES,
    }
  }
}

crate::route!(ReadSharedAttachment);

pub struct ListSharedReferences;

impl Route for ListSharedReferences {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/shares/{token}/references";
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = opened(&mut call).await?;
    let asked: PageQuery = call.query()?;

    reference_page(call.standing(), id, &asked).await
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listSharesReferencesByToken",
      tag: "shares",
      summary: "The pages the document a live link opens refers to, with what is kept of their previews.",
      parameters: [vec![token_parameter()], query::<PageQuery>(generator)].concat(),
      body: None,
      answers: vec![(200, json_answer::<ReferencePage>(generator))],
      codes: CODES,
    }
  }
}

crate::route!(ListSharedReferences);

pub struct ReadSharedReference;

impl Route for ReadSharedReference {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/shares/{token}/references/{referenceId}";
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = opened(&mut call).await?;
    let asked = call.path_id("referenceId").await?;
    let cited = cited_in(call.standing(), id, asked).await?;

    reference(call.standing(), cited).await
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readSharesReferencesByTokenReferenceId",
      tag: "shares",
      summary: "A page the document a live link opens refers to, with its preview.",
      parameters: vec![token_parameter(), id_parameter("referenceId")],
      body: None,
      answers: vec![(200, json_answer::<Reference>(generator))],
      codes: CODES,
    }
  }
}

crate::route!(ReadSharedReference);

pub struct ReadSharedReferenceImage;

impl Route for ReadSharedReferenceImage {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/shares/{token}/references/{referenceId}/image";
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = opened(&mut call).await?;
    let asked = call.path_id("referenceId").await?;
    let cited = cited_in(call.standing(), id, asked).await?;

    reference_image(call.standing(), cited).await
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listSharesReferencesImageByTokenReferenceId",
      tag: "shares",
      summary: "The preview picture of a page the document a live link opens refers to.",
      parameters: vec![token_parameter(), id_parameter("referenceId")],
      body: None,
      answers: vec![(200, bytes_answer())],
      codes: CODES,
    }
  }
}

crate::route!(ReadSharedReferenceImage);
