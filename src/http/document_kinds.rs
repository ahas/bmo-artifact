//! `GET /document-kinds` (`docs/api.md`, "Kinds"): the kinds a document may be filed as, each answering for
//! itself, in the catalogue's order.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;

use crate::kinds::KINDS;
use crate::plumbing::http::{Call, Refused, Route, answer};
use crate::plumbing::openapi::{Operation, json_answer, query};
use crate::store::page_limit;
use crate::wire::{DocumentKindPage, PageQuery};

pub struct ListDocumentKinds;

impl Route for ListDocumentKinds {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/document-kinds";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let asked: PageQuery = call.query()?;
    let limit = page_limit(asked.limit).ok_or_else(|| Refused::bad("limit is a whole number from 1"))?;
    // A kind's place in the catalogue is its cursor, counted from 1.
    let skip = usize::try_from(asked.after.unwrap_or(0).max(0)).unwrap_or(usize::MAX);
    let rest: Vec<_> = KINDS.iter().skip(skip).collect();
    let take = usize::try_from(limit).unwrap_or(usize::MAX);

    Ok(answer(
      StatusCode::OK,
      &DocumentKindPage {
        items: rest.iter().take(take).map(|kind| kind.wire()).collect(),
        more: rest.len() > take,
      },
    ))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listDocumentKinds",
      tag: "document-kinds",
      summary: "The kinds a document may be filed as.",
      parameters: query::<PageQuery>(generator),
      body: None,
      answers: vec![(200, json_answer::<DocumentKindPage>(generator))],
      codes: &["unauthorized", "bad-request"],
    }
  }
}

crate::route!(ListDocumentKinds);
