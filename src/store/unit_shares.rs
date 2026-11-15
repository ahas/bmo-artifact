//! The organisation units a document is shared with, as rows of `artifact_unit_shares` (`docs/artifact.md`, "Unit
//! shares"): each unit named by its number alone, as the account service numbers it. This is the one module that
//! writes the table. Its times are epoch milliseconds.

use sea_orm::{ConnectionTrait, QueryResult};

use super::{all, run};
use crate::plumbing::refusal::Refusal;
use crate::wire::UnitShare;

const COLUMNS: &str = r#""id", "search_id", "unit_id", "shared_at""#;

fn unit_of(row: &QueryResult) -> Result<UnitShare, Refusal> {
  Ok(UnitShare {
    id: row.try_get("", "id")?,
    document_id: row.try_get("", "search_id")?,
    unit: row.try_get("", "unit_id")?,
    shared_at: row.try_get("", "shared_at")?,
  })
}

/// The units one document is shared with, in the order they were shared, after the unit share numbered `after`
/// where that is given, at most `limit`; and whether more stand past them.
pub async fn list(
  db: &impl ConnectionTrait,
  document_id: i64,
  after: Option<i64>,
  limit: i64,
) -> Result<(Vec<UnitShare>, bool), Refusal> {
  let rows = all(
    db,
    &format!(
      r#"SELECT {COLUMNS} FROM "artifact_unit_shares"
          WHERE "search_id" = $1 AND "id" > $2 ORDER BY "id" LIMIT $3"#
    ),
    vec![document_id.into(), after.unwrap_or(i64::MIN).into(), (limit + 1).into()],
  )
  .await?;
  let mut units = rows.iter().map(unit_of).collect::<Result<Vec<_>, _>>()?;
  let more = units.len() as i64 > limit;

  units.truncate(usize::try_from(limit).unwrap_or(usize::MAX));

  Ok((units, more))
}

/// One document shared with every unit of `units` at `now`, a unit already sharing it kept as it stands; the
/// shares of those units, in the order they were shared.
pub async fn add(
  db: &impl ConnectionTrait,
  document_id: i64,
  units: &[i64],
  now: i64,
) -> Result<Vec<UnitShare>, Refusal> {
  for unit in units {
    run(
      db,
      r#"INSERT INTO "artifact_unit_shares" ("search_id", "unit_id", "shared_at")
         VALUES ($1, $2, $3) ON CONFLICT ("search_id", "unit_id") DO NOTHING"#,
      vec![document_id.into(), (*unit).into(), now.into()],
    )
    .await?;
  }

  let numbers = (2..units.len() + 2).map(|at| format!("${at}")).collect::<Vec<_>>().join(", ");
  let values = std::iter::once(document_id.into())
    .chain(units.iter().map(|unit| (*unit).into()))
    .collect();

  all(
    db,
    &format!(
      r#"SELECT {COLUMNS} FROM "artifact_unit_shares"
          WHERE "search_id" = $1 AND "unit_id" IN ({numbers}) ORDER BY "id""#
    ),
    values,
  )
  .await?
  .iter()
  .map(unit_of)
  .collect()
}

/// One unit taken off a document: whether it was shared with the unit to take off.
pub async fn remove(db: &impl ConnectionTrait, document_id: i64, unit: i64) -> Result<bool, Refusal> {
  Ok(
    run(
      db,
      r#"DELETE FROM "artifact_unit_shares" WHERE "search_id" = $1 AND "unit_id" = $2"#,
      vec![document_id.into(), unit.into()],
    )
    .await?
      > 0,
  )
}
