//! The rows of the files a document keeps, `archive_attachments` (`docs/artifact.md`, "Attachments"), read
//! as the API answers them. Its times are epoch seconds, answered in milliseconds.

use sea_orm::{ConnectionTrait, QueryResult};

use super::{all, now_ms, one};
use crate::plumbing::refusal::Refusal;
use crate::wire::Attachment;

const COLUMNS: &str = r#""id", "search_id", "name", "type", "media", "bytes", "role", "kept_at" * 1000 AS "created_at", "path""#;

/// One kept file's row, with where it stands on the disk.
pub struct Kept {
  pub attachment: Attachment,
  pub path: String,
}

fn kept_of(row: &QueryResult) -> Result<Kept, Refusal> {
  Ok(Kept {
    attachment: Attachment {
      id: row.try_get("", "id")?,
      document_id: row.try_get("", "search_id")?,
      name: row.try_get("", "name")?,
      media_type: row.try_get("", "type")?,
      kind: row.try_get("", "media")?,
      byte_count: row.try_get("", "bytes")?,
      role: row.try_get("", "role")?,
      created_at: row.try_get("", "created_at")?,
    },
    path: row.try_get("", "path")?,
  })
}

/// A document's files in the order they were kept, after `after` where it is given, at most `limit`, and
/// whether more stand past them.
pub async fn list<C: ConnectionTrait>(
  db: &C,
  document_id: i64,
  after: Option<i64>,
  limit: i64,
) -> Result<(Vec<Kept>, bool), Refusal> {
  let rows = all(
    db,
    &format!(
      r#"SELECT {COLUMNS} FROM "archive_attachments" WHERE "search_id" = $1 AND "id" > $2 ORDER BY "id" LIMIT $3"#
    ),
    vec![document_id.into(), after.unwrap_or(i64::MIN).into(), (limit + 1).into()],
  )
  .await?;
  let mut kept = rows.iter().map(kept_of).collect::<Result<Vec<_>, _>>()?;
  let more = kept.len() as i64 > limit;

  kept.truncate(usize::try_from(limit).unwrap_or(usize::MAX));

  Ok((kept, more))
}

/// One file of a document, none where the document keeps no such file.
pub async fn read<C: ConnectionTrait>(db: &C, document_id: i64, id: i64) -> Result<Option<Kept>, Refusal> {
  one(
    db,
    &format!(r#"SELECT {COLUMNS} FROM "archive_attachments" WHERE "search_id" = $1 AND "id" = $2"#),
    vec![document_id.into(), id.into()],
  )
  .await?
  .as_ref()
  .map(kept_of)
  .transpose()
}

/// A document's file by the name it is kept under, none where it keeps none so named.
pub async fn named<C: ConnectionTrait>(
  db: &C,
  document_id: i64,
  name: &str,
) -> Result<Option<Kept>, Refusal> {
  one(
    db,
    &format!(
      r#"SELECT {COLUMNS} FROM "archive_attachments" WHERE "search_id" = $1 AND "name" = $2 ORDER BY "id" LIMIT 1"#
    ),
    vec![document_id.into(), name.into()],
  )
  .await?
  .as_ref()
  .map(kept_of)
  .transpose()
}

/// A kept file's row as it is written.
pub struct NewAttachment<'a> {
  pub document_id: i64,
  pub media: &'a str,
  pub url: Option<&'a str>,
  pub path: &'a str,
  pub media_type: &'a str,
  pub byte_count: i64,
  pub name: &'a str,
  pub role: Option<&'a str>,
}

/// A kept file's row written: one row per file, so keeping the same file again writes over it. The
/// attachment as the API answers it.
pub async fn insert<C: ConnectionTrait>(db: &C, kept: NewAttachment<'_>) -> Result<Attachment, Refusal> {
  let row = one(
    db,
    &format!(
      r#"INSERT INTO "archive_attachments" ("search_id", "media", "url", "path", "type", "bytes", "name", "place", "kept_at", "role")
         VALUES ($1, $2, $3, $4, $5, $6, $7, NULL, $8, $9)
         ON CONFLICT ("search_id", "path") DO UPDATE SET
           "media" = excluded."media", "url" = excluded."url", "type" = excluded."type", "bytes" = excluded."bytes",
           "name" = excluded."name", "kept_at" = excluded."kept_at", "role" = excluded."role"
         RETURNING {COLUMNS}"#
    ),
    vec![
      kept.document_id.into(),
      kept.media.into(),
      kept.url.map(str::to_owned).into(),
      kept.path.into(),
      kept.media_type.into(),
      kept.byte_count.into(),
      kept.name.into(),
      (now_ms() / 1000).into(),
      kept.role.map(str::to_owned).into(),
    ],
  )
  .await?
  .ok_or_else(|| Refusal::failed("the attachment's row was not written"))?;

  Ok(kept_of(&row)?.attachment)
}

/// The first file of a kind a document keeps, by the order it was kept; none where it keeps none.
pub async fn first_of_kind<C: ConnectionTrait>(
  db: &C,
  document_id: i64,
  media: &str,
) -> Result<Option<Kept>, Refusal> {
  one(
    db,
    &format!(
      r#"SELECT {COLUMNS} FROM "archive_attachments" WHERE "search_id" = $1 AND "media" = $2 ORDER BY "id" LIMIT 1"#
    ),
    vec![document_id.into(), media.into()],
  )
  .await?
  .as_ref()
  .map(kept_of)
  .transpose()
}
