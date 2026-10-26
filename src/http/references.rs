//! The pages a document refers to (`docs/api.md`, "References"): listed with what is kept of their
//! previews, one read with its preview, and its preview picture.

use axum::body::Body;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::Response;
use schemars::generate::SchemaGenerator;

use super::documents::document;
use super::files::PRIVATE_DAY;
use crate::Refusal;
use crate::documents::preview::PreviewCache;
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, Standing, answer};
use crate::plumbing::openapi::{Operation, bytes_answer, id_parameter, json_answer, query};
use crate::reader::Preview;
use crate::store::page_limit;
use crate::store::references::{self, Cited};
use crate::wire::{PageQuery, Reference, ReferencePage};

fn reference_of(cited: Cited, preview: Option<Preview>) -> Reference {
  let preview = preview.unwrap_or_default();

  Reference {
    id: cited.id,
    document_id: cited.document_id,
    title: cited.title.or(preview.title),
    preview: preview.description,
    url: cited.url,
  }
}

/// One reference a document holds; a reference of another document is absent.
pub async fn cited_in(standing: &Standing, id: i64, reference: i64) -> Result<Cited, Refused> {
  references::read(standing.db().await?, id, reference)
    .await?
    .ok_or_else(|| Refusal::absent(format!("document {id} holds no reference {reference}")).into())
}

async fn cited(call: &mut Call) -> Result<Cited, Refused> {
  let id = call.path_id("documentId").await?;
  let reference = call.path_id("referenceId").await?;

  cited_in(call.standing(), id, reference).await
}

/// A page of the pages a document refers to, with what is kept of their previews.
pub async fn reference_page(standing: &Standing, id: i64, asked: &PageQuery) -> Result<Response, Refused> {
  let limit = page_limit(asked.limit).ok_or_else(|| Refused::bad("limit is a whole number from 1"))?;

  document(standing, id).await?;

  let (cited, more) = references::list(standing.db().await?, id, asked.after, limit).await?;
  let previews: &PreviewCache = &standing.previews;

  Ok(answer(
    StatusCode::OK,
    &ReferencePage {
      items: cited
        .into_iter()
        .map(|cited| {
          let held = previews.held(&cited.url);

          reference_of(cited, held)
        })
        .collect(),
      more,
    },
  ))
}

/// One reference with its preview, read where none is kept fresh.
pub async fn reference(standing: &Standing, cited: Cited) -> Result<Response, Refused> {
  let preview = standing.previews.preview(&cited.url).await;

  Ok(answer(StatusCode::OK, &reference_of(cited, preview)))
}

/// The preview picture of a page a document refers to.
pub async fn reference_image(standing: &Standing, cited: Cited) -> Result<Response, Refused> {
  let previews = &standing.previews;
  let none =
    || Refused::from(Refusal::absent(format!("{} declares no picture that could be read", cited.url)));
  let address = previews.preview(&cited.url).await.and_then(|preview| preview.image).ok_or_else(none)?;
  let picture = previews.image(&address).await.ok_or_else(none)?;
  let mut response = Response::new(Body::from(picture.bytes));
  let headers = response.headers_mut();

  if let Ok(media_type) = HeaderValue::from_str(&picture.media_type) {
    headers.insert(header::CONTENT_TYPE, media_type);
  }

  headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(PRIVATE_DAY));
  headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
  headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("sandbox"));

  Ok(response)
}

pub struct ListReferences;

impl Route for ListReferences {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}/references";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = call.path_id("documentId").await?;
    let asked: PageQuery = call.query()?;

    reference_page(call.standing(), id, &asked).await
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listDocumentsReferencesByDocumentId",
      tag: "documents",
      summary: "The pages a document refers to, with what is kept of their previews.",
      parameters: [vec![id_parameter("documentId")], query::<PageQuery>(generator)].concat(),
      body: None,
      answers: vec![(200, json_answer::<ReferencePage>(generator))],
      codes: &["unauthorized", "bad-request", "not-found"],
    }
  }
}

crate::route!(ListReferences);

pub struct ReadReference;

impl Route for ReadReference {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}/references/{referenceId}";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let cited = cited(&mut call).await?;

    reference(call.standing(), cited).await
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readDocumentsReferencesByDocumentIdReferenceId",
      tag: "documents",
      summary: "A page a document refers to, with its preview.",
      parameters: vec![id_parameter("documentId"), id_parameter("referenceId")],
      body: None,
      answers: vec![(200, json_answer::<Reference>(generator))],
      codes: &["unauthorized", "bad-request", "not-found"],
    }
  }
}

crate::route!(ReadReference);

pub struct ReadReferenceImage;

impl Route for ReadReferenceImage {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}/references/{referenceId}/image";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let cited = cited(&mut call).await?;

    reference_image(call.standing(), cited).await
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listDocumentsReferencesImageByDocumentIdReferenceId",
      tag: "documents",
      summary: "A referred page's image.",
      parameters: vec![id_parameter("documentId"), id_parameter("referenceId")],
      body: None,
      answers: vec![(200, bytes_answer())],
      codes: &["unauthorized", "bad-request", "not-found"],
    }
  }
}

crate::route!(ReadReferenceImage);
