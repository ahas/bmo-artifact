//! The document tables' migrations, from the 1.0.0 baseline, run by SeaORM's migrator, which keeps the ones it
//! ran in its own table beside them (`docs/layout.md`, "Storage").

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{
  ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait,
};

pub mod m20261124_000002_baseline;

/// The table the migrator keeps its record in.
pub const MIGRATIONS_TABLE: &str = "seaql_migrations";

/// The migration that makes the schema whole.
pub const BASELINE: &str = m20261124_000002_baseline::NAME;

/// The record an install from before 1.0.0 holds once every migration of its last release ran. Its schema
/// is the baseline's.
pub const BEFORE: [&str; 12] = [
  "m20261016_000001_documents",
  "m20261017_000001_research_profiles",
  "m20261019_000001_news_briefs",
  "m20261019_000002_drop_news_briefs",
  "m20261020_000001_redesign",
  "m20261020_000002_drop_research_profiles",
  "m20261028_000001_share_notes",
  "m20261113_000001_member_shares",
  "m20261116_000001_owners",
  "m20261121_000001_reads",
  "m20261123_000001_read_times",
  "m20261124_000001_places",
];

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m20261124_000002_baseline::Migration)]
  }
}

/// Every migration a database has not run yet, run on it, after an install from before 1.0.0 is taken as it
/// stands; how many there were.
pub async fn migrate(db: &DatabaseConnection) -> Result<usize, DbErr> {
  adopt(db).await?;
  let pending = Migrator::get_pending_migrations(db).await?.len();

  Migrator::up(db, None).await?;

  Ok(pending)
}

/// A record naming every migration of the last release before 1.0.0, and nothing else, rewritten to name the
/// baseline alone, touching no table and no row; one naming only part of them refused.
async fn adopt(db: &DatabaseConnection) -> Result<(), DbErr> {
  let tx = db.begin().await?;
  let statement = |sql: String, values: Vec<sea_orm::Value>| {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
  };
  let recorded = tx
    .query_one(statement(
      format!("SELECT to_regclass('{MIGRATIONS_TABLE}') IS NOT NULL AS held"),
      vec![],
    ))
    .await?
    .map(|row| row.try_get::<bool>("", "held"))
    .transpose()?
    .unwrap_or(false);
  if !recorded {
    return Ok(());
  }
  tx.execute_unprepared(&format!(r#"LOCK TABLE "{MIGRATIONS_TABLE}" IN EXCLUSIVE MODE"#))
    .await?;
  let held = tx
    .query_all(statement(
      format!(r#"SELECT "version" FROM "{MIGRATIONS_TABLE}" ORDER BY "version""#),
      vec![],
    ))
    .await?
    .into_iter()
    .map(|row| row.try_get::<String>("", "version"))
    .collect::<Result<Vec<_>, _>>()?;
  if held.is_empty() || !held.iter().all(|version| BEFORE.contains(&version.as_str())) {
    return Ok(());
  }
  if held.len() != BEFORE.len() {
    let reached = held.last().map_or("", String::as_str);
    return Err(DbErr::Migration(format!(
      "the database holds the migrations of a release before 1.0.0 only through {reached}; that release's \
       last migration runs first"
    )));
  }
  tx.execute_unprepared(&format!(r#"DELETE FROM "{MIGRATIONS_TABLE}""#)).await?;
  tx.execute(statement(
    format!(
      r#"INSERT INTO "{MIGRATIONS_TABLE}" ("version", "applied_at") VALUES ($1, EXTRACT(EPOCH FROM now())::bigint)"#
    ),
    vec![BASELINE.into()],
  ))
  .await?;
  tx.commit().await
}
