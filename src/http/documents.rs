//! The documents (`docs/api.md`, "Documents"): the list with its filters, a document filed, read,
//! rewritten and let go of, its file and its cover picture.

use axum::body::Body;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use schemars::generate::SchemaGenerator;
use serde_json::json;

use super::files::{Asking, KeptFile, NO_STORE, Opened, PRIVATE_DAY, Reach, inside_root, send};
use super::readers::{Reader, kept, reached};
use crate::Refusal;
use crate::documents::cover::{cover_card, cover_file};
use crate::documents::filing;
use crate::documents::keeper::document_folder;
use crate::events::EventKind;
use crate::kinds;
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, Standing, answer};
use crate::plumbing::openapi::{Operation, bytes_answer, id_parameter, json_answer, json_body, query};
use crate::store::{attachments, documents, page_limit};
use crate::wire::{ContentQuery, Document, DocumentChange, DocumentInput, DocumentPage, DocumentsQuery};

/// What a document that does not stand answers.
pub fn absent(id: i64) -> Refused {
  Refusal::absent(format!("there is no document {id}")).into()
}

/// One document as the API answers it, refused where it does not stand.
pub async fn document(standing: &Standing, id: i64) -> Result<documents::Row, Refused> {
  documents::read(standing.db().await?, id).await?.ok_or_else(|| absent(id))
}

pub struct ListDocuments;

impl Route for ListDocuments {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let asked: DocumentsQuery = call.query()?;
    let limit = page_limit(asked.limit).ok_or_else(|| Refused::bad("limit is a whole number from 1"))?;
    let listing = Reader::of(&call).listing(call.standing()).await?;
    let account = listing.as_ref().map(|listing| listing.account());
    let (items, more) =
      documents::list(call.standing().db().await?, &asked, limit, account.as_ref()).await?;

    Ok(answer(StatusCode::OK, &DocumentPage { items, more }))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listDocuments",
      tag: "documents",
      summary: "Documents by kind, scope, label and time; q searches their words.",
      parameters: query::<DocumentsQuery>(generator),
      body: None,
      answers: vec![(200, json_answer::<DocumentPage>(generator))],
      codes: &["unauthorized", "bad-request", "upstream-failed"],
    }
  }
}

crate::route!(ListDocuments);

pub struct FileDocument;

impl Route for FileDocument {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/documents";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let reader = Reader::of(&call);
    let input: DocumentInput = call.json().await?;
    let standing = call.standing();
    let db = standing.db().await?;
    let id = filing::file(db, &standing.settings.files.root, &input, reader.owner()).await?;
    let filed = document(standing, id).await?.document;

    standing
      .events
      .publish(db, EventKind::DocumentCreated, filed.scope.as_deref(), &filed)
      .await?;

    Ok(answer(StatusCode::CREATED, &filed))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "createDocuments",
      tag: "documents",
      summary: "File a document: a record, a learning book, an import or a page.",
      parameters: Vec::new(),
      body: json_body::<DocumentInput>(generator),
      answers: vec![(201, json_answer::<Document>(generator))],
      codes: &["unauthorized", "bad-request"],
    }
  }
}

crate::route!(FileDocument);

pub struct ReadDocument;

impl Route for ReadDocument {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = call.path_id("documentId").await?;
    let read = Reader::of(&call).document(call.standing(), id).await?;

    Ok(answer(StatusCode::OK, &read.document))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readDocumentsByDocumentId",
      tag: "documents",
      summary: "One document and its head.",
      parameters: vec![id_parameter("documentId")],
      body: None,
      answers: vec![(200, json_answer::<Document>(generator))],
      codes: &["unauthorized", "bad-request", "not-found", "upstream-failed"],
    }
  }
}

crate::route!(ReadDocument);

pub struct ChangeDocument;

impl Route for ChangeDocument {
  const METHOD: Method = Method::PATCH;
  const PATH: &'static str = "/documents/{documentId}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = kept(&mut call).await?;
    let change: DocumentChange = call.json().await?;
    let standing = call.standing();
    let db = standing.db().await?;

    filing::rewrite(db, &standing.settings.files.root, id, &change).await?;

    let changed = document(standing, id).await?.document;

    standing
      .events
      .publish(db, EventKind::DocumentChanged, changed.scope.as_deref(), &changed)
      .await?;

    Ok(answer(StatusCode::OK, &changed))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "changeDocumentsByDocumentId",
      tag: "documents",
      summary: "Rewrite a document under its number; its share links stay.",
      parameters: vec![id_parameter("documentId")],
      body: json_body::<DocumentChange>(generator),
      answers: vec![(200, json_answer::<Document>(generator))],
      codes: &["unauthorized", "bad-request", "forbidden", "not-found", "conflict", "upstream-failed"],
    }
  }
}

crate::route!(ChangeDocument);

