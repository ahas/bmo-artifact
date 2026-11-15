//! The organisation units a document is shared with (`docs/api.md`, "Unit shares"): listed, shared with and taken
//! off by whoever keeps the document, each unit of its owner's company, opening the document to read to everyone
//! standing in it or under it.

use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use schemars::generate::SchemaGenerator;

use super::members::{other_company, owners_company};
use super::readers::kept;
use crate::Refusal;
use crate::now_ms;
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, Standing, answer};
use crate::plumbing::openapi::{Operation, id_parameter, json_answer, json_body, query};
use crate::store::{page_limit, unit_shares};
use crate::wire::{PageQuery, UnitShareInput, UnitSharePage};

/// The most units one call shares a document with.
const MOST_UNITS: usize = 200;

// Every unit asked standing in the company the document's owner stands in; any other is refused before anything is
// stored, and a document with no owner shares with no unit.
async fn of_owners_company(standing: &Standing, id: i64, units: &[i64]) -> Result<(), Refused> {
  let company = owners_company(standing, id)
    .await?
    .ok_or_else(|| Refusal::conflict(format!("document {id} has no owner, so it belongs to no company")))?;

  for unit in units {
    match standing.accounts.company_of_unit(*unit).await? {
      Some(standing_in) if standing_in == company => {}
      Some(_) => {
        return Err(other_company(format!(
          "unit {unit} stands in another company than the owner of document {id}"
        )));
      }
      None => return Err(Refusal::absent(format!("there is no unit {unit}")).into()),
    }
  }

  Ok(())
}

pub struct ListUnits;

impl Route for ListUnits {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}/units";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = kept(&mut call).await?;
    let asked: PageQuery = call.query()?;
    let limit = page_limit(asked.limit).ok_or_else(|| Refused::bad("limit is a whole number from 1"))?;
    let (items, more) = unit_shares::list(call.standing().db().await?, id, asked.after, limit).await?;

    Ok(answer(StatusCode::OK, &UnitSharePage { items, more }))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listDocumentsUnitsByDocumentId",
      tag: "units",
      summary: "The organisation units a document is shared with, in the order they were shared.",
      parameters: [vec![id_parameter("documentId")], query::<PageQuery>(generator)].concat(),
      body: None,
      answers: vec![(200, json_answer::<UnitSharePage>(generator))],
      codes: &["unauthorized", "bad-request", "not-found", "upstream-failed"],
    }
  }
}

crate::route!(ListUnits);

pub struct ShareWithUnits;

impl Route for ShareWithUnits {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/documents/{documentId}/units";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = kept(&mut call).await?;
    let asked: UnitShareInput = call.json().await?;

    if asked.units.is_empty() || asked.units.len() > MOST_UNITS {
      return Err(Refused::bad(format!("units names 1 to {MOST_UNITS} units")));
    }

    of_owners_company(call.standing(), id, &asked.units).await?;

    let items = unit_shares::add(call.standing().db().await?, id, &asked.units, now_ms()).await?;

    Ok(answer(StatusCode::OK, &UnitSharePage { items, more: false }))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "createDocumentsUnitsByDocumentId",
      tag: "units",
      summary: "Share a document with organisation units of its owner's company, to be read by everyone standing in \
                or under each; a unit already sharing it keeps its share.",
      parameters: vec![id_parameter("documentId")],
      body: json_body::<UnitShareInput>(generator),
      answers: vec![(200, json_answer::<UnitSharePage>(generator))],
      codes: &[
        "unauthorized",
        "bad-request",
        "not-found",
        "conflict",
        "other-company",
        "upstream-failed",
      ],
    }
  }
}

crate::route!(ShareWithUnits);

pub struct RemoveUnit;

impl Route for RemoveUnit {
  const METHOD: Method = Method::DELETE;
  const PATH: &'static str = "/documents/{documentId}/units/{unit}";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = kept(&mut call).await?;
    let unit = call.path_id("unit").await?;

    if !unit_shares::remove(call.standing().db().await?, id, unit).await? {
      return Err(Refusal::absent(format!("document {id} is not shared with unit {unit}")).into());
    }

    Ok(StatusCode::NO_CONTENT.into_response())
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "deleteDocumentsUnitsByDocumentIdUnit",
      tag: "units",
      summary: "Take a unit off a document; whoever read it only through that unit stops reading at once.",
      parameters: vec![id_parameter("documentId"), id_parameter("unit")],
      body: None,
      answers: vec![(204, None)],
      codes: &["unauthorized", "bad-request", "not-found", "upstream-failed"],
    }
  }
}

crate::route!(RemoveUnit);
