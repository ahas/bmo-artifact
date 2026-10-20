//! What the service holds under a caller's scope (`docs/api.md`, "Scopes"): its documents with their files
//! and links, and its imports, counted.

use sea_orm::ConnectionTrait;

use super::{all, one, run};
use crate::plumbing::refusal::Refusal;
use crate::wire::ScopeCounts;

/// How many members of each resource a scope holds.
pub async fn counts<C: ConnectionTrait>(db: &C, scope: &str) -> Result<ScopeCounts, Refusal> {
  let row = one(
    db,
    r#"SELECT
         (SELECT COUNT(*) FROM "searches" WHERE "scope" = $1) AS "documents",
         (SELECT COUNT(*) FROM "archive_attachments" a JOIN "searches" d ON d."id" = a."search_id" WHERE d."scope" = $1)
           AS "attachments",
         (SELECT COUNT(*) FROM "web_search_shares" s JOIN "searches" d ON d."id" = s."search_id" WHERE d."scope" = $1)
           AS "shares",
         (SELECT COUNT(*) FROM "imports" WHERE "scope" = $1) AS "imports""#,
    vec![scope.into()],
  )
  .await?
  .ok_or_else(|| Refusal::failed("the scope's counts were not read"))?;

  Ok(ScopeCounts {
    documents: row.try_get("", "documents")?,
    attachments: row.try_get("", "attachments")?,
    shares: row.try_get("", "shares")?,
    imports: row.try_get("", "imports")?,
  })
}

/// The documents held under a scope, by number.
pub async fn documents<C: ConnectionTrait>(db: &C, scope: &str) -> Result<Vec<i64>, Refusal> {
  all(
    db,
    r#"SELECT "id" FROM "searches" WHERE "scope" = $1 ORDER BY "id""#,
    vec![scope.into()],
  )
  .await?
  .iter()
  .map(|row| row.try_get::<i64>("", "id").map_err(Refusal::from))
  .collect()
}

/// The imports asked under a scope, let go of.
pub async fn drop_imports<C: ConnectionTrait>(db: &C, scope: &str) -> Result<(), Refusal> {
  run(db, r#"DELETE FROM "imports" WHERE "scope" = $1"#, vec![scope.into()])
    .await
    .map(drop)
}
