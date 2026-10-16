//! What this repository holds for a room, as bmo-receiver reads it before it removes one (`docs/api.md`,
//! "Room holding (HTTP)"). Each table naming a room declares its own entry beside the code that writes it,
//! through `bmo_artifact::room_holding!`; nothing here names a table.

use axum::extract::{FromRequestParts, Path};
use axum::http::{Method, StatusCode};
use axum::response::Response;
use sea_orm::DatabaseConnection;
use serde::Serialize;
use serde_json::json;

use crate::plumbing::http::{Call, Refused, Route, answer};
use crate::{Refusal, one};

pub use inventory;

/// One table holding a room's rows, which outlive the room.
#[derive(Debug)]
pub struct Kept {
  /// The table's name.
  pub table: &'static str,
  /// The column naming the room a row belongs to.
  pub column: &'static str,
  /// The English sentence saying why the rows outlive the room.
  pub why: &'static str,
}

inventory::collect!(Kept);

/// Declares a table holding a room's rows: `bmo_artifact::room_holding!(Kept { table: .., column: .., why: .. });`
#[macro_export]
macro_rules! room_holding {
  ($kept:expr) => {
    $crate::http::rooms::inventory::submit! { $kept }
  };
}

/// Every table declared to hold a room's rows.
pub fn kept() -> impl Iterator<Item = &'static Kept> {
  inventory::iter::<Kept>.into_iter()
}

/// One table's entry in the answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Holding {
  pub table: String,
  pub rows: i64,
  pub why: String,
}

/// What each declared table holds of one room, by table name, a table holding none of its rows included.
pub async fn holding(db: &DatabaseConnection, chat_id: i64) -> Result<Vec<Holding>, Refusal> {
  let mut tables: Vec<&Kept> = kept().collect();
  let mut held = Vec::new();

  tables.sort_by_key(|kept| kept.table);

  for kept in tables {
    let sql = format!(r#"SELECT COUNT(*) AS "rows" FROM "{}" WHERE "{}" = $1"#, kept.table, kept.column);
    let rows: i64 = match one(db, &sql, vec![chat_id.into()]).await? {
      Some(row) => row.try_get("", "rows")?,
      None => 0,
    };

    held.push(Holding {
      table: kept.table.to_owned(),
      rows,
      why: kept.why.to_owned(),
    });
  }

  Ok(held)
}

/// `GET /rooms/{chatId}/holding`.
pub struct RoomHolding;

impl Route for RoomHolding {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/rooms/{chatId}/holding";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let (mut parts, _) = std::mem::take(&mut call.request).into_parts();
    let Path(room) = Path::<String>::from_request_parts(&mut parts, &()).await.map_err(Refused::failed)?;
    let chat_id: i64 = room
      .parse()
      .map_err(|_| Refused::new(StatusCode::BAD_REQUEST, "bad_room", "the room is not a whole number"))?;
    let db = call.standing().archive.db().map_err(Refusal::from)?;

    Ok(answer(StatusCode::OK, json!({ "kept": holding(db, chat_id).await? })))
  }
}

crate::route!(RoomHolding);
