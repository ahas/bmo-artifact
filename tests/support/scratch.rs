//! A test's own database on the test server (`docs/layout.md`, "Tests"): this repository's migrator run
//! once per test binary into a template database, and each test handed a copy of it, which the server
//! makes in a fraction of the time a migration takes. A copy is dropped when its test is done, and the
//! templates and copies a binary that is gone left behind are dropped by the next binary to make one.

use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};

use bmo_artifact::pool;
use bmo_artifact::store::migration::Migrator;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};
use sea_orm_migration::MigratorTrait;
use sqlx::postgres::{PgConnectOptions, PgConnection};
use sqlx::{Connection as _, Executor};

// What a template's and a copy's names begin with, ahead of the process that made them.
const TEMPLATE: &str = "bmo_rs_tmpl_";
const COPY: &str = "bmo_rs_copy_";

static COPIES: AtomicU32 = AtomicU32::new(0);
static MADE: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();

/// The test server, as the suite names it, and the machine's own socket where nothing does.
pub fn test_address() -> String {
  std::env::var("BMO_TEST_POSTGRES")
    .ok()
    .filter(|named| !named.is_empty())
    .unwrap_or_else(|| "postgres:///bmo_test?host=/var/run/postgresql".into())
}

/// Whether a process is alive, read off the kernel's own listing.
fn alive(pid: &str) -> bool {
  std::path::Path::new("/proc").join(pid).exists()
}

/// The test server's address with another database in place of its own.
pub fn address_of(database: &str) -> String {
  let mut address = url::Url::parse(&test_address()).expect("the test server's address is a URL");

  address.set_path(&format!("/{database}"));
  address.to_string()
}

async fn server() -> PgConnection {
  let options = PgConnectOptions::from_str(&address_of("postgres")).expect("the server's address reads");

  PgConnection::connect_with(&options).await.expect("the test server answers")
}

// The template this binary's copies are made from, migrated the first time a test asks. The databases a
// binary that is gone left behind are dropped first.
async fn template() -> String {
  MADE
    .get_or_init(|| async {
      let mut admin = server().await;
      let left: Vec<String> = sqlx::query_scalar(
        "SELECT datname::text FROM pg_database WHERE datname LIKE $1 OR datname LIKE $2",
      )
      .bind(format!("{}%", TEMPLATE.replace('_', "\\_")))
      .bind(format!("{}%", COPY.replace('_', "\\_")))
      .fetch_all(&mut admin)
      .await
      .expect("the left databases are listed");

      for old in left {
        let pid = old
          .trim_start_matches(TEMPLATE)
          .trim_start_matches(COPY)
          .split('_')
          .next()
          .unwrap_or_default();

        if !alive(pid) {
          let _ = admin.execute(format!("DROP DATABASE IF EXISTS \"{old}\" WITH (FORCE)").as_str()).await;
        }
      }

      let name = format!("{TEMPLATE}{}", std::process::id());

      create(&mut admin, &name, "template0 ENCODING 'UTF8' LC_COLLATE 'C' LC_CTYPE 'C'").await;
      admin.close().await.expect("the server connection closes");

      let db = connect(&name).await;

      Migrator::up(&db, None).await.expect("the template migrates");
      db.close().await.expect("the template's connection closes");

      name
    })
    .await
    .clone()
}

async fn create(admin: &mut PgConnection, database: &str, template: &str) {
  admin
    .execute(format!("CREATE DATABASE \"{database}\" TEMPLATE {template}").as_str())
    .await
    .expect("the database is made");
}

/// A connection to a database of the test server, through this repository's own pool.
pub async fn connect(database: &str) -> DatabaseConnection {
  pool::connect(&address_of(database), pool::pool(4)).await.expect("the database answers")
}

// A new database's name, unique to this process.
fn copy_name() -> String {
  format!("{COPY}{}_{}", std::process::id(), COPIES.fetch_add(1, Ordering::SeqCst))
}

/// A copy dropped, its connections ended with it. It goes on a thread of its own, since a guard dropped
/// inside the runtime may not block on it.
fn drop_copy(database: String) {
  let _ = std::thread::spawn(move || {
    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .expect("a runtime is built");

    runtime.block_on(async move {
      let mut admin = server().await;

      let _ = admin
        .execute(format!("DROP DATABASE IF EXISTS \"{database}\" WITH (FORCE)").as_str())
        .await;
      let _ = admin.close().await;
    });
  })
  .join();
}

/// A database of the test's own, dropped when the guard goes.
pub struct Scratch {
  database: String,
  db: DatabaseConnection,
}

impl Scratch {
  /// A copy of this binary's migrated, empty template.
  pub async fn new() -> Scratch {
    let template = template().await;

    Self::made(&format!("\"{template}\"")).await
  }

  /// A database with nothing in it, not even the migrations' record.
  pub async fn blank() -> Scratch {
    Self::made("template0").await
  }

  async fn made(template: &str) -> Scratch {
    let database = copy_name();
    let mut admin = server().await;

    create(&mut admin, &database, template).await;
    admin.close().await.expect("the server connection closes");

    let db = connect(&database).await;

    Scratch { database, db }
  }

  /// The database's name on the test server.
  pub fn name(&self) -> &str {
    &self.database
  }

  /// The database's address, as a reader handed one opens it.
  pub fn address(&self) -> String {
    address_of(&self.database)
  }

  pub fn db(&self) -> &DatabaseConnection {
    &self.db
  }

  /// A statement run as written.
  pub async fn run(&self, sql: &str, values: Vec<Value>) {
    self
      .db
      .execute(Statement::from_sql_and_values(DbBackend::Postgres, sql, values))
      .await
      .expect("the statement runs");
  }

  /// A count a statement answers.
  pub async fn count(&self, sql: &str, values: Vec<Value>) -> i64 {
    let row = self
      .db
      .query_one(Statement::from_sql_and_values(DbBackend::Postgres, sql, values))
      .await
      .expect("the statement reads")
      .expect("a row");

    row.try_get_by_index::<i64>(0).expect("a count")
  }
}

impl Drop for Scratch {
  fn drop(&mut self) {
    drop_copy(std::mem::take(&mut self.database));
  }
}
