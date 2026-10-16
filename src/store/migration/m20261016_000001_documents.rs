//! The six document tables and their four indexes, in the shape the old archive declares them, with
//! the nullable `user_id` it gave the share links (`docs/layout.md`, "Storage").

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20261016_000001_documents"
  }
}

/// The tables this migration makes, in the order they are dropped.
pub const TABLES: [&str; 6] = [
  "web_search_shares",
  "archive_attachments",
  "search_images",
  "search_sources",
  "search_findings",
  "searches",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(include_str!("documents.sql")).await?;

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
