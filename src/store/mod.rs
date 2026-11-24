//! The statements the service runs, refused in its own terms, and the clock its columns keep
//! (`docs/layout.md`, "Storage").

pub mod attachments;
pub mod documents;
pub mod events;
pub mod imports;
pub mod member_shares;
pub mod migration;
pub mod reach;
pub mod reads;
pub mod references;
pub mod removal;
pub mod scopes;
pub mod shares;
pub mod unit_shares;

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

/// Now, in epoch milliseconds.
pub fn now_ms() -> i64 {
  chrono::Utc::now().timestamp_millis()
}

/// How many members a page holds at most: `limit`, 30 where none is given, held to 200; none where it is
/// not a whole number from 1.
pub fn page_limit(limit: Option<i64>) -> Option<i64> {
  match limit {
    None => Some(30),
    Some(asked) if asked >= 1 => Some(asked.min(200)),
    Some(_) => None,
  }
}
