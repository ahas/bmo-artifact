//! Share links (`docs/api.md`, "Share links (HTTP)"; `docs/archive.md`, "The share page"): the rows a link
//! is, the routes the owner makes, reads and takes one away by, and the roads a link opens for whoever holds
//! it, against a scratch database and a file root of the test's own. Carried from search-shares.test.ts,
//! share-routes.test.ts and share.test.ts.

mod support;

use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Method, Request, StatusCode};
use bmo_artifact::design::DocumentScheme;
use bmo_artifact::documents::ground::is_jpeg;
use bmo_artifact::documents::preview::PreviewCache;
use bmo_artifact::http::hosted_page::page_prelude;
use bmo_artifact::now_ms;
use bmo_artifact::plumbing::http::{Standing, router};
use bmo_artifact::settings::Settings;
use bmo_artifact::store::shares::{self, Closed, SHARE_SPANS, is_share_token, share_token, span_seconds};
use serde_json::{Value, json};
use support::ROOT;
use support::scratch::Scratch;
use support::units::{self, Book, Units};
use support::web::{FakeWeb, PNG};
use tower::ServiceExt;

const HOUR: i64 = 60 * 60;
const DAY: i64 = 24 * HOUR;
const NOW: i64 = 1_800_000_000;
const ROOM: i64 = 42;

// ---------------------------------------------------------------- the rows a link is

async fn rows(scratch: &Scratch) -> i64 {
  scratch.count("SELECT COUNT(*) FROM web_search_shares", vec![]).await
}

async fn make(scratch: &Scratch, id: i64, span: &str, now: i64) -> Option<shares::Share> {
  shares::create(scratch.db(), id, "tester", now, span_seconds(span).expect("a span"))
    .await
    .unwrap()
}

#[test]
fn a_links_token_is_wide_url_safe_and_never_the_same_twice() {
  let tokens: std::collections::HashSet<String> = (0..50).map(|_| share_token()).collect();

  assert_eq!(tokens.len(), 50);

  for token in &tokens {
    assert!(
      token.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
      "{token}"
    );
    assert!(token.len() >= 43);
    assert!(is_share_token(token));
  }
}

#[test]
fn every_span_says_its_own_length_and_the_endless_one_has_none_rather_than_a_long_one() {
  for (key, seconds) in SHARE_SPANS {
    assert_eq!(span_seconds(key), Some(seconds));
    assert!(seconds.is_none_or(|seconds| seconds > 0));
  }

  assert_eq!(span_seconds("forever"), Some(None));
  assert_eq!(
    SHARE_SPANS
      .iter()
      .filter(|(_, seconds)| seconds.is_none())
      .map(|(key, _)| *key)
      .collect::<Vec<_>>(),
    ["forever"]
  );

  // A key nobody offers is not one.
  for key in ["", "never", "3600000"] {
    assert_eq!(span_seconds(key), None, "{key}");
  }
}

#[tokio::test]
async fn making_a_link_sets_its_expiry_the_span_away_and_an_endless_one_none_in_the_row() {
  let scratch = Scratch::new().await;
  let share = make(&scratch, 7, "6h", NOW).await.unwrap();

  assert_eq!(
    (share.search_id, share.created_by.as_str(), share.created_at, share.expires_at),
    (7, "tester", NOW, Some(NOW + 6 * HOUR))
  );

  let endless = make(&scratch, 8, "forever", NOW).await.unwrap();

  assert_eq!(endless.expires_at, None);
  assert_eq!(
    scratch
      .count(
        "SELECT COUNT(*) FROM web_search_shares WHERE search_id = 8 AND expires_at IS NULL",
        vec![]
      )
      .await,
    1
  );
}

#[tokio::test]
async fn a_document_has_one_live_link_and_another_is_refused_rather_than_replacing_it() {
  let scratch = Scratch::new().await;
  let first = make(&scratch, 7, "1h", NOW).await.unwrap();

  assert_eq!(make(&scratch, 7, "30d", NOW).await, None);
  assert_eq!(
    shares::read(scratch.db(), 7, NOW).await.unwrap().map(|share| share.token),
    Some(first.token.clone())
  );

  // Another document's link is its own.
  let other = make(&scratch, 8, "1h", NOW).await.unwrap();

  assert_ne!(other.token, first.token);
  assert_eq!(shares::read(scratch.db(), 8, NOW).await.unwrap().map(|share| share.search_id), Some(8));
}

#[tokio::test]
async fn reading_a_link_back_gives_none_where_there_is_none_and_what_was_made_otherwise() {
  let scratch = Scratch::new().await;

  assert_eq!(shares::read(scratch.db(), 7, NOW).await.unwrap(), None);

  let made = make(&scratch, 7, "24h", NOW).await.unwrap();

  assert_eq!(shares::read(scratch.db(), 7, NOW).await.unwrap(), Some(made));
}

