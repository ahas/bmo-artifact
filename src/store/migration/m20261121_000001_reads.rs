//! Reads (`docs/artifact.md`, "Reads"): one row for each reader's day on a document, the reads of the last seven
//! days the screens rank the documents most read by.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20261121_000001_reads"
  }
}

/// The table this migration makes.
pub const TABLE: &str = "artifact_reads";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(include_str!("reads.sql")).await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .get_connection()
      .execute_unprepared(&format!(r#"DROP TABLE IF EXISTS "{TABLE}";"#))
      .await?;

    Ok(())
  }
}