/// One document let go of with its files, attachments, references and share link, each link a
/// `share.deleted` and the document a `document.deleted` under the scope it was held in.
pub async fn let_go(standing: &Standing, id: i64, scope: Option<&str>) -> Result<(), Refused> {
  let db = standing.db().await?;
  let tokens = documents::delete(db, id).await?;
  let _ = std::fs::remove_dir_all(document_folder(&standing.settings.files.root, id));

  for token in tokens {
    standing
      .events
      .publish(db, EventKind::ShareDeleted, scope, &json!({ "token": token }))
      .await?;
  }

  standing
    .events
    .publish(db, EventKind::DocumentDeleted, scope, &json!({ "id": id }))
    .await?;

  Ok(())
}

pub struct ReleaseDocument;

impl Route for ReleaseDocument {
  const METHOD: Method = Method::DELETE;
  const PATH: &'static str = "/documents/{documentId}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = kept(&mut call).await?;
    let standing = call.standing();
    let held = document(standing, id).await?.document;

    let_go(standing, id, held.scope.as_deref()).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "deleteDocumentsByDocumentId",
      tag: "documents",
      summary: "Let go of a document with its files, attachments, references and share link.",
      parameters: vec![id_parameter("documentId")],
      body: None,
      answers: vec![(204, None)],
      codes: &["unauthorized", "bad-request", "forbidden", "not-found", "upstream-failed"],
    }
  }
}

crate::route!(ReleaseDocument);

/// The file a document reads from: for a page, the HTML it keeps; for any other kind, its markdown.
pub async fn document_file(standing: &Standing, row: &documents::Row) -> Result<KeptFile, Refused> {
  let id = row.document.id;
  let hosted = kinds::kind(Some(&row.document.kind)).hosted();
  let missing = || Refused::from(Refusal::absent(format!("document {id} keeps no file")));

  if hosted {
    // A page filed before its HTML was the document's own file keeps it among its attachments.
    if let Some(kept) = attachments::first_of_kind(standing.db().await?, id, "page").await?
      && inside_root(&standing.settings.files.root, &kept.path)
    {
      return Ok(KeptFile {
        path: kept.path,
        media_type: "text/html".to_owned(),
        name: kept.attachment.name,
      });
    }
  }

  let path = row.file.clone().ok_or_else(missing)?;
  if !inside_root(&standing.settings.files.root, &path) {
    return Err(missing());
  }

  Ok(if hosted {
    KeptFile {
      path,
      media_type: "text/html".to_owned(),
      name: format!("document-{id}.html"),
    }
  } else {
    KeptFile {
      path,
      media_type: "text/markdown; charset=utf-8".to_owned(),
      name: format!("document-{id}.md"),
    }
  })
}

/// A document's file as it is asked for: a page as a hosted page, any other kind as its markdown.
pub async fn content(
  standing: &Standing,
  asking: &Asking,
  id: i64,
  reach: &Reach,
) -> Result<Response, Refused> {
  let row = document(standing, id).await?;
  let file = document_file(standing, &row).await?;

  send(standing, asking, &file, NO_STORE, id, reach).await
}

/// A document's cover picture as the card a link's preview shows.
pub async fn picture(standing: &Standing, id: i64) -> Result<Response, Refused> {
  let row = document(standing, id).await?;
  let body = match (kinds::kind(Some(&row.document.kind)).hosted(), &row.file) {
    (false, Some(path)) => tokio::fs::read_to_string(path).await.ok(),
    _ => None,
  };
  let none = || Refused::from(Refusal::absent(format!("document {id} has no picture")));
  let path = cover_file(standing.db().await?, id, body.as_deref()).await?.ok_or_else(none)?;
  let bytes = tokio::fs::read(path).await.map_err(|_| none())?;
  let card = tokio::task::spawn_blocking(move || cover_card(&bytes))
    .await
    .map_err(|why| Refused::from(Refusal::failed(why.to_string())))?
    .ok_or_else(none)?;
  let mut response = Response::new(Body::from(card));

  response
    .headers_mut()
    .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
  response
    .headers_mut()
    .insert(header::CACHE_CONTROL, HeaderValue::from_static(PRIVATE_DAY));

  Ok(response)
}

pub struct ReadContent;

impl Route for ReadContent {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}/content";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = reached(&mut call).await?;
    let asking = Asking::of(&call)?;

    content(call.standing(), &asking, id, &Reach::new(Opened::Console, Self::PATH)).await
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listDocumentsContentByDocumentId",
      tag: "documents",
      summary: "The document's body as filed.",
      parameters: [vec![id_parameter("documentId")], query::<ContentQuery>(generator)].concat(),
      body: None,
      answers: vec![(200, bytes_answer())],
      codes: &["unauthorized", "bad-request", "not-found", "upstream-failed"],
    }
  }
}

crate::route!(ReadContent);

pub struct ReadPicture;

impl Route for ReadPicture {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}/picture";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = reached(&mut call).await?;

    picture(call.standing(), id).await
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listDocumentsPictureByDocumentId",
      tag: "documents",
      summary: "The document's cover picture.",
      parameters: vec![id_parameter("documentId")],
      body: None,
      answers: vec![(200, bytes_answer())],
      codes: &["unauthorized", "bad-request", "not-found", "upstream-failed"],
    }
  }
}

crate::route!(ReadPicture);
