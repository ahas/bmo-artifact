//! The document tables' migrations, run by SeaORM's migrator, which keeps the ones it ran in its own
//! table beside them (`docs/layout.md`, "Storage").

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseConnection;

pub mod m20261016_000001_documents;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m20261016_000001_documents::Migration)]
  }
}

/// Every migration a database has not run yet, run on it; how many there were.
pub async fn migrate(db: &DatabaseConnection) -> Result<usize, DbErr> {
  let pending = Migrator::get_pending_migrations(db).await?.len();

  Migrator::up(db, None).await?;

  Ok(pending)
}
