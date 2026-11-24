//! What the store keeps of one account or one unit, counted and taken as the platform administrator deletes it
//! for good (`docs/api.md`, 「Removing an account or a unit」). The documents an account owns are let go of one by
//! one by their own route's mechanism; this module counts them and takes the rest.

use sea_orm::ConnectionTrait;

use super::{all, one, run};
use crate::plumbing::refusal::Refusal;
use crate::wire::{AccountHeld, UnitHeld};

async fn count<C: ConnectionTrait>(db: &C, sql: &str, key: sea_orm::Value) -> Result<i64, Refusal> {
  let row = one(db, sql, vec![key]).await?;
  Ok(row.map(|row| row.try_get_by_index::<i64>(0)).transpose()?.unwrap_or(0))
}

/// What the store keeps of an account.
pub async fn account_held<C: ConnectionTrait>(db: &C, account: &str) -> Result<AccountHeld, Refusal> {
  Ok(AccountHeld {
    documents: count(db, r#"SELECT COUNT(*) FROM "searches" WHERE "owner" = $1"#, account.into()).await?,
    member_shares: count(
      db,
      r#"SELECT COUNT(*) FROM "artifact_member_shares" WHERE "user_key" = $1"#,
      account.into(),
    )
    .await?,
    imports: count(db, r#"SELECT COUNT(*) FROM "imports" WHERE "owner" = $1"#, account.into()).await?,
    reads: count(db, r#"SELECT COUNT(*) FROM "artifact_reads" WHERE "reader" = $1"#, account.into())
      .await?,
  })
}

/// The documents an account owns, each with the scope it is held under.
pub async fn owned<C: ConnectionTrait>(
  db: &C,
  account: &str,
) -> Result<Vec<(i64, Option<String>)>, Refusal> {
  all(
    db,
    r#"SELECT "id", "scope" FROM "searches" WHERE "owner" = $1 ORDER BY "id""#,
    vec![account.into()],
  )
  .await?
  .iter()
  .map(|row| Ok((row.try_get("", "id")?, row.try_get("", "scope")?)))
  .collect()
}

/// The account taken off every document shared with it, its imports and its reads forgotten; answers how many of
/// each went.
pub async fn forget_account<C: ConnectionTrait>(db: &C, account: &str) -> Result<(i64, i64, i64), Refusal> {
  let shares = run(
    db,
    r#"DELETE FROM "artifact_member_shares" WHERE "user_key" = $1"#,
    vec![account.into()],
  )
  .await?;
  let imports = run(db, r#"DELETE FROM "imports" WHERE "owner" = $1"#, vec![account.into()]).await?;
  let reads = run(db, r#"DELETE FROM "artifact_reads" WHERE "reader" = $1"#, vec![account.into()]).await?;
  let whole = |gone: u64| i64::try_from(gone).unwrap_or(i64::MAX);
  Ok((whole(shares), whole(imports), whole(reads)))
}

/// What the store keeps of a unit.
pub async fn unit_held<C: ConnectionTrait>(db: &C, unit: i64) -> Result<UnitHeld, Refusal> {
  Ok(UnitHeld {
    unit_shares: count(
      db,
      r#"SELECT COUNT(*) FROM "artifact_unit_shares" WHERE "unit_id" = $1"#,
      unit.into(),
    )
    .await?,
  })
}

/// The unit taken off every document shared with it; answers how many shares went.
pub async fn forget_unit<C: ConnectionTrait>(db: &C, unit: i64) -> Result<i64, Refusal> {
  let gone =
    run(db, r#"DELETE FROM "artifact_unit_shares" WHERE "unit_id" = $1"#, vec![unit.into()]).await?;
  Ok(i64::try_from(gone).unwrap_or(i64::MAX))
}
