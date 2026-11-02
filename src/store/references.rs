//! The pages a document refers to, as rows of `search_sources` (`docs/artifact.md`, "References").

use sea_orm::{ConnectionTrait, QueryResult};

use super::{all, one, run};
use crate::plumbing::refusal::Refusal;

/// One reference's row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cited {
  pub id: i64,
  pub document_id: i64,
  pub url: String,
  pub title: Option<String>,
}

const COLUMNS: &str = r#""id", "search_id", "url", NULLIF("title", '') AS "title""#;

fn cited_of(row: &QueryResult) -> Result<Cited, Refusal> {
  Ok(Cited {
    id: row.try_get("", "id")?,
    document_id: row.try_get("", "search_id")?,
    url: row.try_get("", "url")?,
    title: row.try_get("", "title")?,
  })
}

/// A document's references in its order, after `after` where it is given, at most `limit`, and whether more
/// stand past them.
pub async fn list<C: ConnectionTrait>(
  db: &C,
  document_id: i64,
  after: Option<i64>,
  limit: i64,
) -> Result<(Vec<Cited>, bool), Refusal> {
  let rows = all(
    db,
    &format!(r#"SELECT {COLUMNS} FROM "search_sources" WHERE "search_id" = $1 AND "id" > $2 ORDER BY "id" LIMIT $3"#),
    vec![document_id.into(), after.unwrap_or(i64::MIN).into(), (limit + 1).into()],
  )
  .await?;
  let mut cited = rows.iter().map(cited_of).collect::<Result<Vec<_>, _>>()?;
  let more = cited.len() as i64 > limit;

  cited.truncate(usize::try_from(limit).unwrap_or(usize::MAX));

  Ok((cited, more))
}

/// One reference of a document, none where the document holds no such reference.
pub async fn read<C: ConnectionTrait>(db: &C, document_id: i64, id: i64) -> Result<Option<Cited>, Refusal> {
  one(
    db,
    &format!(r#"SELECT {COLUMNS} FROM "search_sources" WHERE "search_id" = $1 AND "id" = $2"#),
    vec![document_id.into(), id.into()],
  )
  .await?
  .as_ref()
  .map(cited_of)
  .transpose()
}

/// A page a document's body refers to, written in its order.
pub async fn insert<C: ConnectionTrait>(
  db: &C,
  document_id: i64,
  finding_id: i64,
  url: &str,
  title: Option<&str>,
) -> Result<(), Refusal> {
  run(
    db,
    r#"INSERT INTO "search_sources" ("search_id", "finding_id", "url", "title") VALUES ($1, $2, $3, $4)"#,
    vec![document_id.into(), finding_id.into(), url.into(), title.map(str::to_owned).into()],
  )
  .await
  .map(drop)
}
