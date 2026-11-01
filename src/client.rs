//! The service's own client (`docs/api.md`, "The client"): one method for each route, sending and reading
//! the types of `wire`. The command and the MCP server reach the service through it, and so may any other
//! Rust caller.

use std::time::Duration;

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::{Method, RequestBuilder, Response, header};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::settings::Settings;
use crate::wire::{
  Attachment, AttachmentInput, AttachmentPage, ConsoleManifest, ContentQuery, Document, DocumentChange,
  DocumentInput, DocumentKindPage, DocumentPage, DocumentsQuery, DryRunQuery, EventPage, EventsQuery,
  Health, Import, ImportInput, LabelValuePage, LabelValuesQuery, PageQuery, Publication, Reference,
  ReferencePage, ScopeHolding, Share, ShareInput, SharePage, SharesQuery, WireError,
};

// Everything but the unreserved characters is written percent-encoded in a path segment.
const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');

/// How long one call is waited for.
pub const CALL_WAIT: Duration = Duration::from_secs(120);

fn segment(value: &str) -> String {
  utf8_percent_encode(value, SEGMENT).to_string()
}

/// Why a call reached no answer of the route's shape.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ClientFailure {
  /// Nothing answered: the service is not where it was looked for, or did not answer in time.
  #[error("the service did not answer: {0}")]
  Unreached(String),
  /// The service refused, in its error shape where it answered in one.
  #[error("the service answered {status}: {}", answer.as_ref().map_or(body.as_str(), |answer| answer.error.as_str()))]
  Answered {
    status: u16,
    answer: Option<WireError>,
    body: String,
  },
  /// The service answered, but not in the route's shape.
  #[error("the service's answer could not be read: {0}")]
  Unread(String),
}

impl ClientFailure {
  /// The code the service refused with, where it refused in its error shape.
  pub fn code(&self) -> Option<&str> {
    match self {
      Self::Answered {
        answer: Some(answer),
        ..
      } => Some(&answer.code),
      _ => None,
    }
  }
}

/// Bytes the service answered with, and their type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Content {
  /// 200 for the whole, 206 for a range.
  pub status: u16,
  pub media_type: String,
  pub bytes: Vec<u8>,
}

/// The client of one service, at its address, presenting its token.
#[derive(Clone, Debug)]
pub struct ArtifactClient {
  address: String,
  token: String,
  http: reqwest::Client,
}

impl ArtifactClient {
  /// The client of the service at `address`, presenting `token`.
  pub fn new(address: &str, token: &str) -> Self {
    Self {
      address: address.trim_end_matches('/').to_owned(),
      token: token.to_owned(),
      http: reqwest::Client::builder().timeout(CALL_WAIT).build().unwrap_or_default(),
    }
  }

  /// The client of the service the settings name: at `address` where one is given, else at the settings'
  /// listening address, with the settings' token.
  pub fn of_settings(settings: &Settings, address: Option<&str>) -> Self {
    let listening = format!("http://{}", settings.server.listen);

    Self::new(address.unwrap_or(&listening), &settings.server.token)
  }

  fn request(&self, method: Method, path: &str) -> RequestBuilder {
    self.http.request(method, format!("{}{path}", self.address)).bearer_auth(&self.token)
  }

  async fn sent(request: RequestBuilder) -> Result<Response, ClientFailure> {
    let response = request.send().await.map_err(|why| ClientFailure::Unreached(why.to_string()))?;

    if response.status().is_success() {
      return Ok(response);
    }

    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();

    Err(ClientFailure::Answered {
      status,
      answer: serde_json::from_str(&body).ok(),
      body,
    })
  }

  async fn json<Answer: DeserializeOwned>(request: RequestBuilder) -> Result<Answer, ClientFailure> {
    Self::sent(request)
      .await?
      .json()
      .await
      .map_err(|why| ClientFailure::Unread(why.to_string()))
  }

