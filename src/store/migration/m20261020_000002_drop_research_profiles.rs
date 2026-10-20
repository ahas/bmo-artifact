//! `research_profiles` taken away with its rows: this service runs no research, and nothing here reads or
//! writes the table (`docs/archive.md`, "Where it lives").

use sea_orm_migration::prelude::*;

use super::m20261017_000001_research_profiles::TABLES;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20261020_000002_drop_research_profiles"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let db = manager.get_connection();

    for table in TABLES {
      db.execute_unprepared(&format!(r#"DROP TABLE IF EXISTS "{table}""#)).await?;
    }

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .get_connection()
      .execute_unprepared(include_str!("research_profiles.sql"))
      .await?;

    Ok(())
  }
}
