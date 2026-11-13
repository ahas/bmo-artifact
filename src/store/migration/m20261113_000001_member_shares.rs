//! The member shares (`docs/artifact.md`, "Member shares"): one row for each member a document is shared with,
//! the member named by the account key alone.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20261113_000001_member_shares"
  }
}

/// The table this migration makes.
pub const TABLE: &str = "artifact_member_shares";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(include_str!("member_shares.sql")).await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .get_connection()
      .execute_unprepared(&format!(r#"DROP TABLE IF EXISTS "{TABLE}""#))
      .await?;

    Ok(())
  }
}