#[tokio::test]
async fn a_link_whose_time_has_passed_does_not_come_back_its_row_is_gone_and_its_document_is_free_again() {
  let scratch = Scratch::new().await;

  make(&scratch, 7, "1h", NOW).await.unwrap();

  assert!(shares::read(scratch.db(), 7, NOW + HOUR - 1).await.unwrap().is_some());
  assert_eq!(shares::read(scratch.db(), 7, NOW + HOUR).await.unwrap(), None);
  assert_eq!(rows(&scratch).await, 0);

  let next = make(&scratch, 7, "7d", NOW + HOUR).await.unwrap();

  assert_eq!(next.expires_at, Some(NOW + HOUR + 7 * DAY));
}

#[tokio::test]
async fn the_sweep_takes_every_dead_row_and_never_one_with_no_expiry() {
  let scratch = Scratch::new().await;

  make(&scratch, 7, "1h", NOW).await.unwrap();
  make(&scratch, 8, "1h", NOW).await.unwrap();
  make(&scratch, 9, "30d", NOW).await.unwrap();

  assert_eq!(shares::sweep(scratch.db(), NOW + 6 * HOUR).await.unwrap(), 2);
  assert!(shares::read(scratch.db(), 9, NOW + 6 * HOUR).await.unwrap().is_some());
  assert_eq!(shares::sweep(scratch.db(), NOW + 6 * HOUR).await.unwrap(), 0);

  let far = NOW + 100 * 365 * DAY;
  let endless = make(&scratch, 10, "forever", NOW).await.unwrap();

  assert_eq!(shares::sweep(scratch.db(), far).await.unwrap(), 1);
  assert!(shares::read(scratch.db(), 10, far).await.unwrap().is_some());
  // It opens however long after it was made, and is ended by being taken away and by nothing else.
  assert_eq!(shares::opened_by(scratch.db(), &endless.token, far).await.unwrap(), Ok(10));
  assert!(shares::remove(scratch.db(), 10).await.unwrap());
  assert_eq!(
    shares::opened_by(scratch.db(), &endless.token, NOW).await.unwrap(),
    Err(Closed::Unknown)
  );
}

#[tokio::test]
async fn taking_a_link_away_ends_it_at_once_and_touches_no_other_documents() {
  let scratch = Scratch::new().await;

  make(&scratch, 7, "30d", NOW).await.unwrap();
  make(&scratch, 8, "30d", NOW).await.unwrap();

  assert!(shares::remove(scratch.db(), 7).await.unwrap());
  assert_eq!(shares::read(scratch.db(), 7, NOW).await.unwrap(), None);
  assert!(!shares::remove(scratch.db(), 7).await.unwrap());
  assert!(shares::read(scratch.db(), 8, NOW).await.unwrap().is_some());
}

#[tokio::test]
async fn a_token_opens_the_document_it_was_made_for_and_says_why_where_it_opens_none() {
  let scratch = Scratch::new().await;
  let seven = make(&scratch, 7, "1h", NOW).await.unwrap();
  let eight = make(&scratch, 8, "24h", NOW).await.unwrap();
  let opened = |token: String, now: i64| {
    let db = scratch.db().clone();

    async move { shares::opened_by(&db, &token, now).await.unwrap() }
  };

  assert_eq!(opened(seven.token.clone(), NOW).await, Ok(7));
  assert_eq!(opened(eight.token.clone(), NOW).await, Ok(8));
  assert_eq!(opened("not-a-token-anybody-was-given".to_owned(), NOW).await, Err(Closed::Unknown));
  assert_eq!(opened(seven.token.clone(), NOW + HOUR - 1).await, Ok(7));
  assert_eq!(opened(seven.token.clone(), NOW + HOUR).await, Err(Closed::Expired));
  assert_eq!(opened(seven.token.clone(), NOW + 30 * DAY).await, Err(Closed::Expired));

  shares::remove(scratch.db(), 8).await.unwrap();

  assert_eq!(opened(eight.token, NOW).await, Err(Closed::Unknown));
}

// ---------------------------------------------------------------- the archive the routes stand on

struct Desk {
  scratch: Scratch,
  _units: Units,
  root: tempfile::TempDir,
  web: Arc<FakeWeb>,
  settings: HashMap<String, String>,
  first: i64,
  second: i64,
  first_sound: i64,
  second_note: i64,
  stray_note: i64,
  cited: String,
}

async fn returning(scratch: &Scratch, sql: &str, values: Vec<sea_orm::Value>) -> i64 {
  scratch.count(sql, values).await
}

