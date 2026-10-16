//! The statements the carried code runs: bound at their widest, run once more where a table's shape
//! moved under a prepared one, refused in the carried code's own terms, and run on the database the
//! settings name (`docs/layout.md`, "Storage" and "Plumbing").

mod support;

use std::collections::HashMap;
use std::path::Path;

use axum::http::StatusCode;
use bmo_artifact::plumbing::http::Refused;
use bmo_artifact::plumbing::verb::{FAILURE, Shell, VerbError, finished};
use bmo_artifact::pool::{self, Fault};
use bmo_artifact::settings::Settings;
use bmo_artifact::{Refusal, all, now_ms, one, run};
use sea_orm::DbErr;
use support::ROOT;
use support::scratch::{Scratch, connect};

#[tokio::test]
async fn one_all_and_run_bind_every_number_at_its_widest() {
  let scratch = Scratch::new().await;
  let db = scratch.db();

  // The same text bound first with a 64-bit number and then with a 32-bit one.
  let insert = "INSERT INTO search_findings (search_id, place, text) VALUES ($1, $2, $3)";

  assert_eq!(
    run(db, insert, vec![1_i64.into(), 1_i64.into(), "first".into()])
      .await
      .expect("written"),
    1
  );
  assert_eq!(
    run(db, insert, vec![1_i32.into(), 2_u16.into(), "second".into()])
      .await
      .expect("written"),
    1
  );

  let rows = all(
    db,
    "SELECT text FROM search_findings WHERE search_id = $1 ORDER BY place",
    vec![1_i8.into()],
  )
  .await
  .expect("read");
  let texts: Vec<String> = rows.iter().map(|row| row.try_get("", "text").expect("a text")).collect();

  assert_eq!(texts, ["first", "second"]);

  let none = one(db, "SELECT id FROM search_findings WHERE search_id = $1", vec![2_i32.into()])
    .await
    .expect("read");

  assert!(none.is_none());
}

#[tokio::test]
async fn a_statement_prepared_before_its_table_moved_is_prepared_again() {
  let scratch = Scratch::new().await;
  let db = pool::connect(&scratch.address(), pool::pool(1)).await.expect("the pool opens");
  let read = "SELECT * FROM search_findings";

  scratch
    .run("INSERT INTO search_findings (search_id, place, text) VALUES (1, 1, 'a')", vec![])
    .await;
  pool::all(&db, read, vec![]).await.expect("read before the move");

  let other = connect(scratch.name()).await;

  pool::run(&other, "ALTER TABLE search_findings ADD COLUMN later text", vec![])
    .await
    .expect("moved");

  let rows = pool::all(&db, read, vec![]).await.expect("read after the move");

  assert_eq!(rows.len(), 1);
  assert!(rows[0].try_get::<Option<String>>("", "later").expect("the new column").is_none());
}

#[tokio::test]
async fn a_database_failure_is_refused_with_500_and_its_sentence() {
  let scratch = Scratch::new().await;
  let refusal = run(scratch.db(), "SELECT * FROM no_such_table", vec![]).await.expect_err("refused");

  assert_eq!(refusal.status, StatusCode::INTERNAL_SERVER_ERROR);
  assert!(refusal.detail.starts_with("the archive's database: "), "{}", refusal.detail);

  let refused = Refused::from(refusal);

  assert_eq!(refused.status, StatusCode::INTERNAL_SERVER_ERROR);
  assert_eq!(refused.code, "internal_server_error");
}

#[test]
fn a_refusal_carries_its_status_to_a_route_and_its_sentence_to_a_verb() {
  assert_eq!(
    Refused::from(Refusal::absent("no such document")),
    Refused::new(StatusCode::NOT_FOUND, "not_found", "no such document")
  );
  assert_eq!(Refused::from(Refusal::bad("no title")).code, "bad_request");
  assert_eq!(Refused::from(Refusal::forbidden("not yours")).status, StatusCode::FORBIDDEN);

  let ended = finished(Err(VerbError::from(Refusal::bad("no title"))));

  assert_eq!(ended.out, Vec::<String>::new());
  assert_eq!(ended.err.as_deref(), Some("no title"));
  assert_eq!(ended.code, FAILURE);

  let ended = finished(Err(VerbError::from(DbErr::Custom("gone".into()))));

  assert_eq!(ended.err.as_deref(), Some("the archive's database: Custom Error: gone"));
  assert_eq!(Fault::from(Refusal::bad("no title")), Fault::new("no title"));
}

#[test]
fn now_is_in_epoch_milliseconds() {
  let before = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .expect("after the epoch")
    .as_millis() as i64;
  let now = now_ms();

  assert!((now - before).abs() < 5_000, "{now} against {before}");
}

#[tokio::test]
async fn a_shell_opens_the_database_its_settings_name() {
  let scratch = Scratch::new().await;
  let environment = HashMap::from([("BMO_ARTIFACT_DATABASE__URL".to_owned(), scratch.address())]);
  let settings = Settings::read(Path::new(ROOT), Some(environment)).expect("the settings read");
  let shell = Shell::new(settings).expect("the shell stands");

  assert_eq!(shell.archive.place(), scratch.address());

  let row = one(shell.archive.db().expect("the pool opens"), "SELECT count(*) FROM searches", vec![])
    .await
    .expect("read")
    .expect("a row");

  assert_eq!(row.try_get_by_index::<i64>(0).expect("a count"), 0);
}
