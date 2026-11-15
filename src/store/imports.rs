//! The imports asked for, as rows of `imports` (`docs/api.md`, "Imports"). Their times are epoch
//! milliseconds.

use sea_orm::{ConnectionTrait, QueryResult};

use super::{all, now_ms, one};
use crate::plumbing::refusal::Refusal;
use crate::wire::{Import, ImportInput, ImportState};

const COLUMNS: &str =
  r#""id", "url", "title", "scope", "actor_name", "owner", "state", "document_id", "error", "created_at""#;

/// One import's row: the import as the API answers it, and what it was asked to file under.
pub struct Asked {
  pub import: Import,
  pub title: Option<String>,
  pub scope: Option<String>,
  pub actor_name: String,
  /// The account the document is filed for, none for a document with no owner.
  pub owner: Option<String>,
}

fn asked_of(row: &QueryResult) -> Result<Asked, Refusal> {
  let state: String = row.try_get("", "state")?;

  Ok(Asked {
    import: Import {
      id: row.try_get("", "id")?,
      url: row.try_get("", "url")?,
      state: ImportState::of_word(&state).unwrap_or(ImportState::Failed),
      document_id: row.try_get("", "document_id")?,
      error: row.try_get("", "error")?,
      created_at: row.try_get("", "created_at")?,
    },
    title: row.try_get("", "title")?,
    scope: row.try_get("", "scope")?,
    actor_name: row.try_get("", "actor_name")?,
    owner: row.try_get("", "owner")?,
  })
}

/// An import written as asked for `owner`, none for a document with no owner, `fetching`.
pub async fn insert<C: ConnectionTrait>(
  db: &C,
  asked: &ImportInput,
  owner: Option<&str>,
) -> Result<Asked, Refusal> {
  let row = one(
    db,
    &format!(
      r#"INSERT INTO "imports" ("url", "title", "scope", "actor_name", "state", "created_at", "owner")
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {COLUMNS}"#
    ),
    vec![
      asked.url.trim().into(),
      asked.title.clone().filter(|title| !title.trim().is_empty()).into(),
      asked.scope.clone().into(),
      asked.actor_name.clone().into(),
      ImportState::Fetching.word().into(),
      now_ms().into(),
      owner.map(str::to_owned).into(),
    ],
  )
  .await?
  .ok_or_else(|| Refusal::failed("the import's row was not written"))?;

  asked_of(&row)
}

/// One import, none where it does not stand.
pub async fn read<C: ConnectionTrait>(db: &C, id: i64) -> Result<Option<Asked>, Refusal> {
  one(db, &format!(r#"SELECT {COLUMNS} FROM "imports" WHERE "id" = $1"#), vec![id.into()])
    .await?
    .as_ref()
    .map(asked_of)
    .transpose()
}

/// An import moved on: filed with its document, or failed with why. The import as it now stands.
pub async fn settle<C: ConnectionTrait>(
  db: &C,
  id: i64,
  filed: Result<i64, &str>,
) -> Result<Option<Import>, Refusal> {
  let (state, document_id, error) = match filed {
    Ok(document_id) => (ImportState::Filed, Some(document_id), None),
    Err(error) => (ImportState::Failed, None, Some(error.to_owned())),
  };

  one(
    db,
    &format!(
      r#"UPDATE "imports" SET "state" = $1, "document_id" = $2, "error" = $3 WHERE "id" = $4 RETURNING {COLUMNS}"#
    ),
    vec![state.word().into(), document_id.into(), error.into(), id.into()],
  )
  .await?
  .as_ref()
  .map(|row| asked_of(row).map(|asked| asked.import))
  .transpose()
}

/// Every import still `fetching`, failed with `why`: a process that stopped left them unread. The imports as
/// they now stand.
pub async fn abandon<C: ConnectionTrait>(db: &C, why: &str) -> Result<Vec<Import>, Refusal> {
  all(
    db,
    &format!(r#"UPDATE "imports" SET "state" = $1, "error" = $2 WHERE "state" = $3 RETURNING {COLUMNS}"#),
    vec![ImportState::Failed.word().into(), why.into(), ImportState::Fetching.word().into()],
  )
  .await?
  .iter()
  .map(|row| asked_of(row).map(|asked| asked.import))
  .collect()
}
