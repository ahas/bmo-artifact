//! The company and the place a document was filed under (`docs/artifact.md`, "Places"): a document filed before
//! places were named keeps neither, and stands at the local place.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20261124_000001_places"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(include_str!("places.sql")).await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .get_connection()
      .execute_unprepared(
        r#"ALTER TABLE "searches" DROP COLUMN IF EXISTS "storage";
           ALTER TABLE "searches" DROP COLUMN IF EXISTS "company";"#,
      )
      .await?;

    Ok(())
  }
}
