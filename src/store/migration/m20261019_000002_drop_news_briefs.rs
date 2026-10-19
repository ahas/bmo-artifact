//! `news_briefs` and `holidays` taken away with their rows: no table of this repository's own, and
//! nothing here reads or writes them (`docs/layout.md`, "Storage").

use sea_orm_migration::prelude::*;

use super::m20261019_000001_news_briefs::TABLES;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20261019_000002_drop_news_briefs"
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
    manager.get_connection().execute_unprepared(include_str!("news_briefs.sql")).await?;

    Ok(())
  }
}
