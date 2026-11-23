//! The reads (`docs/artifact.md`, "Reads"; `docs/api.md`, "Reads"): a read kept once a day for each reader, an
//! account through the console and a browser through a share link, the documents most read ranked by them and held
//! to what a reader reaches, the documents one reader read last held to that reader's own reads, the week kept and
//! no more, and the reads going with their document.

use crate::support;

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use support::scratch::Scratch;
use support::service::{CONSOLE_TOKEN, Service};

async fn filed(service: &Service, title: &str, owner: Option<&str>) -> i64 {
  let headers: Vec<(&str, &str)> = owner.map(|key| vec![("owner-key", key)]).unwrap_or_default();

  service
    .call_with(
      Method::POST,
      "/documents",
      Some(
        json!({ "kind": "record", "title": title, "body": "Words.", "actorName": "tester", "labels": {} }),
      ),
      &headers,
    )
    .await
    .json()["id"]
    .as_i64()
    .expect("an id")
}

// A read kept through the console for the account named: a member, or the administrator where the account is one of
// the administrators the tests name, who reads the documents with no owner.
async fn read_by(service: &Service, id: i64, account: &str) -> StatusCode {
  let role = if account.starts_with('2') { "administrator" } else { "member" };

  service
    .call_by(
      Some(CONSOLE_TOKEN),
      Method::POST,
      &format!("/documents/{id}/reads"),
      None,
      &[("owner-role", role), ("owner-key", account)],
    )
    .await
    .status
}

// The most read as the reader the headers name reads them: each title and its reads.
async fn most_read(service: &Service, headers: &[(&str, &str)]) -> Vec<(String, i64)> {
  let page: Value = service.call_with(Method::GET, "/document-reads", None, headers).await.json();

  page["items"]
    .as_array()
    .expect("items")
    .iter()
    .map(|item| {
      (
        item["document"]["title"].as_str().unwrap_or_default().to_owned(),
        item["reads"].as_i64().unwrap_or(-1),
      )
    })
    .collect()
}

#[tokio::test]
async fn a_read_is_kept_once_a_day_for_each_reader_and_ranks_the_documents_most_read() {
  let service = Service::new().await;
  let first = filed(&service, "First", None).await;
  let second = filed(&service, "Second", None).await;
  let third = filed(&service, "Third", None).await;

  assert_eq!(most_read(&service, &[]).await, [], "nothing is read yet");

  for (id, account) in [(first, "21"), (first, "21"), (second, "21"), (second, "22"), (third, "22")] {
    assert_eq!(read_by(&service, id, account).await, StatusCode::NO_CONTENT);
  }

  // The second's two readers rank it first; the first, read twice by one reader, ties the third and goes before it
  // as the older one does not.
  assert_eq!(
    most_read(&service, &[]).await,
    [("Second".to_owned(), 2), ("Third".to_owned(), 1), ("First".to_owned(), 1)]
  );

  let limited = service.call(Method::GET, "/document-reads?limit=1", None).await.json();

  assert_eq!(
    (limited["items"].as_array().map(Vec::len), limited["more"].clone()),
    (Some(1), json!(true))
  );
  assert_eq!(
    service.call(Method::GET, "/document-reads?limit=0", None).await.status,
    StatusCode::BAD_REQUEST
  );
  assert_eq!(read_by(&service, 999, "21").await, StatusCode::NOT_FOUND);
}

// The documents read last as the account named reads them through the console: each title.
async fn recent(service: &Service, account: &str) -> Vec<String> {
  let page: Value = service
    .call_by(
      Some(CONSOLE_TOKEN),
      Method::GET,
      "/document-reads/recent",
      None,
      &[("owner-role", "member"), ("owner-key", account)],
    )
    .await
    .json();

  page["items"]
    .as_array()
    .expect("items")
    .iter()
    .map(|item| item["document"]["title"].as_str().unwrap_or_default().to_owned())
    .collect()
}

