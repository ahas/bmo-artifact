//! The old archive's document tables moved into the store (TODO #1923): the mapping, the redesigned
//! `searches` columns, `search_images` settled into the attachments without adding a picture, the refusal of
//! a store already holding rows, the dry run, and the count-and-sample compare.

mod support;

use std::path::Path;

use sea_orm::ConnectionTrait;
use support::scratch::Scratch;

const SEARCH: &str = r#"{"id":1,"chat_id":-9000000000001,"query":"where is it","started_depth":"one","ended_depth":"two","status":"done","started_at":1791866256,"finished_at":1791866300,"document":"searches/1/document.md","asker_id":1,"options":null,"title":"Where","topic":null,"keywords":null,"rule":null,"documented":1,"kind":"search","author":"searcher","rowid":1}"#;
const FINDING: &str =
  r#"{"id":1,"search_id":1,"run_id":2,"branch":"main","place":0,"text":"a finding","rowid":1}"#;
const SOURCE: &str = r#"{"id":1,"search_id":1,"finding_id":1,"url":"https://example.com","title":"Example","supports":"yes","mark":null,"rowid":1}"#;
const IMAGE: &str = r#"{"id":1,"search_id":1,"url":"https://example.com/a.png","path":"searches/1/images/a.png","type":"image/png","bytes":10,"kept_at":1791866256,"rowid":1}"#;
const ATTACHMENT: &str = r#"{"id":1,"search_id":1,"media":"image","url":"https://example.com/a.png","path":"searches/1/images/a.png","type":"image/png","bytes":10,"name":"a.png","place":null,"kept_at":1791866256,"rowid":1}"#;
const SHARE: &str = r#"{"share_token":"tok","search_id":1,"created_by":"BMO","created_at":1791866256,"expires_at":null,"rowid":1,"user_id":1}"#;

fn export(folder: &Path) {
  let tables: [(&str, &str, &str); 6] = [
    ("searches", "[\"id\"]", SEARCH),
    ("search_findings", "[\"id\"]", FINDING),
    ("search_sources", "[\"id\"]", SOURCE),
    ("search_images", "[\"id\"]", IMAGE),
    ("archive_attachments", "[\"id\"]", ATTACHMENT),
    ("web_search_shares", "[\"share_token\"]", SHARE),
  ];
  let mut manifest = Vec::new();
  for (source, key, row) in tables {
    manifest.push(format!(r#"{{"source":"{source}","file":"{source}.ndjson","count":1,"key":{key}}}"#));
    std::fs::write(folder.join(format!("{source}.ndjson")), format!("{row}\n"))
      .expect("the row is written");
  }
  let said = format!(
    r#"{{"snapshot":"23569140:23569140:","readAt":"2026-10-21T16:59:44+09:00","tables":[{}]}}"#,
    manifest.join(",")
  );
  std::fs::write(folder.join("manifest.json"), said).expect("the manifest is written");
}

#[tokio::test]
async fn the_document_tables_are_moved_and_compared() {
  let folder = tempfile::tempdir().expect("a scratch folder");
  export(folder.path());
  let scratch = Scratch::new().await;

  let report = bmo_artifact::import::run(&scratch.address(), folder.path(), false)
    .await
    .expect("the import runs");

  assert_eq!(report.tables.iter().map(|table| table.written).sum::<u64>(), 6);
  assert_eq!(report.sample.checked, 6);
  assert_eq!(report.sample.failures, Vec::<String>::new());

  let search = scratch
    .db()
    .query_one(sea_orm::Statement::from_string(
      sea_orm::DbBackend::Postgres,
      "SELECT scope, actor_name, changed_at FROM searches WHERE id = 1".to_owned(),
    ))
    .await
    .expect("the search reads")
    .expect("a row");
  assert_eq!(search.try_get_by_index::<Option<String>>(0).unwrap().as_deref(), Some("-9000000000001"));
  assert_eq!(search.try_get_by_index::<Option<String>>(1).unwrap().as_deref(), Some("searcher"));
  assert_eq!(search.try_get_by_index::<Option<i64>>(2).unwrap(), Some(1791866300000));
}

#[tokio::test]
async fn search_images_settle_into_the_attachments_without_adding_one() {
  let folder = tempfile::tempdir().expect("a scratch folder");
  export(folder.path());
  let scratch = Scratch::new().await;

  bmo_artifact::import::run(&scratch.address(), folder.path(), false)
    .await
    .expect("the import runs");
  let before = scratch.count("SELECT COUNT(*) FROM archive_attachments", Vec::new()).await;

  bmo_artifact::store::documents::settle(scratch.db(), None)
    .await
    .expect("the documents settle");

  let after = scratch.count("SELECT COUNT(*) FROM archive_attachments", Vec::new()).await;
  assert_eq!(before, 1);
  assert_eq!(after, 1, "the picture already stands among the attachments");
}

#[tokio::test]
async fn a_dry_run_writes_nothing() {
  let folder = tempfile::tempdir().expect("a scratch folder");
  export(folder.path());
  let scratch = Scratch::blank().await;

  bmo_artifact::import::run(&scratch.address(), folder.path(), true)
    .await
    .expect("the dry run runs");

  assert_eq!(scratch.count("SELECT COUNT(*) FROM searches", Vec::new()).await, 0);
}

#[tokio::test]
async fn a_store_holding_rows_is_refused() {
  let folder = tempfile::tempdir().expect("a scratch folder");
  export(folder.path());
  let scratch = Scratch::new().await;

  bmo_artifact::import::run(&scratch.address(), folder.path(), false)
    .await
    .expect("the first import runs");
  let refused = bmo_artifact::import::run(&scratch.address(), folder.path(), false)
    .await
    .expect_err("the second is refused");
  assert!(refused.contains("already holds rows"), "{refused}");
  assert!(refused.contains("searches holds 1"), "{refused}");
}
