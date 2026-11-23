//! The rows a document is written and read as (`docs/artifact.md`, "Where it lives"): the document tables
//! settled once a process opens them, a document's row read as the API answers it, the list with its
//! filters, a row filed and rewritten, its body as its one finding, and a document let go of.

use sea_orm::{ConnectionTrait, QueryResult, Value};

use super::member_shares::permission_of;
use super::reach::Account;
use super::{all, now_ms, one, run};
use crate::kinds;
use crate::plumbing::refusal::Refusal;
use crate::wire::{Document, DocumentsQuery, LabelValue, Labels};

/// The writer every search and import written before the kind was kept is recorded under.
pub const SEARCHER: &str = "searcher";

/// The longest phrase a search reads.
pub const PHRASE_LENGTH: usize = 200;

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
             "author" = COALESCE("author", $2),
             "actor_name" = COALESCE("actor_name", "author", $2)
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

// Whether the document `d` keeps a picture: what its `hasPicture` says and the list's `picture` holds it to.
macro_rules! keeps_picture {
  () => {
    r#"EXISTS (SELECT 1 FROM "archive_attachments" a WHERE a."search_id" = d."id" AND a."media" = 'image')"#
  };
}

// What a document's row is read as, every time and wherever: its times in milliseconds, its title its
// question where it wrote none, its actor its writer where none was kept.
pub(super) const COLUMNS: &str = concat!(
  r#"d."id", d."kind", COALESCE(NULLIF(d."title", ''), d."query") AS "title", d."scope",
  COALESCE(d."actor_name", d."author", '') AS "actor_name", d."owner", d."labels"::text AS "labels", d."description",
  (SELECT COUNT(*) FROM "archive_attachments" a WHERE a."search_id" = d."id") AS "attachment_count", "#,
  keeps_picture!(),
  r#" AS "has_picture",
  d."started_at" * 1000 AS "created_at", COALESCE(d."changed_at", d."started_at" * 1000) AS "changed_at",
  d."document", d."storage"::text AS "storage", d."company""#
);

// A row that left a document: one that names its file and is not marked as having written none. Every
// other row is no document, never listed and read as absent.
pub(crate) const LEFT_A_DOCUMENT: &str =
  r#"NULLIF(d."document", '') IS NOT NULL AND COALESCE(d."documented", 1) <> 0"#;

/// A document's row as the API answers it, with the file it reads from and what it was filed under.
pub struct Row {
  pub document: Document,
  /// The path of the document's file at its place, none where it keeps none.
  pub file: Option<String>,
  /// The storage value that chose its place, none for the local place (`docs/artifact.md`, "Places").
  pub storage: Option<serde_json::Value>,
  /// The company its owner stood in as it was filed.
  pub company: Option<i64>,
}

pub(super) fn row_of(row: &QueryResult) -> Result<Row, Refusal> {
  let stored: Option<String> = row.try_get("", "kind")?;
  let labels: String = row.try_get("", "labels")?;

  Ok(Row {
    document: Document {
      id: row.try_get("", "id")?,
      kind: kinds::kind(stored.as_deref()).name().to_owned(),
      title: row.try_get("", "title")?,
      scope: row.try_get("", "scope")?,
      actor_name: row.try_get("", "actor_name")?,
      owner: row.try_get("", "owner")?,
      labels: serde_json::from_str(&labels).unwrap_or_default(),
      description: row.try_get("", "description")?,
      attachment_count: row.try_get("", "attachment_count")?,
      has_picture: row.try_get("", "has_picture")?,
      created_at: row.try_get("", "created_at")?,
      changed_at: row.try_get("", "changed_at")?,
      permission: None,
    },
    file: row.try_get::<Option<String>>("", "document")?.filter(|file| !file.is_empty()),
    storage: row
      .try_get::<Option<String>>("", "storage")?
      .and_then(|storage| serde_json::from_str(&storage).ok()),
    company: row.try_get("", "company")?,
  })
}