  async fn bytes(request: RequestBuilder) -> Result<Content, ClientFailure> {
    let response = Self::sent(request).await?;
    let status = response.status().as_u16();
    let media_type = response
      .headers()
      .get(header::CONTENT_TYPE)
      .and_then(|value| value.to_str().ok())
      .unwrap_or("application/octet-stream")
      .to_owned();
    let bytes = response.bytes().await.map_err(|why| ClientFailure::Unread(why.to_string()))?;

    Ok(Content {
      status,
      media_type,
      bytes: bytes.to_vec(),
    })
  }

  fn get<Query: Serialize>(&self, path: &str, query: &Query) -> RequestBuilder {
    self.request(Method::GET, path).query(query)
  }

  /// `GET /health`.
  pub async fn health(&self) -> Result<Health, ClientFailure> {
    Self::json(self.request(Method::GET, "/health")).await
  }

  /// `GET /console-manifest`.
  pub async fn console_manifest(&self) -> Result<ConsoleManifest, ClientFailure> {
    Self::json(self.request(Method::GET, "/console-manifest")).await
  }

  /// `GET /console/words`.
  pub async fn console_words(&self) -> Result<serde_json::Value, ClientFailure> {
    Self::json(self.request(Method::GET, "/console/words")).await
  }

  /// `GET /console/{file}`.
  pub async fn console_file(&self, file: &str) -> Result<Content, ClientFailure> {
    Self::bytes(self.request(Method::GET, &format!("/console/{}", segment(file)))).await
  }

  /// `GET /events`, as a page.
  pub async fn events(&self, query: &EventsQuery) -> Result<EventPage, ClientFailure> {
    Self::json(self.get("/events", query).header(header::ACCEPT, "application/json")).await
  }

  /// `GET /documents`.
  pub async fn documents(&self, query: &DocumentsQuery) -> Result<DocumentPage, ClientFailure> {
    Self::json(self.get("/documents", query)).await
  }

  /// `POST /documents`.
  pub async fn file(&self, input: &DocumentInput) -> Result<Document, ClientFailure> {
    Self::json(self.request(Method::POST, "/documents").json(input)).await
  }

  /// `POST /publications`.
  pub async fn publish(&self, input: &DocumentInput) -> Result<Publication, ClientFailure> {
    Self::json(self.request(Method::POST, "/publications").json(input)).await
  }

  /// `GET /documents/{documentId}`.
  pub async fn document(&self, document_id: i64) -> Result<Document, ClientFailure> {
    Self::json(self.request(Method::GET, &format!("/documents/{document_id}"))).await
  }

  /// `PATCH /documents/{documentId}`.
  pub async fn rewrite(
    &self,
    document_id: i64,
    change: &DocumentChange,
  ) -> Result<Document, ClientFailure> {
    Self::json(self.request(Method::PATCH, &format!("/documents/{document_id}")).json(change)).await
  }

  /// `DELETE /documents/{documentId}`.
  pub async fn release_document(&self, document_id: i64) -> Result<(), ClientFailure> {
    Self::sent(self.request(Method::DELETE, &format!("/documents/{document_id}")))
      .await
      .map(drop)
  }

  /// `GET /documents/{documentId}/content`.
  pub async fn content(&self, document_id: i64, query: &ContentQuery) -> Result<Content, ClientFailure> {
    Self::bytes(self.get(&format!("/documents/{document_id}/content"), query)).await
  }

  /// `GET /documents/{documentId}/picture`.
  pub async fn picture(&self, document_id: i64) -> Result<Content, ClientFailure> {
    Self::bytes(self.request(Method::GET, &format!("/documents/{document_id}/picture"))).await
  }

  /// `GET /documents/{documentId}/attachments`.
  pub async fn attachments(
    &self,
    document_id: i64,
    query: &PageQuery,
  ) -> Result<AttachmentPage, ClientFailure> {
    Self::json(self.get(&format!("/documents/{document_id}/attachments"), query)).await
  }

