//! The note a share link leaves where it stops opening (`docs/artifact.md`, "Share links"): what it opened, the
//! second it ended and why, kept under a token that names no live link any more.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20261028_000001_share_notes"
  }
}

/// The table this migration makes.
pub const TABLE: &str = "web_search_share_notes";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(include_str!("share_notes.sql")).await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let db = manager.get_connection();

    db.execute_unprepared(r#"DROP INDEX IF EXISTS "web_search_shares_expiry""#).await?;
    db.execute_unprepared(&format!(r#"DROP TABLE IF EXISTS "{TABLE}""#)).await?;

    Ok(())
  }
}
