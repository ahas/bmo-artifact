//! The reads (`docs/api.md`, "Reads"): a read of one document kept for the call's reader, and the documents most
//! read in the last seven days.

use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use schemars::generate::SchemaGenerator;

use super::readers::{Reader, reached};
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, Standing, answer};
use crate::plumbing::openapi::{Operation, id_parameter, json_answer, query};
use crate::store::{page_limit, reads};
use crate::wire::{DocumentReadsPage, DocumentReadsQuery};

/// Who a read is kept for: an account by its key, or a browser reading through a share link by the token it keeps
/// for itself. The two never meet, so an account key never counts as a browser's token.
pub enum ReadBy<'a> {
  Account(&'a str),
  Visitor(&'a str),
}

impl ReadBy<'_> {
  fn key(&self) -> String {
    match self {
      Self::Account(key) => format!("account:{key}"),
      Self::Visitor(token) => format!("visitor:{token}"),
    }
  }
}

/// A read of one document kept for one reader.
pub async fn keep_read(standing: &Standing, id: i64, by: ReadBy<'_>) -> Result<(), Refused> {
  reads::keep(standing.db().await?, id, &by.key()).await?;

  Ok(())
}

/// Whether a visitor's token is one a browser makes: 8 to 64 letters, digits, `-` and `_`.
pub fn visitor_shaped(token: &str) -> bool {
  (8..=64).contains(&token.len())
    && token.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

pub struct KeepRead;

impl Route for KeepRead {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/documents/{documentId}/reads";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = reached(&mut call).await?;
    let reader = Reader::of(&call);

    keep_read(call.standing(), id, ReadBy::Account(reader.owner().unwrap_or_default())).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "createDocumentsReadsByDocumentId",
      tag: "documents",
      summary: "Keep a read of the document for the call's reader, once a day.",
      parameters: vec![id_parameter("documentId")],
      body: None,
      answers: vec![(204, None)],
      codes: &["unauthorized", "bad-request", "not-found", "upstream-failed"],
    }
  }
}

crate::route!(KeepRead);

pub struct ListDocumentReads;

impl Route for ListDocumentReads {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/document-reads";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let asked: DocumentReadsQuery = call.query()?;
    let limit = page_limit(asked.limit).ok_or_else(|| Refused::bad("limit is a whole number from 1"))?;
    let listing = Reader::of(&call).listing(call.standing()).await?;
    let account = listing.as_ref().map(|listing| listing.account());
    let (items, more) = reads::most_read(call.standing().db().await?, limit, account.as_ref()).await?;

    Ok(answer(StatusCode::OK, &DocumentReadsPage { items, more }))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listDocumentReads",
      tag: "documents",
      summary: "The documents most read in the last seven days, the most first.",
      parameters: query::<DocumentReadsQuery>(generator),
      body: None,
      answers: vec![(200, json_answer::<DocumentReadsPage>(generator))],
      codes: &["unauthorized", "bad-request", "upstream-failed"],
    }
  }
}

crate::route!(ListDocumentReads);