async fn document(
  scratch: &Scratch,
  chat: i64,
  query: &str,
  kind: &str,
  author: &str,
  options: Option<&str>,
) -> i64 {
  returning(
    scratch,
    "INSERT INTO searches (chat_id, query, started_depth, ended_depth, status, started_at, finished_at, options, kind, author)
       VALUES ($1, $2, 'basic', 'basic', 'done', 1, 2, $3, $4, $5) RETURNING id",
    vec![chat.into(), query.into(), options.map(str::to_owned).into(), kind.into(), author.into()],
  )
  .await
}

async fn finding(scratch: &Scratch, id: i64, text: &str) -> i64 {
  returning(
    scratch,
    "INSERT INTO search_findings (search_id, place, text) VALUES ($1, 1, $2) RETURNING id",
    vec![id.into(), text.into()],
  )
  .await
}

impl Desk {
  async fn new() -> Self {
    let scratch = Scratch::new().await;
    let (units, address) = units::serve(Book {
      rooms: vec![(ROOM, "Alpha Room".to_owned())],
      ..Book::default()
    })
    .await;
    let root = tempfile::tempdir().unwrap();
    let first = document(
      &scratch,
      ROOM,
      "where to buy a mini PC",
      "search",
      "searcher",
      Some(r#"{"shape":"brief"}"#),
    )
    .await;
    let one = finding(
      &scratch,
      first,
      "NAME: GEM12\nSee the official store at https://shop.example.com/gem12.\nIMAGE: https://cdn.example.com/p.jpg :: photo of the unit",
    )
    .await;
    let cited = returning(
      &scratch,
      "INSERT INTO search_sources (search_id, finding_id, url, supports, mark) VALUES ($1, $2, 'https://cited.example.com/page', 'price', '1') RETURNING id",
      vec![first.into(), one.into()],
    )
    .await;

    scratch
      .run(
        "INSERT INTO search_sources (search_id, url) VALUES ($1, 'https://opened.example.com/')",
        vec![first.into()],
      )
      .await;
    scratch
      .run(
        r#"UPDATE searches SET title = 'Mini PCs are cheaper at the official store', topic = 'where to buy mini PCs',
             keywords = '["mini PC"]', document = $2 WHERE id = $1"#,
        vec![first.into(), root.path().join(format!("searches/{first}/document.md")).display().to_string().into()],
      )
      .await;

    let second = document(&scratch, 5, "someone else's search", "search", "searcher", None).await;

    finding(&scratch, second, "SUMMARY: This document is someone else's.").await;

    let folder = |id: i64| root.path().join("searches").join(id.to_string()).join("attachments");

    std::fs::create_dir_all(folder(first)).unwrap();
    std::fs::create_dir_all(folder(second)).unwrap();
    std::fs::write(folder(first).join("a-meeting.ogg"), "OggS sound").unwrap();
    std::fs::write(folder(second).join("b-note.txt"), "others' notes").unwrap();
    std::fs::write(root.path().join(format!("searches/{first}/document.md")), "# Search · mini PC\n")
      .unwrap();

    let first_sound = keep(
      &scratch,
      first,
      "audio",
      &folder(first).join("a-meeting.ogg"),
      "audio/ogg",
      "meeting.ogg",
      10,
    )
    .await;
    let second_note = keep(
      &scratch,
      second,
      "text",
      &folder(second).join("b-note.txt"),
      "text/plain",
      "note.txt",
      13,
    )
    .await;
    // A row of the first whose path has wandered into the second's folder, which no road may serve.
    let stray_note = keep(
      &scratch,
      first,
      "text",
      &folder(second).join("b-note.txt"),
      "text/plain",
      "moved note.txt",
      13,
    )
    .await;
    let web = Arc::new(FakeWeb::default());
    let settings = HashMap::from([
      ("BMO_ARTIFACT_DATABASE__URL".to_owned(), scratch.address()),
      ("BMO_ARTIFACT_FILES__ROOT".to_owned(), root.path().display().to_string()),
      ("BMO_ARTIFACT_UNITS__JOB".to_owned(), address.clone()),
      ("BMO_ARTIFACT_UNITS__RECEIVER".to_owned(), address),
    ]);

    Self {
      scratch,
      _units: units,
      root,
      web,
      settings,
      first,
      second,
      first_sound,
      second_note,
      stray_note,
      cited: format!("s{cited}"),
    }
  }

  // A link to one document, made a month long from now, any it had taken away first.
  async fn link(&self, id: i64) -> String {
    let db = self.scratch.db();

    shares::remove(db, id).await.unwrap();
    shares::create(db, id, "tester", now_ms() / 1000, Some(30 * DAY))
      .await
      .unwrap()
      .unwrap()
      .token
  }

  async fn ask(
    &self,
    method: Method,
    path: &str,
    body: Option<Value>,
    headers: &[(&str, &str)],
  ) -> (StatusCode, HeaderMap, Vec<u8>) {
    let settings = Settings::read(Path::new(ROOT), Some(self.settings.clone())).unwrap();
    let mut standing = Standing::new(settings).unwrap();

    standing.previews = Arc::new(PreviewCache::new(self.root.path().join("web-cache"), self.web.clone()));

    let mut request = Request::builder().method(method).uri(path);

    for (name, value) in headers {
      request = request.header(*name, *value);
    }

    let request = match body {
      Some(body) => request.header("content-type", "application/json").body(Body::from(body.to_string())),
      None => request.body(Body::empty()),
    };
    let response = router(standing).unwrap().oneshot(request.unwrap()).await.unwrap();
    let (status, headers) = (response.status(), response.headers().clone());
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec();

    (status, headers, bytes)
  }

  async fn get(&self, path: &str) -> (StatusCode, Value) {
    let (status, _, body) = self.ask(Method::GET, path, None, &[]).await;

    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
  }

  async fn status(&self, path: &str) -> StatusCode {
    self.ask(Method::GET, path, None, &[]).await.0
  }

  async fn page_document(&self, title: &str, html: &str) -> (i64, i64) {
    let id = document(&self.scratch, ROOM, title, "page", "bmo", None).await;
    let folder = self.root.path().join("searches").join(id.to_string()).join("attachments");

    self
      .scratch
      .run("UPDATE searches SET title = $1 WHERE id = $2", vec![title.into(), id.into()])
      .await;
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("page.html"), html).unwrap();

    let page = keep(
      &self.scratch,
      id,
      "page",
      &folder.join("page.html"),
      "text/html",
      "page.html",
      html.len() as i64,
    )
    .await;

    (id, page)
  }
}

