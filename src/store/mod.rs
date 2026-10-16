//! The queries the carried code runs, refused in its own terms, and the clock its columns keep
//! (`docs/layout.md`, "Storage").

pub mod documents;
pub mod entity;
pub mod migration;
pub mod shares;

use sea_orm::{ConnectionTrait, QueryResult, Value};

use crate::plumbing::refusal::Refusal;
use crate::pool;

pub use crate::pool::statement;

/// The one row a statement answers, or none.
pub async fn one<C: ConnectionTrait>(
  db: &C,
  sql: &str,
  values: Vec<Value>,
) -> Result<Option<QueryResult>, Refusal> {
  Ok(pool::one(db, sql, values).await?)
}

/// Every row a statement answers.
pub async fn all<C: ConnectionTrait>(
  db: &C,
  sql: &str,
  values: Vec<Value>,
) -> Result<Vec<QueryResult>, Refusal> {
  Ok(pool::all(db, sql, values).await?)
}

/// A statement run for what it writes.
pub async fn run<C: ConnectionTrait>(db: &C, sql: &str, values: Vec<Value>) -> Result<u64, Refusal> {
  Ok(pool::run(db, sql, values).await?)
}

/// Now, in the epoch milliseconds the document tables' columns keep.
pub fn now_ms() -> i64 {
  chrono::Utc::now().timestamp_millis()
}
