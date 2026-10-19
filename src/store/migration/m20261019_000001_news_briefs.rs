//! The morning news page's tables, `news_briefs`, one row per room that gets a page, and `holidays`, one
//! row per day off, in the shapes the old archive declares them; neither is seeded (`docs/layout.md`,
//! "Storage").

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20261019_000001_news_briefs"
  }
}

/// The tables this migration makes.
pub const TABLES: [&str; 2] = ["news_briefs", "holidays"];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(include_str!("news_briefs.sql")).await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let db = manager.get_connection();

    for table in TABLES {
      db.execute_unprepared(&format!(r#"DROP TABLE IF EXISTS "{table}""#)).await?;
    }

    Ok(())
  }
}
