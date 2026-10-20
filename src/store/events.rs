//! The service's events, as rows of `events` (`docs/api.md`, "Events"): each kept with the scope of the
//! member it is about, so a reader narrows by scope, and swept once it is older than the window.

use sea_orm::ConnectionTrait;
use serde_json::Value;

use super::{all, now_ms, one, run};
use crate::plumbing::refusal::Refusal;
use crate::wire::Event;

/// How long an event is kept: a week.
pub const WINDOW_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// An event kept: its kind, the scope of the member it is about and the member. The event as it is read.
pub async fn keep<C: ConnectionTrait>(
  db: &C,
  kind: &str,
  scope: Option<&str>,
  data: &Value,
) -> Result<Event, Refusal> {
  let at = now_ms();
  let id: i64 = one(
    db,
    r#"INSERT INTO "events" ("kind", "at", "scope", "data") VALUES ($1, $2, $3, $4::jsonb) RETURNING "id""#,
    vec![kind.into(), at.into(), scope.map(str::to_owned).into(), data.to_string().into()],
  )
  .await?
  .ok_or_else(|| Refusal::failed("the event was not kept"))?
  .try_get("", "id")?;

  Ok(Event {
    id,
    kind: kind.to_owned(),
    at,
    data: data.clone(),
  })
}

/// The events older than the window, swept.
pub async fn sweep<C: ConnectionTrait>(db: &C) -> Result<(), Refusal> {
  run(db, r#"DELETE FROM "events" WHERE "at" < $1"#, vec![(now_ms() - WINDOW_MS).into()])
    .await
    .map(drop)
}

/// The oldest event still kept, none where none is.
pub async fn oldest<C: ConnectionTrait>(db: &C) -> Result<Option<i64>, Refusal> {
  Ok(
    one(db, r#"SELECT MIN("id") AS "id" FROM "events""#, Vec::new())
      .await?
      .and_then(|row| row.try_get("", "id").ok()),
  )
}

/// The newest event kept, none where none is.
pub async fn newest<C: ConnectionTrait>(db: &C) -> Result<Option<i64>, Refusal> {
  Ok(
    one(db, r#"SELECT MAX("id") AS "id" FROM "events""#, Vec::new())
      .await?
      .and_then(|row| row.try_get("", "id").ok()),
  )
}

/// The events after `after`, oldest first, of the kinds named where any are and under the scope named where
/// one is, at most `limit`; and whether more stand past them.
pub async fn after<C: ConnectionTrait>(
  db: &C,
  after: i64,
  kinds: &[String],
  scope: Option<&str>,
  limit: i64,
) -> Result<(Vec<Event>, bool), Refusal> {
  let rows = all(
    db,
    r#"SELECT "id", "kind", "at", "data"::text AS "data" FROM "events"
        WHERE "id" > $1 AND ($2::text = '' OR "kind" = ANY(string_to_array($2::text, ',')))
          AND ($3::text IS NULL OR "scope" = $3)
        ORDER BY "id" LIMIT $4"#,
    vec![
      after.into(),
      kinds.join(",").into(),
      scope.map(str::to_owned).into(),
      (limit + 1).into(),
    ],
  )
  .await?;
  let mut events = rows
    .iter()
    .map(|row| {
      let data: String = row.try_get("", "data")?;

      Ok(Event {
        id: row.try_get("", "id")?,
        kind: row.try_get("", "kind")?,
        at: row.try_get("", "at")?,
        data: serde_json::from_str(&data).unwrap_or(Value::Null),
      })
    })
    .collect::<Result<Vec<_>, Refusal>>()?;
  let more = events.len() as i64 > limit;

  events.truncate(usize::try_from(limit).unwrap_or(usize::MAX));

  Ok((events, more))
}
