//! Share links, scopes and the event stream as a caller reaches them (`docs/api.md`, "Share links",
//! "Scopes" and "Events"): a link made once and answered again, read, listed and ended, swept once its time
//! has passed; what a scope holds counted, previewed and let go of; one document let go of alone; and the
//! events each change leaves.

use crate::support;

use axum::http::{Method, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use support::service::{CONSOLE_TOKEN, SHARE_ADDRESS, Service};

async fn filed(service: &Service, title: &str, scope: &str) -> i64 {
  service
    .call(
      Method::POST,
      "/documents",
      Some(json!({ "kind": "record", "title": title, "body": "Words.", "scope": scope, "actorName": "tester", "labels": {} })),
    )
    .await
    .json()["id"]
    .as_i64()
    .expect("an id")
}

#[tokio::test]
async fn a_link_is_made_once_answered_again_read_listed_and_ended() {
  let service = Service::new().await;
  let id = filed(&service, "Shared", "team-a").await;
  let made = service.call(Method::POST, "/shares", Some(json!({ "documentId": id }))).await;

  assert_eq!(made.status, StatusCode::CREATED, "{}", made.json());

  let link = made.json();
  let token = link["token"].as_str().expect("a token").to_owned();

  assert_eq!(link["url"], format!("{SHARE_ADDRESS}/{token}"));
  assert_eq!((link["documentId"].as_i64(), link["expiresAt"].clone()), (Some(id), Value::Null));

  let again = service
    .call(
      Method::POST,
      "/shares",
      Some(json!({ "documentId": id, "expiresAt": 4_102_444_800_000_i64 })),
    )
    .await;

  assert_eq!((again.status, again.json()["token"].as_str()), (StatusCode::OK, Some(token.as_str())));
  assert_eq!(service.call(Method::GET, &format!("/shares/{token}"), None).await.json(), link);
  assert_eq!(
    service.call(Method::GET, &format!("/shares?documentId={id}"), None).await.json()["items"],
    json!([link])
  );
  assert_eq!(
    service.call(Method::DELETE, &format!("/shares/{token}"), None).await.status,
    StatusCode::NO_CONTENT
  );
  // The link no longer opens, and what it opened stands where it ended.
  let ended = service.call(Method::GET, &format!("/shares/{token}"), None).await;

  assert_eq!((ended.status, ended.json()["code"].clone()), (StatusCode::GONE, json!("gone")));
  assert_eq!(
    service.call(Method::DELETE, &format!("/shares/{token}"), None).await.status,
    StatusCode::NOT_FOUND
  );
  assert_eq!(service.event_kinds().await, ["document.created", "share.created", "share.deleted"]);
}

async fn attached(service: &Service, id: i64, name: &str, bytes: &[u8]) -> i64 {
  service
    .call(
      Method::POST,
      &format!("/documents/{id}/attachments"),
      Some(json!({ "name": name, "mediaType": "text/plain", "content": STANDARD.encode(bytes) })),
    )
    .await
    .json()["id"]
    .as_i64()
    .expect("an attachment")
}

#[tokio::test]
async fn a_live_link_opens_its_one_document_through_the_share_reads_with_no_role() {
  let service = Service::new().await;
  let id = filed(&service, "Shared", "team-a").await;
  let other = filed(&service, "Not shared", "team-a").await;
  let kept = attached(&service, id, "notes.txt", b"Kept words.").await;
  let elsewhere = attached(&service, other, "other.txt", b"Other words.").await;
  let token =
    service.call(Method::POST, "/shares", Some(json!({ "documentId": id }))).await.json()["token"]
      .as_str()
      .expect("a token")
      .to_owned();
  // The console relays a share read with no session behind it: its token, and no role and no owner key.
  let read = |path: String| {
    let service = &service;

    async move { service.call_by(Some(CONSOLE_TOKEN), Method::GET, &path, None, &[]).await }
  };

  let document = read(format!("/shares/{token}/document")).await;

  assert_eq!(document.status, StatusCode::OK, "{}", document.json());
  assert_eq!(
    (document.json()["id"].as_i64(), document.json()["title"].clone()),
    (Some(id), json!("Shared"))
  );
  assert_eq!(read(format!("/shares/{token}")).await.json()["documentId"].as_i64(), Some(id));
  assert_eq!(read(format!("/shares/{token}/kind")).await.json()["name"], "record");

  let content = read(format!("/shares/{token}/content")).await;

  assert_eq!((content.status, content.bytes.as_slice()), (StatusCode::OK, b"Words.".as_slice()));

  let files = read(format!("/shares/{token}/attachments")).await.json();

  assert_eq!(files["items"].as_array().map(Vec::len), Some(1));
  assert_eq!(files["items"][0]["id"].as_i64(), Some(kept));

  let file = read(format!("/shares/{token}/attachments/{kept}")).await;

  assert_eq!((file.status, file.bytes.as_slice()), (StatusCode::OK, b"Kept words.".as_slice()));
  assert_eq!(
    read(format!("/shares/{token}/attachments/{elsewhere}")).await.status,
    StatusCode::NOT_FOUND
  );
  assert_eq!(read(format!("/shares/{token}/references")).await.json()["items"], json!([]));
  assert_eq!(read(format!("/shares/{token}/references/1")).await.status, StatusCode::NOT_FOUND);
  assert_eq!(read(format!("/shares/{token}/picture")).await.status, StatusCode::NOT_FOUND);

  // The document's own routes stay behind the role.
  assert_eq!(read(format!("/documents/{id}")).await.status, StatusCode::FORBIDDEN);

  service.call(Method::DELETE, &format!("/shares/{token}"), None).await;

  // What the link opened stands where it ended: the token answers it, and never nothing at all.
  for path in ["", "/document", "/kind", "/content", "/attachments", "/references"] {
    let ended = read(format!("/shares/{token}{path}")).await;

    assert_eq!(
      (ended.status, ended.json()["code"].clone()),
      (StatusCode::GONE, json!("gone")),
      "{path}"
    );
    assert_eq!(
      ended.json()["error"],
      format!("This share link has ended: it opened \"Shared\" (document {id})."),
      "{path}"
    );
  }

  assert_eq!(read(format!("/shares/{}/document", "u".repeat(8))).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_link_is_refused_for_a_time_gone_by_or_no_document_and_swept_once_its_time_passes() {
  let service = Service::new().await;
  let id = filed(&service, "Shared", "team-a").await;

  assert_eq!(
    service
      .call(Method::POST, "/shares", Some(json!({ "documentId": id, "expiresAt": 1_000 })))
      .await
      .status,
    StatusCode::BAD_REQUEST
  );
  assert_eq!(
    service.call(Method::POST, "/shares", Some(json!({ "documentId": 999 }))).await.status,
    StatusCode::NOT_FOUND
  );

  let token = "t".repeat(32);

  service
    .scratch
    .run(
      r#"INSERT INTO "web_search_shares" ("share_token", "search_id", "created_by", "created_at", "expires_at")
         VALUES ($1, $2, '', 1, 2)"#,
      vec![token.clone().into(), id.into()],
    )
    .await;

  let ended = service.call(Method::GET, &format!("/shares/{token}"), None).await;

  assert_eq!((ended.status, ended.json()["code"].clone()), (StatusCode::GONE, json!("gone")));
  assert_eq!(
    ended.json()["error"],
    format!("This share link has ended: it opened \"Shared\" (document {id}).")
  );
  assert_eq!(service.event_kinds().await, ["document.created", "share.deleted"]);
}

#[tokio::test]
async fn a_scope_is_counted_previewed_and_let_go_of_with_its_documents_and_their_files() {
  let service = Service::new().await;
  let kept = filed(&service, "Kept", "team-b").await;
  let gone = filed(&service, "Gone", "team-a").await;

  service.call(Method::POST, "/shares", Some(json!({ "documentId": gone }))).await;

  let counts = json!({ "documents": 1, "attachments": 0, "shares": 1, "imports": 0 });

  assert_eq!(
    service.call(Method::GET, "/scopes/team-a", None).await.json(),
    json!({ "scope": "team-a", "counts": counts, "state": null })
  );
  assert_eq!(
    service.call(Method::DELETE, "/scopes/team-a?dryRun=true", None).await.json()["state"],
    "preview"
  );
  assert_eq!(
    service.call(Method::GET, &format!("/documents/{gone}"), None).await.status,
    StatusCode::OK
  );

  let released = service.call(Method::DELETE, "/scopes/team-a", None).await.json();

  assert_eq!((released["state"].as_str(), released["counts"].clone()), (Some("released"), counts));
  assert_eq!(
    service.call(Method::GET, &format!("/documents/{gone}"), None).await.status,
    StatusCode::NOT_FOUND
  );
  assert!(!service.files.path().join("searches").join(gone.to_string()).exists());
  assert_eq!(
    service.call(Method::GET, &format!("/documents/{kept}"), None).await.status,
    StatusCode::OK
  );
  assert_eq!(service.call(Method::GET, "/scopes/team-a", None).await.status, StatusCode::NOT_FOUND);
  assert_eq!(
    service.event_kinds().await,
    [
      "document.created",
      "document.created",
      "share.created",
      "share.deleted",
      "document.deleted"
    ]
  );
}

#[tokio::test]
async fn one_document_is_let_go_of_with_its_files_and_its_link_and_an_unknown_one_is_not_found() {
  let service = Service::new().await;
  let kept = filed(&service, "Kept", "team-a").await;
  let gone = filed(&service, "Gone", "team-a").await;
  let file = attached(&service, gone, "notes.txt", b"Gone words.").await;
  let token =
    service.call(Method::POST, "/shares", Some(json!({ "documentId": gone }))).await.json()["token"]
      .as_str()
      .expect("a token")
      .to_owned();
  let folder = service.files.path().join("searches").join(gone.to_string());

  assert!(folder.exists());

  let released = service.call(Method::DELETE, &format!("/documents/{gone}"), None).await;

  assert_eq!(released.status, StatusCode::NO_CONTENT);
  assert_eq!(
    service.call(Method::GET, &format!("/documents/{gone}"), None).await.status,
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    service
      .call(Method::GET, &format!("/documents/{gone}/attachments/{file}"), None)
      .await
      .status,
    StatusCode::NOT_FOUND
  );
  assert!(!folder.exists());

  let ended = service.call(Method::GET, &format!("/shares/{token}"), None).await;

  assert_eq!((ended.status, ended.json()["code"].clone()), (StatusCode::GONE, json!("gone")));
  assert_eq!(
    service.call(Method::GET, &format!("/documents/{kept}"), None).await.status,
    StatusCode::OK
  );
  assert_eq!(
    service.call(Method::GET, "/scopes/team-a", None).await.json()["counts"],
    json!({ "documents": 1, "attachments": 0, "shares": 0, "imports": 0 })
  );

  for id in [gone, gone + 1000] {
    let absent = service.call(Method::DELETE, &format!("/documents/{id}"), None).await;

    assert_eq!(
      (absent.status, absent.json()["code"].clone()),
      (StatusCode::NOT_FOUND, json!("not-found")),
      "{id}"
    );
  }

  assert_eq!(
    service.event_kinds().await,
    [
      "document.created",
      "document.created",
      "attachment.created",
      "share.created",
      "share.deleted",
      "document.deleted"
    ]
  );
}

#[tokio::test]
async fn the_stream_ends_when_the_server_is_stopped() {
  let service = Service::new().await;

  filed(&service, "One", "team-a").await;

  assert!(service.stream_ends_on_stop("/events?after=0").await);
}

#[tokio::test]
async fn the_events_are_paged_narrowed_by_kind_and_scope_and_a_resume_before_the_window_is_gone() {
  let service = Service::new().await;

  filed(&service, "One", "team-a").await;
  filed(&service, "Two", "team-b").await;

  let page = |query: &'static str| {
    let service = &service;

    async move {
      service
        .call_with(Method::GET, &format!("/events?{query}"), None, &[("accept", "application/json")])
        .await
    }
  };
  let all = page("").await.json();
  let first = all["items"][0]["id"].as_i64().expect("an id");

  assert_eq!(all["items"][0]["data"]["title"], "One");
  assert_eq!(page("scope=team-b").await.json()["items"].as_array().map(Vec::len), Some(1));
  assert_eq!(page("kinds=share.created").await.json()["items"], json!([]));

  let one = page("limit=1").await.json();

  assert_eq!((one["items"].as_array().map(Vec::len), one["more"].clone()), (Some(1), json!(true)));
  assert_eq!(
    service
      .call_with(Method::GET, &format!("/events?after={first}"), None, &[("accept", "application/json")])
      .await
      .json()["items"][0]["data"]["title"],
    "Two"
  );

  let (status, frame) = service.first_frame(&format!("/events?after={first}"), &[]).await;

  assert_eq!(status, StatusCode::OK);
  assert!(
    frame.contains("event: document.created") && frame.contains("\"title\":\"Two\""),
    "{frame}"
  );

  let (_, frame) = service.first_frame("/events", &[("last-event-id", &first.to_string())]).await;

  assert!(frame.contains(&format!("id: {}", first + 1)), "{frame}");

  service.scratch.run(r#"DELETE FROM "events" WHERE "id" = $1"#, vec![first.into()]).await;

  let gone = page("after=0").await;

  assert_eq!((gone.status, gone.json()["code"].clone()), (StatusCode::GONE, json!("gone")));
}
