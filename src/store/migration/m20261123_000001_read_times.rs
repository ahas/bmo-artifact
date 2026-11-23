//! When each read was last opened (`docs/artifact.md`, "Reads"): the moment a reader last opened a document on the
//! day a read stands for, by which the screens list the documents a reader read last. A read kept before stands at
//! the start of its day.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20261123_000001_read_times"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(include_str!("read_times.sql")).await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .get_connection()
      .execute_unprepared(
        r#"DROP INDEX IF EXISTS "artifact_reads_reader";
           ALTER TABLE "artifact_reads" DROP COLUMN IF EXISTS "read_at";"#,
      )
      .await?;

    Ok(())
  }
}