async fn keep(
  scratch: &Scratch,
  id: i64,
  media: &str,
  path: &Path,
  kind: &str,
  name: &str,
  bytes: i64,
) -> i64 {
  returning(
    scratch,
    "INSERT INTO archive_attachments (search_id, media, path, type, bytes, name, kept_at) VALUES ($1, $2, $3, $4, $5, $6, 1) RETURNING id",
    vec![id.into(), media.into(), path.display().to_string().into(), kind.into(), bytes.into(), name.into()],
  )
  .await
}

fn header(headers: &HeaderMap, name: &str) -> String {
  headers.get(name).and_then(|value| value.to_str().ok()).unwrap_or_default().to_owned()
}

// ---------------------------------------------------------------- the owner's routes

#[tokio::test]
async fn reading_back_gives_the_link_that_stands_or_nothing() {
  let desk = Desk::new().await;
  let path = format!("/api/archive/{}/share", desk.first);

  assert_eq!(desk.get(&path).await, (StatusCode::OK, json!({ "share": null })));

  let token = desk.link(desk.first).await;
  let (status, body) = desk.get(&path).await;

  assert_eq!(
    (status, &body["share"]["token"], &body["share"]["createdBy"]),
    (StatusCode::OK, &json!(token), &json!("tester"))
  );
}

#[tokio::test]
async fn making_one_takes_a_span_key_and_the_maker_never_a_time_of_the_requests_own() {
  let desk = Desk::new().await;
  let path = format!("/api/archive/{}/share", desk.first);
  let (status, _, body) = desk
    .ask(Method::POST, &path, Some(json!({ "span": "7d", "by": "tester", "expiresAt": 1 })), &[])
    .await;
  let made: Value = serde_json::from_slice(&body).unwrap();
  let share = &made["share"];

  assert_eq!(status, StatusCode::OK);
  assert_eq!((&share["searchId"], &share["createdBy"]), (&json!(desk.first), &json!("tester")));
  assert_eq!(share["expiresAt"].as_i64().unwrap() - share["createdAt"].as_i64().unwrap(), 7 * DAY);
  assert!(is_share_token(share["token"].as_str().unwrap()));
}

// The endless span is a key like any other, and the expiry it leads to is no time at all.
#[tokio::test]
async fn the_endless_span_makes_a_link_with_no_expiry() {
  let desk = Desk::new().await;
  let (_, _, body) = desk
    .ask(
      Method::POST,
      &format!("/api/archive/{}/share", desk.first),
      Some(json!({ "span": "forever", "by": "tester" })),
      &[],
    )
    .await;

  assert_eq!(serde_json::from_slice::<Value>(&body).unwrap()["share"]["expiresAt"], Value::Null);
}

