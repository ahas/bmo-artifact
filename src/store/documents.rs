//! The rows a document is written as: the document tables settled once a process opens them, a document
//! that answers no question started, and one attachment kept (`docs/archive.md`, "`searches`" and "The
//! other five tables").

use sea_orm::ConnectionTrait;

use super::{now_ms, one, run};
use crate::plumbing::refusal::Refusal;

/// The writer every search and import is recorded under.
pub const SEARCHER: &str = "searcher";

/// The document tables settled: a row that names no kind given its kind and its writer, an import where
/// its question opens with `kindless_import_prefix`, and every picture of `search_images` not yet among the
/// attachments copied there as an image named by its file, with no place.
pub async fn settle<C: ConnectionTrait>(
  db: &C,
  kindless_import_prefix: Option<&str>,
) -> Result<(), Refusal> {
  run(
    db,
    r#"UPDATE "searches"
         SET "kind" = CASE WHEN $1::text IS NOT NULL AND starts_with("query", $1::text) THEN 'import' ELSE 'search' END,
             "author" = COALESCE("author", $2)
       WHERE "kind" IS NULL"#,
    vec![kindless_import_prefix.map(str::to_owned).into(), SEARCHER.into()],
  )
  .await?;
  run(
    db,
    r#"INSERT INTO "archive_attachments" ("search_id", "media", "url", "path", "type", "bytes", "name", "place", "kept_at")
       SELECT i."search_id", 'image', i."url", i."path", i."type", i."bytes", regexp_replace(i."path", '^.*/', ''), NULL,
              i."kept_at"
         FROM "search_images" i
        WHERE NOT EXISTS (
          SELECT 1 FROM "archive_attachments" a WHERE a."search_id" = i."search_id" AND a."path" = i."path"
        )
       ON CONFLICT ("search_id", "path") DO NOTHING"#,
    Vec::new(),
  )
  .await?;

  Ok(())
}

/// A document that answers no question, as it is started.
pub struct NewDocument<'a> {
  /// The room that asked.
  pub chat_id: i64,
  /// The request as it was put.
  pub request: &'a str,
  pub title: &'a str,
  /// Its kind, which stands in place of a depth.
  pub kind: &'a str,
  pub asker_id: Option<i64>,
  /// The writer it is recorded under.
  pub author: &'a str,
}

/// A document's row started: born done, its moment of writing both its start and its end. Its number.
pub async fn start_document<C: ConnectionTrait>(db: &C, document: NewDocument<'_>) -> Result<i64, Refusal> {
  let now = now_ms() / 1000;

  one(
    db,
    r#"INSERT INTO "searches" ("chat_id", "query", "started_depth", "ended_depth", "status", "started_at", "finished_at",
                               "asker_id", "title", "documented", "kind", "author")
       VALUES ($1, $2, $3, $3, 'done', $4, $4, $5, $6, 1, $3, $7) RETURNING "id""#,
    vec![
      document.chat_id.into(),
      document.request.into(),
      document.kind.into(),
      now.into(),
      document.asker_id.into(),
      document.title.into(),
      document.author.into(),
    ],
  )
  .await?
  .ok_or_else(|| Refusal::bad("the document's row was not written"))?
  .try_get("", "id")
  .map_err(Refusal::from)
}

/// One file kept beside a document, as its row holds it.
pub struct Attachment<'a> {
  pub search_id: i64,
  /// `image`, `audio`, `video`, `text`, `page` or `file`.
  pub media: &'a str,
  /// The address it came from, where it came from one.
  pub url: Option<&'a str>,
  /// The file it was kept as.
  pub path: &'a str,
  /// Its type.
  pub kind: &'a str,
  pub bytes: i64,
  /// The name it is shown by.
  pub name: &'a str,
  /// Its place in the document.
  pub place: Option<i64>,
}

/// An attachment's row written: one row per file, so keeping the same file again writes over it.
pub async fn keep_attachment<C: ConnectionTrait>(db: &C, kept: Attachment<'_>) -> Result<(), Refusal> {
  run(
    db,
    r#"INSERT INTO "archive_attachments" ("search_id", "media", "url", "path", "type", "bytes", "name", "place", "kept_at")
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
       ON CONFLICT ("search_id", "path") DO UPDATE SET
         "media" = excluded."media", "url" = excluded."url", "type" = excluded."type", "bytes" = excluded."bytes",
         "name" = excluded."name", "place" = excluded."place", "kept_at" = excluded."kept_at""#,
    vec![
      kept.search_id.into(),
      kept.media.into(),
      kept.url.map(str::to_owned).into(),
      kept.path.into(),
      kept.kind.into(),
      kept.bytes.into(),
      kept.name.into(),
      kept.place.into(),
      (now_ms() / 1000).into(),
    ],
  )
  .await
  .map(drop)
}
