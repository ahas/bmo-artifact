//! A refusal of the service's own code: a status and an English sentence naming why, which a command prints
//! and a route answers under the code its status stands for (`docs/api.md`, "Codes").

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

  /// The request is not the shape the route takes.
  pub fn bad(detail: impl Into<String>) -> Self {
    Self::new(StatusCode::BAD_REQUEST, detail)
  }

  /// Nothing stands at the key the path names.
  pub fn absent(detail: impl Into<String>) -> Self {
    Self::new(StatusCode::NOT_FOUND, detail)
  }

  /// The member does not take the change.
  pub fn conflict(detail: impl Into<String>) -> Self {
    Self::new(StatusCode::CONFLICT, detail)
  }

  /// The body or a file is over its ceiling.
  pub fn too_large(detail: impl Into<String>) -> Self {
    Self::new(StatusCode::PAYLOAD_TOO_LARGE, detail)
  }

  /// A unit this service called failed.
  pub fn upstream(detail: impl Into<String>) -> Self {
    Self::new(StatusCode::BAD_GATEWAY, detail)
  }

  /// The service itself failed.
  pub fn failed(detail: impl Into<String>) -> Self {
    Self::new(StatusCode::INTERNAL_SERVER_ERROR, detail)
  }

  /// The code a caller matches this refusal on, as bmo-chat's conventions give each status.
  pub fn code(&self) -> &'static str {
    match self.status {
      StatusCode::BAD_REQUEST => "bad-request",
      StatusCode::UNAUTHORIZED => "unauthorized",
      StatusCode::FORBIDDEN => "forbidden",
      StatusCode::NOT_FOUND => "not-found",
      StatusCode::CONFLICT => "conflict",
      StatusCode::GONE => "gone",
      StatusCode::PAYLOAD_TOO_LARGE => "too-large",
      StatusCode::BAD_GATEWAY => "upstream-failed",
      StatusCode::SERVICE_UNAVAILABLE => "not-ready",
      _ => "failed",
    }
  }
}

impl From<DbErr> for Refusal {
  fn from(error: DbErr) -> Self {
    Fault::from(error).into()
  }
}

impl From<Fault> for Refusal {
  fn from(fault: Fault) -> Self {
    Self::failed(fault.detail)
  }
}

impl From<Refusal> for Fault {
  fn from(refusal: Refusal) -> Self {
    Self::new(refusal.detail)
  }
}

impl From<Refusal> for Refused {
  fn from(refusal: Refusal) -> Self {
    Self::new(refusal.status, refusal.code(), refusal.detail)
  }
}