#[tokio::test]
async fn a_span_nobody_offers_or_no_maker_is_refused_before_anything_is_read_or_made() {
  let desk = Desk::new().await;

  for body in [
    json!({ "span": "never", "by": "tester" }),
    json!({ "span": "", "by": "tester" }),
    json!({ "span": 3_600_000, "by": "tester" }),
    json!({ "span": null, "by": "tester" }),
    json!({ "by": "tester" }),
    json!({}),
    json!(null),
  ] {
    let (status, _, refused) =
      desk.ask(Method::POST, "/api/archive/9999/share", Some(body.clone()), &[]).await;

    assert_eq!(
      (status, serde_json::from_slice::<Value>(&refused).unwrap()["code"].clone()),
      (StatusCode::BAD_REQUEST, json!("bad_expiry")),
      "{body}"
    );
  }

  let (status, _, refused) = desk
    .ask(Method::POST, "/api/archive/9999/share", Some(json!({ "span": "1h", "by": " " })), &[])
    .await;

  assert_eq!(
    (status, serde_json::from_slice::<Value>(&refused).unwrap()["code"].clone()),
    (StatusCode::BAD_REQUEST, json!("bad_maker"))
  );
  assert_eq!(rows(&desk.scratch).await, 0);
}

#[tokio::test]
async fn a_document_that_is_not_there_or_kept_nothing_is_given_no_link() {
  let desk = Desk::new().await;
  let ask = |id: i64| {
    let desk = &desk;

    async move {
      desk
        .ask(
          Method::POST,
          &format!("/api/archive/{id}/share"),
          Some(json!({ "span": "1h", "by": "tester" })),
          &[],
        )
        .await
    }
  };
  let code = |body: Vec<u8>| serde_json::from_slice::<Value>(&body).unwrap()["code"].clone();
  let (status, _, body) = ask(9999).await;

  assert_eq!((status, code(body)), (StatusCode::NOT_FOUND, json!("document_not_found")));

  // A question after one value kept no document, so a link would hand out an empty page.
  desk
    .scratch
    .run("UPDATE searches SET documented = 0 WHERE id = $1", vec![desk.second.into()])
    .await;

  let (status, _, body) = ask(desk.second).await;

  assert_eq!((status, code(body)), (StatusCode::CONFLICT, json!("nothing_to_share")));
  assert_eq!(rows(&desk.scratch).await, 0);
}

#[tokio::test]
async fn a_document_number_that_is_not_one_is_refused_on_every_road() {
  let desk = Desk::new().await;

  for id in ["abc", "0"] {
    let path = format!("/api/archive/{id}/share");

    for method in [Method::GET, Method::POST, Method::DELETE] {
      let body = (method == Method::POST).then(|| json!({ "span": "1h", "by": "tester" }));

      assert_eq!(
        desk.ask(method.clone(), &path, body, &[]).await.0,
        StatusCode::BAD_REQUEST,
        "{method} {path}"
      );
    }
  }
}

#[tokio::test]
async fn a_second_link_is_refused_while_one_stands_and_taking_one_away_says_whether_there_was_one() {
  let desk = Desk::new().await;
  let path = format!("/api/archive/{}/share", desk.first);
  let (_, _, first) =
    desk.ask(Method::POST, &path, Some(json!({ "span": "1h", "by": "tester" })), &[]).await;
  let token = serde_json::from_slice::<Value>(&first).unwrap()["share"]["token"].clone();
  let (status, _, _) =
    desk.ask(Method::POST, &path, Some(json!({ "span": "30d", "by": "tester" })), &[]).await;

  assert_eq!(status, StatusCode::CONFLICT);
  assert_eq!(desk.get(&path).await.1["share"]["token"], token);

  for removed in [true, false] {
    let (status, _, body) = desk.ask(Method::DELETE, &path, None, &[]).await;

    assert_eq!(
      (status, serde_json::from_slice::<Value>(&body).unwrap()),
      (StatusCode::OK, json!({ "removed": removed }))
    );
  }

  assert_eq!(rows(&desk.scratch).await, 0);
}

// ---------------------------------------------------------------- a link that was made

#[tokio::test]
async fn a_link_opens_the_document_it_was_made_for_not_to_be_stored() {
  let desk = Desk::new().await;
  let token = desk.link(desk.first).await;
  let (status, headers, body) = desk.ask(Method::GET, &format!("/api/share/{token}"), None, &[]).await;
  let shared: Value = serde_json::from_slice(&body).unwrap();

  assert_eq!(status, StatusCode::OK);
  assert_eq!(shared["title"], "Mini PCs are cheaper at the official store");
  assert_eq!(shared["findings"].as_array().unwrap().len(), 1);
  assert_eq!(header(&headers, "cache-control"), "private, no-store");
}

