//! A search as a record that stands (`docs/archive.md`, "`searches`" and "Findings and sources"): its row
//! opened, a run's ending put under it, and the document a person reads written beside it.

mod support;

use bmo_artifact::store::documents::SEARCHER;
use bmo_artifact::verbs::search::record::{
  Ending, NewSearch, finish_search, join_search, read_search, record_run, reopen_search, start_search,
};
use bmo_artifact::verbs::search::request::{Options, options};
use bmo_artifact::wording::Wording;
use support::scratch::Scratch;

const RESULT: &str = "ANSWERED: yes :: price\nTITLE: what to buy\nNAME: used\nUsed ones go for about 70% of launch price.\nSOURCE: https://joonggo.example/m4 :: price\n===ITEM===\nSUMMARY: buying new is better.";

fn stream() -> String {
  [
    serde_json::json!({ "type": "assistant", "parent_tool_use_id": null, "message": { "content": [{ "type": "tool_use", "id": "t1", "name": "mcp__web__web_fetch", "input": { "url": "https://joonggo.example/m4" } }] } }),
    serde_json::json!({ "type": "assistant", "parent_tool_use_id": null, "message": { "content": [{ "type": "tool_use", "id": "t2", "name": "mcp__web__web_fetch", "input": { "url": "https://apple.com/kr/shop/mac" } }] } }),
  ]
  .map(|event| event.to_string())
  .join("\n")
}

async fn opened(scratch: &Scratch, options: &Options) -> i64 {
  start_search(
    scratch.db(),
    &NewSearch {
      chat_id: 7,
      query: "cheapest M4 mac mini",
      depth: "basic",
      asker_id: Some(5),
      options,
      kind: None,
    },
  )
  .await
  .unwrap()
}

#[tokio::test]
async fn a_runs_ending_is_put_under_its_search_and_written_as_its_document() {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let wording = Wording::english().unwrap();
  let options = options(Some("the last month"), Some("domestic"), None, Some("table"), None).unwrap();
  let search_id = opened(&scratch, &options).await;

  join_search(scratch.db(), search_id, "thorough").await.unwrap();

  let written = record_run(
    scratch.db(),
    &Ending {
      search_id,
      run_id: Some(11),
      runs: &[11],
      result: Some(RESULT),
      stream: &stream(),
      outcome: "done",
      artifacts: artifacts.path(),
      wording: &wording,
      zone: chrono_tz::UTC,
    },
  )
  .await
  .unwrap()
  .expect("a document");
  let document = std::fs::read_to_string(&written).unwrap();
  let search = read_search(scratch.db(), search_id, &[11]).await.unwrap().unwrap();

  assert_eq!(written, artifacts.path().join(search_id.to_string()).join("document.md"));
  assert!(document.starts_with("# what to buy\n"));
  assert!(document.contains("- https://joonggo.example/m4 :: price"));
  assert_eq!(search.status, "done");
  assert_eq!(search.ended_depth.as_deref(), Some("thorough"));
  assert_eq!(search.runs, [11]);
  assert_eq!(
    search.options,
    [("region", "domestic"), ("shape", "table"), ("period", "the last month")]
      .map(|(name, value)| (name.to_owned(), value.to_owned()))
  );
  assert_eq!(search.findings.len(), 2);
  assert_eq!(search.findings[0].sources[0].url, "https://joonggo.example/m4");
  // The page the run opened and no finding cites is kept under the search itself.
  assert_eq!(
    search.pages.iter().map(|page| page.url.as_str()).collect::<Vec<_>>(),
    ["https://apple.com/kr/shop/mac"]
  );
  assert_eq!(
    scratch
      .count("SELECT documented::bigint FROM searches WHERE id = $1", vec![search_id.into()])
      .await,
    1
  );
}

#[tokio::test]
async fn a_one_value_answer_keeps_its_row_and_head_and_writes_no_document() {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let wording = Wording::english().unwrap();
  let search_id = opened(&scratch, &Options::default()).await;
  let brief =
    "ANSWERED: yes :: one gram of silver\nTITLE: silver 12,410 won\nDOCUMENT: no\nSUMMARY:\n1. 12,410 won";

  let written = record_run(
    scratch.db(),
    &Ending {
      search_id,
      run_id: Some(3),
      runs: &[3],
      result: Some(brief),
      stream: "",
      outcome: "done",
      artifacts: artifacts.path(),
      wording: &wording,
      zone: chrono_tz::UTC,
    },
  )
  .await
  .unwrap();
  let search = read_search(scratch.db(), search_id, &[3]).await.unwrap().unwrap();

  assert_eq!(written, None);
  assert_eq!(search.head.title.as_deref(), Some("silver 12,410 won"));
  assert!(search.findings.is_empty());
  assert!(search.options.is_empty());
  assert!(!artifacts.path().join(search_id.to_string()).exists());
}

#[tokio::test]
async fn a_search_is_the_searchers_search_unless_it_says_otherwise() {
  let scratch = Scratch::new().await;
  let search_id = opened(&scratch, &Options::default()).await;
  let imported = start_search(
    scratch.db(),
    &NewSearch {
      chat_id: 1,
      query: "import the page: https://c.example",
      depth: "basic",
      asker_id: None,
      options: &Options::default(),
      kind: Some("import"),
    },
  )
  .await
  .unwrap();
  let row = async |id: i64| {
    scratch
      .texts(&format!(
        r#"SELECT unnest(ARRAY["kind", "author", "status"]) FROM "searches" WHERE "id" = {id}"#
      ))
      .await
  };

  assert_eq!(row(search_id).await, ["search", SEARCHER, "open"]);
  assert_eq!(row(imported).await, ["import", SEARCHER, "open"]);
}

#[tokio::test]
async fn a_finished_search_of_the_room_is_reopened_once_and_another_rooms_never() {
  let scratch = Scratch::new().await;
  let search_id = opened(&scratch, &Options::default()).await;

  assert_eq!(reopen_search(scratch.db(), search_id, 7).await.unwrap(), None);

  finish_search(scratch.db(), search_id, "done").await.unwrap();

  assert_eq!(reopen_search(scratch.db(), search_id, 8).await.unwrap(), None);
  assert_eq!(reopen_search(scratch.db(), search_id, 7).await.unwrap().as_deref(), Some("done"));
  assert_eq!(reopen_search(scratch.db(), search_id, 7).await.unwrap(), None);
}