  /// `POST /documents/{documentId}/attachments`.
  pub async fn attach(
    &self,
    document_id: i64,
    input: &AttachmentInput,
  ) -> Result<Attachment, ClientFailure> {
    Self::json(self.request(Method::POST, &format!("/documents/{document_id}/attachments")).json(input))
      .await
  }

  /// `GET /documents/{documentId}/attachments/{attachmentId}`, a range of it where `range` names one.
  pub async fn attachment(
    &self,
    document_id: i64,
    attachment_id: i64,
    query: &ContentQuery,
    range: Option<&str>,
  ) -> Result<Content, ClientFailure> {
    let request = self.get(&format!("/documents/{document_id}/attachments/{attachment_id}"), query);
    let request = match range {
      Some(range) => request.header(header::RANGE, range),
      None => request,
    };

    Self::bytes(request).await
  }

  /// `GET /documents/{documentId}/references`.
  pub async fn references(
    &self,
    document_id: i64,
    query: &PageQuery,
  ) -> Result<ReferencePage, ClientFailure> {
    Self::json(self.get(&format!("/documents/{document_id}/references"), query)).await
  }

  /// `GET /documents/{documentId}/references/{referenceId}`.
  pub async fn reference(&self, document_id: i64, reference_id: i64) -> Result<Reference, ClientFailure> {
    Self::json(self.request(Method::GET, &format!("/documents/{document_id}/references/{reference_id}")))
      .await
  }

  /// `GET /documents/{documentId}/references/{referenceId}/image`.
  pub async fn reference_image(
    &self,
    document_id: i64,
    reference_id: i64,
  ) -> Result<Content, ClientFailure> {
    Self::bytes(
      self.request(Method::GET, &format!("/documents/{document_id}/references/{reference_id}/image")),
    )
    .await
  }

  /// `GET /document-kinds`.
  pub async fn document_kinds(&self, query: &PageQuery) -> Result<DocumentKindPage, ClientFailure> {
    Self::json(self.get("/document-kinds", query)).await
  }

  /// `GET /label-values`.
  pub async fn label_values(&self, query: &LabelValuesQuery) -> Result<LabelValuePage, ClientFailure> {
    Self::json(self.get("/label-values", query)).await
  }

  /// `GET /shares`.
  pub async fn shares(&self, query: &SharesQuery) -> Result<SharePage, ClientFailure> {
    Self::json(self.get("/shares", query)).await
  }

  /// `POST /shares`.
  pub async fn share(&self, input: &ShareInput) -> Result<Share, ClientFailure> {
    Self::json(self.request(Method::POST, "/shares").json(input)).await
  }

  /// `GET /shares/{token}`.
  pub async fn share_link(&self, token: &str) -> Result<Share, ClientFailure> {
    Self::json(self.request(Method::GET, &format!("/shares/{}", segment(token)))).await
  }

  /// `DELETE /shares/{token}`.
  pub async fn unshare(&self, token: &str) -> Result<(), ClientFailure> {
    Self::sent(self.request(Method::DELETE, &format!("/shares/{}", segment(token))))
      .await
      .map(drop)
  }

  /// `POST /imports`.
  pub async fn import(&self, input: &ImportInput) -> Result<Import, ClientFailure> {
    Self::json(self.request(Method::POST, "/imports").json(input)).await
  }

  /// `GET /imports/{importId}`.
  pub async fn import_of(&self, import_id: i64) -> Result<Import, ClientFailure> {
    Self::json(self.request(Method::GET, &format!("/imports/{import_id}"))).await
  }

  /// `GET /scopes/{scope}`.
  pub async fn scope(&self, scope: &str) -> Result<ScopeHolding, ClientFailure> {
    Self::json(self.request(Method::GET, &format!("/scopes/{}", segment(scope)))).await
  }

  /// `DELETE /scopes/{scope}`.
  pub async fn release_scope(
    &self,
    scope: &str,
    query: &DryRunQuery,
  ) -> Result<ScopeHolding, ClientFailure> {
    Self::json(self.request(Method::DELETE, &format!("/scopes/{}", segment(scope))).query(query)).await
  }
}
