//! The reads of the last seven days, as rows of `artifact_reads` (`docs/artifact.md`, "Reads"): one reader's day on
//! one document, kept once however often the reader opens it that day, and the documents most read across them.

use sea_orm::{ConnectionTrait, Value};

use super::documents::{COLUMNS, LEFT_A_DOCUMENT, row_of};
use super::member_shares::permission_of;
use super::reach::Account;
use super::{all, now_ms, run};
use crate::plumbing::refusal::Refusal;
use crate::wire::{Document, DocumentReads};

/// How many days of reads are kept and counted, today among them.
pub const DAYS: i64 = 7;

const DAY_MS: i64 = 86_400_000;

// The day a moment falls in, counted in whole days of the service clock's UTC from the epoch.
fn day_of(ms: i64) -> i64 {
  ms.div_euclid(DAY_MS)
}

// The first day the reads are counted from.
fn first_day(today: i64) -> i64 {
  today - (DAYS - 1)
}

/// A read of one document kept for one reader today, nothing more where the reader already read it today; every
/// read older than the days kept let go of.
pub async fn keep<C: ConnectionTrait>(db: &C, id: i64, reader: &str) -> Result<(), Refusal> {
  let today = day_of(now_ms());

  run(db, r#"DELETE FROM "artifact_reads" WHERE "day" < $1"#, vec![first_day(today).into()]).await?;
  run(
    db,
    r#"INSERT INTO "artifact_reads" ("search_id", "reader", "day") VALUES ($1, $2, $3)
       ON CONFLICT ("search_id", "reader", "day") DO NOTHING"#,
    vec![id.into(), reader.into(), today.into()],
  )
  .await
  .map(drop)
}

/// The documents with the most reads in the days counted, the most first and the newest first among equals; at
/// most `limit`, and whether more stand past them. Read for an `account`, it holds the documents that account reads
/// alone, each with the permission it reads it under where it reads it through a share.
pub async fn most_read<C: ConnectionTrait>(
  db: &C,
  limit: i64,
  account: Option<&Account<'_>>,
) -> Result<(Vec<DocumentReads>, bool), Refusal> {
  let mut values: Vec<Value> = vec![first_day(day_of(now_ms())).into(), (limit + 1).into()];
  let (reads, permission) = match account {
    Some(account) => {
      values.push(account.key.into());

      (format!(" AND {}", account.reads("$3")), account.permission("$3"))
    }
    None => (String::new(), "NULL::text".to_owned()),
  };
  let rows = all(
    db,
    &format!(
      r#"SELECT {COLUMNS}, {permission} AS "permission", r."reads"
           FROM "searches" d
           JOIN (SELECT "search_id", COUNT(*) AS "reads" FROM "artifact_reads" WHERE "day" >= $1 GROUP BY 1) r
             ON r."search_id" = d."id"
          WHERE {LEFT_A_DOCUMENT}{reads}
          ORDER BY r."reads" DESC, d."id" DESC
          LIMIT $2"#
    ),
    values,
  )
  .await?;
  let mut items = rows
    .iter()
    .map(|row| -> Result<DocumentReads, Refusal> {
      Ok(DocumentReads {
        document: Document {
          permission: permission_of(row.try_get("", "permission")?),
          ..row_of(row)?.document
        },
        reads: row.try_get("", "reads")?,
      })
    })
    .collect::<Result<Vec<_>, _>>()?;
  let more = items.len() as i64 > limit;

  items.truncate(usize::try_from(limit).unwrap_or(usize::MAX));

  Ok((items, more))
}
