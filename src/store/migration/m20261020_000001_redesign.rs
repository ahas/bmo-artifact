//! The document tables brought to the redesigned API (`docs/archive.md`, "Where it lives"): a document's
//! scope, actor, labels, description and change time beside its row, an attachment's role,
//! and the imports and the events, each a table of its own. A row written before carries its old grouping
//! number as its scope, in decimal, and its writer as its actor.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20261020_000001_redesign"
  }
}

/// The tables this migration makes, in the order they are dropped.
pub const TABLES: [&str; 2] = ["events", "imports"];

/// The columns it adds, by table.
const COLUMNS: [(&str, &str); 6] = [
  ("searches", "scope"),
  ("searches", "actor_name"),
  ("searches", "labels"),
  ("searches", "description"),
  ("searches", "changed_at"),
  ("archive_attachments", "role"),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(include_str!("redesign.sql")).await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let db = manager.get_connection();

    for table in TABLES {
      db.execute_unprepared(&format!(r#"DROP TABLE IF EXISTS "{table}""#)).await?;
    }

    db.execute_unprepared(r#"DROP INDEX IF EXISTS "searches_scope""#).await?;

    for (table, column) in COLUMNS {
      db.execute_unprepared(&format!(r#"ALTER TABLE "{table}" DROP COLUMN IF EXISTS "{column}""#))
        .await?;
    }

    db.execute_unprepared(r#"UPDATE "searches" SET "chat_id" = 0 WHERE "chat_id" IS NULL"#)
      .await?;
    db.execute_unprepared(r#"ALTER TABLE "searches" ALTER COLUMN "chat_id" SET NOT NULL"#)
      .await?;

    Ok(())
  }
}
