//! `GET /health` (`docs/api.md`, "Health"): ready while the database answers.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;

use crate::plumbing::http::{Call, Refused, Route, answer};
use crate::plumbing::openapi::{Operation, json_answer};
use crate::wire::Health;

pub struct ReadHealth;

impl Route for ReadHealth {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/health";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let reason = match call.standing().db().await {
      Ok(db) => crate::one(db, "SELECT 1 AS one", Vec::new())
        .await
        .err()
        .map(|why| format!("the database does not answer: {}", why.detail)),
      Err(why) => Some(format!("the database does not answer: {}", why.error)),
    };

    Ok(answer(
      StatusCode::OK,
      &Health {
        ready: reason.is_none(),
        reason,
        version: crate::VERSION.to_owned(),
      },
    ))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readHealth",
      tag: "health",
      summary: "Readiness and version.",
      parameters: Vec::new(),
      body: None,
      answers: vec![(200, json_answer::<Health>(generator))],
      codes: &["unauthorized"],
    }
  }
}

crate::route!(ReadHealth);