#[tokio::test]
async fn its_refs_are_answered_under_the_token_with_the_address_read_from_the_record() {
  let desk = Desk::new().await;
  let token = desk.link(desk.first).await;

  desk
    .status(&format!("/api/share/{token}/refs/{}/preview?url=http://127.0.0.1/", desk.cited))
    .await;
  desk
    .status(&format!(
      "/api/share/{token}/refs/i1.1/image?as=http://evil&url=http://169.254.169.254/"
    ))
    .await;

  assert_eq!(
    desk.web.taken(),
    ["preview https://cited.example.com/page", "image https://cdn.example.com/p.jpg"]
  );
}

#[tokio::test]
async fn a_picture_goes_out_as_the_type_its_bytes_are_not_sniffed_sandboxed_and_privately_cached() {
  let desk = Desk::new().await;
  let token = desk.link(desk.first).await;
  let (status, headers, body) =
    desk.ask(Method::GET, &format!("/api/share/{token}/refs/i1.1/image"), None, &[]).await;

  assert_eq!((status, body.as_slice()), (StatusCode::OK, PNG.as_slice()));
  assert_eq!(
    [
      "content-type",
      "content-length",
      "x-content-type-options",
      "content-security-policy",
      "cache-control"
    ]
    .map(|name| header(&headers, name)),
    ["image/png", "8", "nosniff", "default-src 'none'; sandbox", "private, max-age=86400"]
      .map(str::to_owned)
  );
}

