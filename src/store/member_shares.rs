//! The members a document is shared with, as rows of `artifact_member_shares` (`docs/artifact.md`, "Member
//! shares"): each member named by the account key alone, with the permission it reads the document under. This
//! is the one module that writes the table. Its times are epoch milliseconds.

use sea_orm::{ConnectionTrait, QueryResult};

use super::{all, one, run};
use crate::plumbing::refusal::Refusal;
use crate::wire::{MemberShare, Permission};

const COLUMNS: &str = r#""id", "search_id", "user_key", "permission", "shared_at""#;

/// The condition a document's row, `d`, meets where it is shared with the member the placeholder `at` binds.
pub fn shared_with(at: &str) -> String {
  format!(
    r#"EXISTS (SELECT 1 FROM "artifact_member_shares" m WHERE m."search_id" = d."id" AND m."user_key" = {at})"#
  )
}

/// The permission the member the placeholder `at` binds holds on a document's row, `d`, as a column read
/// beside it: null where it is not shared with them.
pub fn permission_column(at: &str) -> String {
  format!(
    r#"(SELECT m."permission" FROM "artifact_member_shares" m WHERE m."search_id" = d."id" AND m."user_key" = {at})"#
  )
}

/// The permission a kept word names; a word no permission is written as reads as none.
pub fn permission_of(word: Option<String>) -> Option<Permission> {
  word.as_deref().and_then(Permission::of_word)
}

fn member_of(row: &QueryResult) -> Result<MemberShare, Refusal> {
  let word: String = row.try_get("", "permission")?;

  Ok(MemberShare {
    id: row.try_get("", "id")?,
    document_id: row.try_get("", "search_id")?,
    user: row.try_get("", "user_key")?,
    permission: Permission::of_word(&word)
      .ok_or_else(|| Refusal::failed(format!("a member share keeps no permission named {word:?}")))?,
    shared_at: row.try_get("", "shared_at")?,
  })
}

/// The members of one document, in the order they were shared, after the member numbered `after` where that is
/// given, at most `limit`; and whether more stand past them.
pub async fn list(
  db: &impl ConnectionTrait,
  document_id: i64,
  after: Option<i64>,
  limit: i64,
) -> Result<(Vec<MemberShare>, bool), Refusal> {
  let rows = all(
    db,
    &format!(
      r#"SELECT {COLUMNS} FROM "artifact_member_shares"
          WHERE "search_id" = $1 AND "id" > $2 ORDER BY "id" LIMIT $3"#
    ),
    vec![document_id.into(), after.unwrap_or(i64::MIN).into(), (limit + 1).into()],
  )
  .await?;
  let mut members = rows.iter().map(member_of).collect::<Result<Vec<_>, _>>()?;
  let more = members.len() as i64 > limit;

  members.truncate(usize::try_from(limit).unwrap_or(usize::MAX));

  Ok((members, more))
}

/// One document shared with every member of `users` under `permission` at `now`, a member already sharing it
/// kept as it stands; the shares of those members, in the order they were shared.
pub async fn add(
  db: &impl ConnectionTrait,
  document_id: i64,
  users: &[String],
  permission: Permission,
  now: i64,
) -> Result<Vec<MemberShare>, Refusal> {
  for user in users {
    run(
      db,
      r#"INSERT INTO "artifact_member_shares" ("search_id", "user_key", "permission", "shared_at")
         VALUES ($1, $2, $3, $4) ON CONFLICT ("search_id", "user_key") DO NOTHING"#,
      vec![document_id.into(), user.clone().into(), permission.word().into(), now.into()],
    )
    .await?;
  }

  let keys = (2..users.len() + 2).map(|at| format!("${at}")).collect::<Vec<_>>().join(", ");
  let values = std::iter::once(document_id.into())
    .chain(users.iter().map(|user| user.clone().into()))
    .collect();

  all(
    db,
    &format!(
      r#"SELECT {COLUMNS} FROM "artifact_member_shares"
          WHERE "search_id" = $1 AND "user_key" IN ({keys}) ORDER BY "id""#
    ),
    values,
  )
  .await?
  .iter()
  .map(member_of)
  .collect()
}

/// One member's permission on a document set: the share, none where the account is no member of it.
pub async fn set(
  db: &impl ConnectionTrait,
  document_id: i64,
  user: &str,
  permission: Permission,
) -> Result<Option<MemberShare>, Refusal> {
  one(
    db,
    &format!(
      r#"UPDATE "artifact_member_shares" SET "permission" = $3 WHERE "search_id" = $1 AND "user_key" = $2
         RETURNING {COLUMNS}"#
    ),
    vec![document_id.into(), user.into(), permission.word().into()],
  )
  .await?
  .as_ref()
  .map(member_of)
  .transpose()
}

/// One member taken off a document: whether it was a member to take off.
pub async fn remove(db: &impl ConnectionTrait, document_id: i64, user: &str) -> Result<bool, Refusal> {
  Ok(
    run(
      db,
      r#"DELETE FROM "artifact_member_shares" WHERE "search_id" = $1 AND "user_key" = $2"#,
      vec![document_id.into(), user.into()],
    )
    .await?
      > 0,
  )
}

/// The permission one member holds on a document, none where it is not shared with them.
pub async fn permission(
  db: &impl ConnectionTrait,
  document_id: i64,
  user: &str,
) -> Result<Option<Permission>, Refusal> {
  let row = one(
    db,
    r#"SELECT "permission" FROM "artifact_member_shares" WHERE "search_id" = $1 AND "user_key" = $2"#,
    vec![document_id.into(), user.into()],
  )
  .await?;

  Ok(match row {
    Some(row) => permission_of(row.try_get("", "permission")?),
    None => None,
  })
}
