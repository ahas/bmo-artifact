//! A refusal of the carried code: a status and an English sentence naming why, which a verb prints and
//! a route answers (`docs/layout.md`, "Plumbing").

use axum::http::StatusCode;
use sea_orm::DbErr;

use crate::plumbing::http::Refused;
use crate::pool::Fault;

/// A refusal, said with its status and a sentence naming why.
#[derive(Debug, thiserror::Error)]
#[error("{detail}")]
pub struct Refusal {
  pub status: StatusCode,
  pub detail: String,
}

impl Refusal {
  pub fn new(status: StatusCode, detail: impl Into<String>) -> Self {
    Self {
      status,
      detail: detail.into(),
    }
  }

  pub fn bad(detail: impl Into<String>) -> Self {
    Self::new(StatusCode::BAD_REQUEST, detail)
  }

  pub fn forbidden(detail: impl Into<String>) -> Self {
    Self::new(StatusCode::FORBIDDEN, detail)
  }

  pub fn absent(detail: impl Into<String>) -> Self {
    Self::new(StatusCode::NOT_FOUND, detail)
  }
}

impl From<DbErr> for Refusal {
  fn from(error: DbErr) -> Self {
    Fault::from(error).into()
  }
}

impl From<Fault> for Refusal {
  fn from(fault: Fault) -> Self {
    Self::new(StatusCode::INTERNAL_SERVER_ERROR, fault.detail)
  }
}

impl From<Refusal> for Fault {
  fn from(refusal: Refusal) -> Self {
    Self::new(refusal.detail)
  }
}

/// A route answers a refusal with its status, coded by the status's own name: `not_found`.
impl From<Refusal> for Refused {
  fn from(refusal: Refusal) -> Self {
    let code = refusal
      .status
      .canonical_reason()
      .unwrap_or("failed")
      .to_ascii_lowercase()
      .replace([' ', '-'], "_");

    Self::new(refusal.status, code, refusal.detail)
  }
}