/// One document's row, none where it does not stand or its row left no document.
pub async fn read<C: ConnectionTrait>(db: &C, id: i64) -> Result<Option<Row>, Refusal> {
  one(
    db,
    &format!(r#"SELECT {COLUMNS} FROM "searches" d WHERE d."id" = $1 AND {LEFT_A_DOCUMENT}"#),
    vec![id.into()],
  )
  .await?
  .as_ref()
  .map(row_of)
  .transpose()
}

/// One page of documents, newest first, filtered in the reading itself; and whether more stand past it.
/// A row that left no document is never listed. Read for an `account`, it holds the documents that account reads
/// alone, each with the permission it is read under where it reads it through a share.
pub async fn list<C: ConnectionTrait>(
  db: &C,
  query: &DocumentsQuery,
  limit: i64,
  account: Option<&Account<'_>>,
) -> Result<(Vec<Document>, bool), Refusal> {
  let mut values: Vec<Value> = Vec::new();
  let mut filters = vec![LEFT_A_DOCUMENT.to_owned()];
  let bind = |value: Value, values: &mut Vec<Value>| {
    values.push(value);
    format!("${}", values.len())
  };
  let permission = match account {
    Some(account) => {
      let at = bind(account.key.into(), &mut values);

      filters.push(account.reads(&at));
      account.permission(&at)
    }
    None => "NULL::text".to_owned(),
  };

  if let Some(kind) = query.kind.as_deref().filter(|kind| !kind.is_empty()) {
    let at = bind(kind.into(), &mut values);
    // A row whose kind no module answers reads as a record, so a record's filter takes it in.
    let unknown = kinds::KINDS
      .iter()
      .map(|kind| format!("'{}'", kind.name()))
      .collect::<Vec<_>>()
      .join(", ");

    filters.push(format!(
      r#"(d."kind" = {at} OR ({at} = 'record' AND (d."kind" IS NULL OR d."kind" NOT IN ({unknown}))))"#
    ));
  }

  if let Some(scope) = &query.scope {
    filters.push(format!(r#"d."scope" = {}"#, bind(scope.clone().into(), &mut values)));
  }

  if let Some(label) = query.label.as_deref().filter(|label| !label.is_empty()) {
    match label.split_once(':') {
      Some((key, value)) => {
        let key = bind(key.into(), &mut values);
        let value = bind(value.into(), &mut values);

        filters.push(format!(r#"d."labels" ->> {key} = {value}"#));
      }
      None => filters.push(format!(r#"d."labels" ? {}"#, bind(label.into(), &mut values))),
    }
  }

  let phrase: String = query.q.as_deref().unwrap_or_default().trim().chars().take(PHRASE_LENGTH).collect();

  if !phrase.is_empty() {
    let at = bind(phrase.to_lowercase().into(), &mut values);
    // A phrase that is a number, with or without its `#`, finds the document of that number too.
    let numbered = match number_of(&phrase) {
      Some(id) => format!(r#" OR d."id" = {}"#, bind(id.into(), &mut values)),
      None => String::new(),
    };

    filters.push(format!(
      r#"(strpos(lower(COALESCE(d."title", '') || ' ' || d."query"), {at}) > 0
          OR strpos(lower(COALESCE(d."topic", '') || ' ' || COALESCE(d."keywords", '')), {at}) > 0
          OR EXISTS (SELECT 1 FROM jsonb_each_text(d."labels") l WHERE strpos(lower(l."value"), {at}) > 0)
          OR EXISTS (SELECT 1 FROM "search_findings" f WHERE f."search_id" = d."id" AND strpos(lower(f."text"), {at}) > 0){numbered})"#
    ));
  }

  if query.picture == Some(true) {
    filters.push(keeps_picture!().to_owned());
  }

  if let Some(before) = query.before {
    filters.push(format!(r#"d."id" < {}"#, bind(before.into(), &mut values)));
  }

  if let Some(after) = query.after {
    filters.push(format!(r#"d."id" > {}"#, bind(after.into(), &mut values)));
  }

  let take = bind((limit + 1).into(), &mut values);
  let rows = all(
    db,
    &format!(
      r#"SELECT {COLUMNS}, {permission} AS "permission" FROM "searches" d WHERE {} ORDER BY d."id" DESC
          LIMIT {take}"#,
      filters.join(" AND ")
    ),
    values,
  )
  .await?;
  let mut documents = rows
    .iter()
    .map(|row| -> Result<Document, Refusal> {
      Ok(Document {
        permission: permission_of(row.try_get("", "permission")?),
        ..row_of(row)?.document
      })
    })
    .collect::<Result<Vec<_>, _>>()?;
  let more = documents.len() as i64 > limit;

  documents.truncate(usize::try_from(limit).unwrap_or(usize::MAX));

  Ok((documents, more))
}

/// The document number a phrase names: its digits alone, a leading `#` aside.
pub fn number_of(phrase: &str) -> Option<i64> {
  let digits = phrase.strip_prefix('#').unwrap_or(phrase);

  if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
    return None;
  }

  digits.parse().ok()
}

/// The distinct values the label `key` holds across the listed documents, the most carried first and, among
/// equals, in the values' order; at most `limit`, and whether more stand past them. Read for an `account`, it
/// counts the documents that account reads alone.
pub async fn label_values<C: ConnectionTrait>(
  db: &C,
  key: &str,
  limit: i64,
  account: Option<&Account<'_>>,
) -> Result<(Vec<LabelValue>, bool), Refusal> {
  let shared = match account {
    Some(account) => format!(" AND {}", account.reads("$3")),
    None => String::new(),
  };
  let mut values: Vec<Value> = vec![key.into(), (limit + 1).into()];

  values.extend(account.map(|account| Value::from(account.key)));

  let rows = all(
    db,
    &format!(
      r#"SELECT d."labels" ->> $1 AS "value", COUNT(*) AS "count" FROM "searches" d
          WHERE {LEFT_A_DOCUMENT} AND d."labels" ->> $1 IS NOT NULL{shared}
          GROUP BY 1 ORDER BY 2 DESC, 1 LIMIT $2"#
    ),
    values,
  )
  .await?;
  let mut items = rows
    .iter()
    .map(|row| -> Result<LabelValue, Refusal> {
      Ok(LabelValue {
        value: row.try_get("", "value")?,
        count: row.try_get("", "count")?,
      })
    })
    .collect::<Result<Vec<_>, _>>()?;
  let more = items.len() as i64 > limit;

  items.truncate(usize::try_from(limit).unwrap_or(usize::MAX));

  Ok((items, more))
}

/// A document's row as it is filed.
pub struct NewRow<'a> {
  pub kind: &'a str,
  pub title: &'a str,
  pub scope: Option<&'a str>,
  pub actor_name: &'a str,
  /// The account that owns it, none for a document with no owner.
  pub owner: Option<&'a str>,
  pub labels: &'a Labels,
  pub description: Option<&'a str>,
}

/// A document's row written: done from the moment it is filed, its moment of filing both its start and its
/// end, its kind in place of a depth. Its number.
pub async fn insert<C: ConnectionTrait>(db: &C, row: NewRow<'_>) -> Result<i64, Refusal> {
  let now = now_ms();

  one(
    db,
    r#"INSERT INTO "searches" ("chat_id", "query", "started_depth", "ended_depth", "status", "started_at", "finished_at",
                               "title", "documented", "kind", "author", "scope", "actor_name", "labels", "description",
                               "changed_at", "owner")
       VALUES (NULL, $1, $2, $2, 'done', $3, $3, $1, 1, $2, $4, $5, $4, $6::jsonb, $7, $8, $9) RETURNING "id""#,
    vec![
      row.title.into(),
      row.kind.into(),
      (now / 1000).into(),
      row.actor_name.into(),
      row.scope.map(str::to_owned).into(),
      labels_text(row.labels).into(),
      row.description.map(str::to_owned).into(),
      now.into(),
      row.owner.map(str::to_owned).into(),
    ],
  )
  .await?
  .ok_or_else(|| Refusal::failed("the document's row was not written"))?
  .try_get("", "id")
  .map_err(Refusal::from)
}

fn labels_text(labels: &Labels) -> String {
  serde_json::to_string(labels).unwrap_or_else(|_| "{}".to_owned())
}

/// A row rewritten: its title where one is given, its labels whole, its description where one is given, and
/// its change time now.
pub async fn rewrite<C: ConnectionTrait>(
  db: &C,
  id: i64,
  title: Option<&str>,
  labels: &Labels,
  description: Option<Option<&str>>,
) -> Result<(), Refusal> {
  run(
    db,
    r#"UPDATE "searches"
          SET "title" = COALESCE($1, "title"), "query" = COALESCE($1, "query"), "labels" = $2::jsonb,
              "description" = CASE WHEN $3 THEN $4 ELSE "description" END, "changed_at" = $5
        WHERE "id" = $6"#,
    vec![
      title.map(str::to_owned).into(),
      labels_text(labels).into(),
      description.is_some().into(),
      description.flatten().map(str::to_owned).into(),
      now_ms().into(),
      id.into(),
    ],
  )
  .await
  .map(drop)
}

/// The company and the place a document is filed under, set as it is filed.
pub async fn set_place<C: ConnectionTrait>(
  db: &C,
  id: i64,
  storage: Option<&serde_json::Value>,
  company: Option<i64>,
) -> Result<(), Refusal> {
  run(
    db,
    r#"UPDATE "searches" SET "storage" = $1::text::jsonb, "company" = $2 WHERE "id" = $3"#,
    vec![storage.map(serde_json::Value::to_string).into(), company.into(), id.into()],
  )
  .await
  .map(drop)
}

/// The file a document reads from, set.
pub async fn set_file<C: ConnectionTrait>(db: &C, id: i64, file: &str) -> Result<(), Refusal> {
  run(
    db,
    r#"UPDATE "searches" SET "document" = $1, "documented" = 1 WHERE "id" = $2"#,
    vec![file.into(), id.into()],
  )
  .await
  .map(drop)
}

/// A document's body set as its one finding, the references it made let go of with what it replaced. The
/// finding's number.
pub async fn set_body<C: ConnectionTrait>(db: &C, id: i64, text: &str) -> Result<i64, Refusal> {
  for table in ["search_sources", "search_findings"] {
    run(db, &format!(r#"DELETE FROM "{table}" WHERE "search_id" = $1"#), vec![id.into()]).await?;
  }

  one(
    db,
    r#"INSERT INTO "search_findings" ("search_id", "run_id", "branch", "place", "text") VALUES ($1, NULL, NULL, 1, $2)
       RETURNING "id""#,
    vec![id.into(), text.into()],
  )
  .await?
  .ok_or_else(|| Refusal::failed("the document's body was not written"))?
  .try_get("", "id")
  .map_err(Refusal::from)
}

/// A document's rows let go of: its references, its body, its
/// attachments, its member and unit shares, its reads, its share links and itself, its link leaving a note of what it opened. The share tokens it
/// held.
pub async fn delete<C: ConnectionTrait>(db: &C, id: i64) -> Result<Vec<String>, Refusal> {
  // Its link leaves a note of what it opened behind it, written while the document's own row still stands.
  super::shares::note_of_document(db, id, now_ms() / 1000, super::shares::RELEASED).await?;

  let tokens = all(
    db,
    r#"SELECT "share_token" FROM "web_search_shares" WHERE "search_id" = $1"#,
    vec![id.into()],
  )
  .await?
  .iter()
  .map(|row| row.try_get::<String>("", "share_token"))
  .collect::<Result<Vec<_>, _>>()?;

  for table in [
    "search_sources",
    "search_findings",
    "search_images",
    "archive_attachments",
    "artifact_member_shares",
    "artifact_unit_shares",
    "artifact_reads",
    "web_search_shares",
  ] {
    run(db, &format!(r#"DELETE FROM "{table}" WHERE "search_id" = $1"#), vec![id.into()]).await?;
  }

  run(db, r#"DELETE FROM "searches" WHERE "id" = $1"#, vec![id.into()]).await?;

  Ok(tokens)
}
