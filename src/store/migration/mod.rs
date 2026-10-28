//! The document tables' migrations, and the ones that made and took away tables no longer this
//! repository's, run by SeaORM's migrator, which keeps the ones it ran in its own table beside them
//! (`docs/layout.md`, "Storage").

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseConnection;

pub mod m20261016_000001_documents;
pub mod m20261017_000001_research_profiles;
pub mod m20261019_000001_news_briefs;
pub mod m20261019_000002_drop_news_briefs;
pub mod m20261020_000001_redesign;
pub mod m20261020_000002_drop_research_profiles;
pub mod m20261028_000001_share_notes;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
      Box::new(m20261016_000001_documents::Migration),
      Box::new(m20261017_000001_research_profiles::Migration),
      Box::new(m20261019_000001_news_briefs::Migration),
      Box::new(m20261019_000002_drop_news_briefs::Migration),
      Box::new(m20261020_000001_redesign::Migration),
      Box::new(m20261020_000002_drop_research_profiles::Migration),
      Box::new(m20261028_000001_share_notes::Migration),
    ]
  }
}

/// Every migration a database has not run yet, run on it; how many there were.
pub async fn migrate(db: &DatabaseConnection) -> Result<usize, DbErr> {
  let pending = Migrator::get_pending_migrations(db).await?.len();

  Migrator::up(db, None).await?;

  Ok(pending)
}
