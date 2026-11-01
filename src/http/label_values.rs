//! `GET /label-values` (`docs/api.md`, "Labels"): the distinct values one label holds across the listed
//! documents, the most carried first.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;

use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, answer};
use crate::plumbing::openapi::{Operation, json_answer, query};
use crate::store::{documents, page_limit};
use crate::wire::{LabelValuePage, LabelValuesQuery};

pub struct ListLabelValues;

impl Route for ListLabelValues {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/label-values";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let asked: LabelValuesQuery = call.query()?;
    let limit = page_limit(asked.limit).ok_or_else(|| Refused::bad("limit is a whole number from 1"))?;
    let key = asked.key.trim();

    if key.is_empty() {
      return Err(Refused::bad("key names the label whose values are asked"));
    }

    let (items, more) = documents::label_values(call.standing().db().await?, key, limit).await?;

    Ok(answer(StatusCode::OK, &LabelValuePage { items, more }))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listLabelValues",
      tag: "documents",
      summary: "The values one label holds across the documents, the most carried first.",
      parameters: query::<LabelValuesQuery>(generator),
      body: None,
      answers: vec![(200, json_answer::<LabelValuePage>(generator))],
      codes: &["unauthorized", "bad-request"],
    }
  }
}

crate::route!(ListLabelValues);
