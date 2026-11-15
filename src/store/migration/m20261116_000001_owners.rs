//! Owners and unit shares (`docs/artifact.md`, "Owners" and "Unit shares"): each document's owner and the owner an
//! import files under, null for every row written before, and one row for each organisation unit a document is
//! shared with, the unit named by its number alone.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20261116_000001_owners"
  }
}

/// The table this migration makes.
pub const TABLE: &str = "artifact_unit_shares";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(include_str!("owners.sql")).await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .get_connection()
      .execute_unprepared(&format!(
        r#"DROP TABLE IF EXISTS "{TABLE}";
           ALTER TABLE "imports" DROP COLUMN IF EXISTS "owner";
           DROP INDEX IF EXISTS "searches_owner";
           ALTER TABLE "searches" DROP COLUMN IF EXISTS "owner";"#
      ))
      .await?;

    Ok(())
  }
}