#[tokio::test]
async fn the_answer_carries_no_state_room_question_options_rule_depth_times_or_file() {
  let desk = Desk::new().await;
  let token = desk.link(desk.first).await;
  let shared = desk.get(&format!("/api/share/{token}")).await.1;
  let mut keys: Vec<&str> = shared.as_object().unwrap().keys().map(String::as_str).collect();

  keys.sort_unstable();
  assert_eq!(keys, ["attachments", "findings", "kind", "opened", "runCount", "title"]);

  let written = shared.to_string();

  for absent in [
    "status",
    "chatId",
    "room",
    "query",
    "options",
    "rule",
    "Depth",
    "startedAt",
    "finishedAt",
    "document",
  ] {
    assert!(!written.contains(absent), "{absent}");
  }

  for absent in ["where to buy a mini PC", "Alpha Room", "done", "basic", "brief"] {
    assert!(!written.contains(absent), "{absent}");
  }

  // There is no road at all to the document's own file under a link.
  assert_eq!(desk.status(&format!("/api/share/{token}/document")).await, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------- a link that opens nothing

#[tokio::test]
async fn one_whose_time_has_passed_says_so_and_fetches_nothing() {
  let desk = Desk::new().await;
  let token = desk.link(desk.first).await;

  assert_eq!(desk.status(&format!("/api/share/{token}")).await, StatusCode::OK);

  desk
    .scratch
    .run("UPDATE web_search_shares SET expires_at = $1", vec![(now_ms() / 1000 - HOUR).into()])
    .await;

  for path in [
    String::new(),
    format!("/refs/{}/preview", desk.cited),
    "/refs/i1.1/image".to_owned(),
    "/head".to_owned(),
  ] {
    let (status, body) = desk.get(&format!("/api/share/{token}{path}")).await;

    assert_eq!((status, &body["code"]), (StatusCode::GONE, &json!("share_expired")), "{path}");
  }

  assert!(desk.web.taken().is_empty());
}

#[tokio::test]
async fn a_token_that_names_no_link_was_taken_away_or_is_not_shaped_like_one_opens_nothing() {
  let desk = Desk::new().await;
  let token = desk.link(desk.first).await;
  let nobodys = "a".repeat(43);

  for path in ["", "/refs/s1/preview", "/refs/i1.1/image", "/attachments/1", "/head", "/picture"] {
    assert_eq!(
      desk.status(&format!("/api/share/{nobodys}{path}")).await,
      StatusCode::NOT_FOUND,
      "{path}"
    );
  }

  for shaped in ["%2E%2E%2Fsearches%2F1", "1", &"a".repeat(200), "tok%20en"] {
    assert_eq!(desk.get(&format!("/api/share/{shaped}")).await.1["code"], "share_not_found", "{shaped}");
  }

  shares::remove(desk.scratch.db(), desk.first).await.unwrap();

  assert_eq!(desk.status(&format!("/api/share/{token}")).await, StatusCode::NOT_FOUND);
  assert!(desk.web.taken().is_empty());
}

// ---------------------------------------------------------------- one link, one document

#[tokio::test]
async fn a_link_to_one_document_never_answers_with_anothers_whatever_the_request_says() {
  let desk = Desk::new().await;
  let mine = desk.link(desk.second).await;
  let first = desk.first;

  assert_eq!(
    desk.get(&format!("/api/share/{mine}?id={first}&searchId={first}")).await.1["title"],
    "someone else's search"
  );
  // The first document's own source, asked for under the second's link, is looked for in the second's
  // record, where it is not.
  assert_eq!(
    desk.status(&format!("/api/share/{mine}/refs/{}/preview", desk.cited)).await,
    StatusCode::NOT_FOUND
  );
  assert!(desk.web.taken().is_empty());
}

// ---------------------------------------------------------------- what a link reads as before it is opened

#[tokio::test]
async fn the_head_is_the_title_the_opening_prose_past_the_name_line_and_a_picture_under_the_links_road() {
  let desk = Desk::new().await;
  let token = desk.link(desk.first).await;
  let (status, headers, body) = desk.ask(Method::GET, &format!("/api/share/{token}/head"), None, &[]).await;

  assert_eq!(status, StatusCode::OK);
  assert_eq!(
    serde_json::from_slice::<Value>(&body).unwrap(),
    json!({
      "title": "Mini PCs are cheaper at the official store",
      "description": "See the official store at https://shop.example.com/gem12.",
      "image": format!("api/share/{token}/picture"),
      "declared": null,
    })
  );
  assert_eq!(header(&headers, "cache-control"), "private, no-store");
}

#[tokio::test]
async fn a_document_with_no_picture_names_none_and_its_picture_road_opens_nothing() {
  let desk = Desk::new().await;
  let token = desk.link(desk.second).await;

  assert_eq!(
    desk.get(&format!("/api/share/{token}/head")).await.1,
    json!({ "title": "someone else's search", "description": "This document is someone else's.", "image": null, "declared": null })
  );
  assert_eq!(desk.get(&format!("/api/share/{token}/picture")).await.1["code"], "image_unavailable");
}

#[tokio::test]
async fn a_picture_the_document_keeps_goes_out_as_a_jpeg_a_preview_card_draws_under_the_picture_guards() {
  let desk = Desk::new().await;
  let pictured = document(&desk.scratch, ROOM, "picture", "record", "bmo", None).await;
  let folder = desk.root.path().join("searches").join(pictured.to_string()).join("attachments");
  let mut png = Cursor::new(Vec::new());

  image::RgbaImage::from_pixel(4, 3, image::Rgba([255, 0, 0, 128]))
    .write_to(&mut png, image::ImageFormat::Png)
    .unwrap();
  std::fs::create_dir_all(&folder).unwrap();
  std::fs::write(folder.join("p-picture.png"), png.get_ref()).unwrap();
  keep(
    &desk.scratch,
    pictured,
    "image",
    &folder.join("p-picture.png"),
    "image/png",
    "picture.png",
    png.get_ref().len() as i64,
  )
  .await;

  let token = desk.link(pictured).await;

  assert_eq!(
    desk.get(&format!("/api/share/{token}/head")).await.1["image"],
    format!("api/share/{token}/picture")
  );

  let (status, headers, card) =
    desk.ask(Method::GET, &format!("/api/share/{token}/picture"), None, &[]).await;
  let drawn = image::load_from_memory(&card).unwrap();

  assert_eq!(status, StatusCode::OK);
  assert!(is_jpeg(&card));
  assert_eq!((drawn.width(), drawn.height()), (4, 3));
  assert_eq!(
    ["content-type", "x-content-type-options", "content-security-policy"]
      .map(|name| header(&headers, name)),
    ["image/jpeg", "nosniff", "default-src 'none'; sandbox"].map(str::to_owned)
  );
}

#[tokio::test]
async fn a_link_to_a_page_carries_the_tags_the_page_declares_itself() {
  let desk = Desk::new().await;
  let page = r#"<!doctype html><title>Canvas</title><meta property="og:image" content="https://example.com/c.png"><p>two plans &amp; side by side</p>"#;
  let (canvas, _) = desk.page_document("Canvas", page).await;
  let token = desk.link(canvas).await;
  let head = desk.get(&format!("/api/share/{token}/head")).await.1;

  assert_eq!(head["title"], "Canvas");
  assert_eq!(
    head["declared"],
    json!({ "title": "Canvas", "meta": [{ "property": "og:image", "content": "https://example.com/c.png" }], "canonical": null })
  );
}

// ---------------------------------------------------------------- the files a shared document keeps

#[tokio::test]
async fn stand_in_the_answer_by_number_name_kind_and_size_never_by_where_they_are_kept() {
  let desk = Desk::new().await;
  let token = desk.link(desk.first).await;
  let shared = desk.get(&format!("/api/share/{token}")).await.1;

  assert_eq!(
    shared["attachments"],
    json!([
      { "id": desk.first_sound, "media": "audio", "type": "audio/ogg", "bytes": 10, "name": "meeting.ogg" },
      { "id": desk.stray_note, "media": "text", "type": "text/plain", "bytes": 13, "name": "moved note.txt" },
    ])
  );
  assert!(!shared.to_string().contains(&desk.root.path().display().to_string()));
}

#[tokio::test]
async fn one_of_its_own_is_served_under_its_link_never_cached_past_the_link_never_run() {
  let desk = Desk::new().await;
  let token = desk.link(desk.first).await;
  let path = format!("/api/share/{token}/attachments/{}", desk.first_sound);
  let (status, headers, body) = desk.ask(Method::GET, &path, None, &[]).await;

  assert_eq!((status, body.as_slice()), (StatusCode::OK, b"OggS sound".as_slice()));
  assert_eq!(
    ["cache-control", "content-disposition", "x-content-type-options", "referrer-policy"]
      .map(|name| header(&headers, name)),
    ["private, no-store", "inline", "nosniff", "no-referrer"].map(str::to_owned)
  );
  assert!(header(&headers, "content-security-policy").contains("script-src 'none'"));

  // Asked to be saved, it is handed over as a file.
  let (_, headers, _) = desk.ask(Method::GET, &format!("{path}?download=1"), None, &[]).await;

  assert_eq!(header(&headers, "content-disposition"), "attachment; filename*=UTF-8''meeting.ogg");
}

// A page opens as a page, with its own scripts, but sandboxed into an origin of its own, so what it runs
// holds nothing of the console's and can send nothing anywhere.
#[tokio::test]
async fn an_html_page_runs_as_a_page_in_a_sandbox_of_its_own_origin_with_nowhere_to_send_anything() {
  let desk = Desk::new().await;
  let html = r##"<!DOCTYPE html><title>t</title><button onclick="navigator.clipboard.writeText('#FEFEFE')">x</button>"##;
  let (page, file) = desk.page_document("t", html).await;
  let token = desk.link(page).await;
  let path = format!("/api/share/{token}/attachments/{file}");
  let (status, headers, body) = desk.ask(Method::GET, &path, None, &[]).await;
  let policy = header(&headers, "content-security-policy");
  let sent = String::from_utf8(body).unwrap();

  assert_eq!(status, StatusCode::OK);
  assert!(policy.starts_with("sandbox allow-scripts "), "{policy}");
  assert!(!policy.contains("allow-same-origin"));

  for part in ["script-src 'unsafe-inline'", "connect-src 'none'", "form-action 'none'"] {
    assert!(policy.contains(part), "{part}");
  }

  // What goes out is the page with its prelude ahead of its first element, in the scheme of a reader who never
  // chose one; saved, it is the file.
  assert!(
    sent
      .starts_with(&format!("<!DOCTYPE html>{}<title>t</title>", page_prelude(html, DocumentScheme::Dark))),
    "{sent}"
  );
  assert_eq!(
    ["content-type", "cache-control", "x-content-type-options", "referrer-policy"]
      .map(|name| header(&headers, name)),
    ["text/html; charset=utf-8", "private, no-store", "nosniff", "no-referrer"].map(str::to_owned)
  );

  let (_, headers, kept) = desk.ask(Method::GET, &format!("{path}?download=1"), None, &[]).await;

  assert_eq!(kept, html.as_bytes());
  assert!(header(&headers, "content-disposition").starts_with("attachment;"));
}

// A link names its document, and a number is looked up under that document alone.
#[tokio::test]
async fn a_link_does_not_reach_another_documents_file_nor_one_kept_outside_its_own_folder() {
  let desk = Desk::new().await;
  let (first, second) = (desk.link(desk.first).await, desk.link(desk.second).await);

  assert_eq!(
    desk.status(&format!("/api/share/{first}/attachments/{}", desk.second_note)).await,
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    desk.status(&format!("/api/share/{second}/attachments/{}", desk.first_sound)).await,
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    desk.status(&format!("/api/share/{second}/attachments/{}", desk.second_note)).await,
    StatusCode::OK
  );
  // The path is the archive's, and a row whose path has wandered into another document's folder is not
  // served even under its own document's link.
  assert_eq!(
    desk.status(&format!("/api/share/{first}/attachments/{}", desk.stray_note)).await,
    StatusCode::NOT_FOUND
  );
}

#[tokio::test]
async fn a_number_that_is_no_number_or_a_link_that_opens_nothing_reaches_no_file() {
  let desk = Desk::new().await;
  let token = desk.link(desk.first).await;

  assert_eq!(
    desk.get(&format!("/api/share/{token}/attachments/0")).await.1["code"],
    "bad_attachment_id"
  );
  assert_eq!(
    desk
      .get(&format!(
        "/api/share/no-such-token-at-all-000000000000/attachments/{}",
        desk.first_sound
      ))
      .await
      .1["code"],
    "share_not_found"
  );
}