#[tokio::test]
async fn the_documents_read_last_are_the_readers_own_the_one_opened_last_first() {
  let service = Service::new().await;
  let scratch: &Scratch = &service.scratch;
  let first = filed(&service, "First", Some("41")).await;
  let second = filed(&service, "Second", Some("41")).await;
  filed(&service, "Third", Some("41")).await;
  let elsewhere = filed(&service, "Elsewhere", Some("42")).await;
  let now = chrono::Utc::now().timestamp_millis();
  let today = now.div_euclid(86_400_000);

  assert_eq!(recent(&service, "41").await, Vec::<String>::new(), "nothing is read yet");

  // Two earlier openings today, the second the later; the first is opened again just now through the route.
  for (id, at) in [(first, now - 2_000), (second, now - 1_000)] {
    scratch
      .run(
        r#"INSERT INTO "artifact_reads" ("search_id", "reader", "day", "read_at") VALUES ($1, 'account:41', $2, $3)"#,
        vec![id.into(), today.into(), at.into()],
      )
      .await;
  }
  assert_eq!(recent(&service, "41").await, ["Second", "First"]);

  assert_eq!(read_by(&service, first, "41").await, StatusCode::NO_CONTENT);
  assert_eq!(
    recent(&service, "41").await,
    ["First", "Second"],
    "the same day's opening moves the read to now"
  );
  assert_eq!(
    scratch
      .count(r#"SELECT COUNT(*) FROM "artifact_reads" WHERE "reader" = 'account:41'"#, Vec::new())
      .await,
    2,
    "and keeps no second read"
  );

  // Another reader's reads, of a document both reach and of one only it reaches, never show.
  assert_eq!(read_by(&service, elsewhere, "42").await, StatusCode::NO_CONTENT);
  assert_eq!(recent(&service, "42").await, ["Elsewhere"]);
  assert_eq!(
    recent(&service, "41").await,
    ["First", "Second"],
    "nor does a document unread, the third"
  );

  let limited: Value = service
    .call_by(
      Some(CONSOLE_TOKEN),
      Method::GET,
      "/document-reads/recent?limit=1",
      None,
      &[("owner-role", "member"), ("owner-key", "41")],
    )
    .await
    .json();

  assert_eq!(
    (limited["items"].as_array().map(Vec::len), limited["more"].clone()),
    (Some(1), json!(true))
  );
  assert_eq!(
    service.call(Method::GET, "/document-reads/recent?limit=0", None).await.status,
    StatusCode::BAD_REQUEST
  );
}

#[tokio::test]
async fn an_account_keeps_reads_of_what_it_reaches_and_is_shown_the_most_read_it_reads_alone() {
  let service = Service::new().await;
  let mine = filed(&service, "Mine", Some("31")).await;
  let theirs = filed(&service, "Theirs", Some("32")).await;

  assert_eq!(read_by(&service, mine, "31").await, StatusCode::NO_CONTENT);
  assert_eq!(
    read_by(&service, theirs, "31").await,
    StatusCode::NOT_FOUND,
    "a document it does not reach"
  );
  assert_eq!(read_by(&service, theirs, "32").await, StatusCode::NO_CONTENT);

  assert_eq!(most_read(&service, &[("owner-key", "31")]).await, [("Mine".to_owned(), 1)]);
  assert_eq!(most_read(&service, &[]).await.len(), 2, "a reader of every document sees both");
}

#[tokio::test]
async fn a_share_page_keeps_a_read_for_its_browser_and_a_visitor_of_another_shape_is_refused() {
  let service = Service::new().await;
  let id = filed(&service, "Shared", None).await;
  let token =
    service.call(Method::POST, "/shares", Some(json!({ "documentId": id }))).await.json()["token"]
      .as_str()
      .expect("a token")
      .to_owned();
  let open = |query: &'static str| {
    let service = &service;
    let path = format!("/shares/{token}/document{query}");

    async move { service.call_by(Some(CONSOLE_TOKEN), Method::GET, &path, None, &[]).await.status }
  };

  assert_eq!(open("").await, StatusCode::OK);
  assert_eq!(most_read(&service, &[]).await, [], "a read with no visitor keeps nothing");

  for _ in 0..2 {
    assert_eq!(open("?visitor=browser-one").await, StatusCode::OK);
  }
  assert_eq!(open("?visitor=browser_two").await, StatusCode::OK);
  assert_eq!(most_read(&service, &[]).await, [("Shared".to_owned(), 2)]);

  for shapeless in ["?visitor=short", "?visitor=has%20space1"] {
    assert_eq!(open(shapeless).await, StatusCode::BAD_REQUEST, "{shapeless}");
  }
}

#[tokio::test]
async fn only_the_last_seven_days_are_kept_and_the_reads_go_with_their_document() {
  let service = Service::new().await;
  let id = filed(&service, "Kept", None).await;
  let scratch: &Scratch = &service.scratch;
  let today = chrono::Utc::now().timestamp_millis().div_euclid(86_400_000);

  for (reader, day) in [("account:old", today - 7), ("account:week", today - 6)] {
    scratch
      .run(
        r#"INSERT INTO "artifact_reads" ("search_id", "reader", "day", "read_at") VALUES ($1, $2, $3, $4)"#,
        vec![id.into(), reader.into(), day.into(), (day * 86_400_000).into()],
      )
      .await;
  }

  assert_eq!(
    most_read(&service, &[]).await,
    [("Kept".to_owned(), 1)],
    "the day past the week counts nothing"
  );

  assert_eq!(read_by(&service, id, "24").await, StatusCode::NO_CONTENT);

  assert_eq!(
    scratch
      .count(r#"SELECT COUNT(*) FROM "artifact_reads" WHERE "reader" = 'account:old'"#, Vec::new())
      .await,
    0,
    "keeping a read lets the older ones go"
  );

  service.call(Method::DELETE, &format!("/documents/{id}"), None).await;

  assert_eq!(scratch.count(r#"SELECT COUNT(*) FROM "artifact_reads""#, Vec::new()).await, 0);
}
