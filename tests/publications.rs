//! Publishing (`docs/api.md`, "Publications"): a document filed and a link with no end to it opened in one
//! call, the link read by a reader with no session behind it, still opening after the service is stood up
//! again, what it leaves where it ends, and the plain refusal of a token that never named one.

use crate::support;

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use support::service::{CONSOLE_TOKEN, SHARE_ADDRESS, Service};

const WORDS: &str = "# Quarterlies\n\nWhat the *quarter* came to.\n";

async fn published(service: &Service) -> Value {
  let published = service
    .call(
      Method::POST,
      "/publications",
      Some(json!({
        "kind": "record",
        "title": "Quarterlies",
        "body": WORDS,
        "description": "What the quarter came to, in a page.",
        "scope": "team-a",
        "actorName": "tester",
        "labels": {},
      })),
    )
    .await;

  assert_eq!(published.status, StatusCode::CREATED, "{}", published.json());

  published.json()
}

// A share read as whoever serves the service to a browser relays it: the console's token, no role, no owner
// key, and no session of the reader's own.
async fn read(service: &Service, path: &str) -> support::service::Answered {
  service.call_by(Some(CONSOLE_TOKEN), Method::GET, path, None, &[]).await
}

#[tokio::test]
async fn publishing_a_markdown_document_answers_its_id_and_a_link_with_no_end() {
  let service = Service::new().await;
  let publication = published(&service).await;
  let document = &publication["document"];
  let id = document["id"].as_i64().expect("an id");
  let token = publication["share"]["token"].as_str().expect("a token").to_owned();

  assert_eq!(
    (document["title"].clone(), document["description"].clone()),
    (json!("Quarterlies"), json!("What the quarter came to, in a page."))
  );
  assert_eq!(publication["share"]["url"], format!("{SHARE_ADDRESS}/{token}"));
  assert_eq!(publication["share"]["documentId"].as_i64(), Some(id));
  assert_eq!(publication["share"]["expiresAt"], Value::Null);
  assert_eq!(
    service
      .call(Method::GET, &format!("/documents/{id}/content"), None)
      .await
      .bytes
      .as_slice(),
    WORDS.as_bytes()
  );
  assert_eq!(service.event_kinds().await, ["document.created", "share.created"]);
}

#[tokio::test]
async fn the_link_opens_the_document_for_a_reader_with_no_session_and_no_role() {
  let service = Service::new().await;
  let publication = published(&service).await;
  let token = publication["share"]["token"].as_str().expect("a token").to_owned();

  let content = read(&service, &format!("/shares/{token}/content")).await;

  assert_eq!((content.status, content.bytes.as_slice()), (StatusCode::OK, WORDS.as_bytes()));
  assert_eq!(
    read(&service, &format!("/shares/{token}")).await.json()["url"],
    format!("{SHARE_ADDRESS}/{token}")
  );
  assert_eq!(
    read(&service, &format!("/shares/{token}/document")).await.json()["title"],
    "Quarterlies"
  );
}

#[tokio::test]
async fn the_link_still_opens_after_the_service_stands_up_again() {
  let mut service = Service::new().await;
  let publication = published(&service).await;
  let token = publication["share"]["token"].as_str().expect("a token").to_owned();

  service.restart();

  let content = read(&service, &format!("/shares/{token}/content")).await;

  assert_eq!((content.status, content.bytes.as_slice()), (StatusCode::OK, WORDS.as_bytes()));
}

#[tokio::test]
async fn the_link_serves_the_newest_body_where_the_document_is_rewritten() {
  let service = Service::new().await;
  let publication = published(&service).await;
  let id = publication["document"]["id"].as_i64().expect("an id");
  let token = publication["share"]["token"].as_str().expect("a token").to_owned();

  service
    .call(
      Method::PATCH,
      &format!("/documents/{id}"),
      Some(json!({ "body": "# Quarterlies\n\nWhat the *quarter* came to, said again.\n", "labels": {} })),
    )
    .await;

  let content = read(&service, &format!("/shares/{token}/content")).await;

  assert_eq!(
    (content.status, content.bytes.as_slice()),
    (StatusCode::OK, b"# Quarterlies\n\nWhat the *quarter* came to, said again.\n".as_slice())
  );
  assert_eq!(read(&service, &format!("/shares/{token}")).await.json()["token"], token.as_str());
}

#[tokio::test]
async fn a_token_that_named_no_link_is_a_plain_404() {
  let service = Service::new().await;
  let answered = read(&service, "/shares/abcdefgh").await;

  assert_eq!(answered.status, StatusCode::NOT_FOUND);
  assert_eq!(answered.json()["code"], "not-found");
  assert_eq!(answered.json()["error"], "no live share link has this token");
}

#[tokio::test]
async fn a_link_that_ended_answers_what_it_opened() {
  let service = Service::new().await;
  let publication = published(&service).await;
  let id = publication["document"]["id"].as_i64().expect("an id");
  let token = publication["share"]["token"].as_str().expect("a token").to_owned();

  assert_eq!(
    service.call(Method::DELETE, &format!("/shares/{token}"), None).await.status,
    StatusCode::NO_CONTENT
  );

  for path in ["", "/document", "/content"] {
    let ended = read(&service, &format!("/shares/{token}{path}")).await;

    assert_eq!(
      (ended.status, ended.json()["code"].clone()),
      (StatusCode::GONE, json!("gone")),
      "{path}"
    );
    assert_eq!(
      ended.json()["error"],
      format!("This share link has ended: it opened \"Quarterlies\" (document {id})."),
      "{path}"
    );
  }
}

#[tokio::test]
async fn a_document_let_go_of_leaves_its_link_answering_what_it_opened() {
  let service = Service::new().await;
  let publication = published(&service).await;
  let id = publication["document"]["id"].as_i64().expect("an id");
  let token = publication["share"]["token"].as_str().expect("a token").to_owned();

  service.call(Method::DELETE, "/scopes/team-a", None).await;

  let ended = read(&service, &format!("/shares/{token}/document")).await;

  assert_eq!((ended.status, ended.json()["code"].clone()), (StatusCode::GONE, json!("gone")));
  assert_eq!(
    ended.json()["error"],
    format!("This share link has ended: it opened \"Quarterlies\" (document {id}).")
  );
}

#[tokio::test]
async fn the_console_does_not_publish() {
  let service = Service::new().await;

  let refused = service
    .call_by(
      Some(CONSOLE_TOKEN),
      Method::POST,
      "/publications",
      Some(
        json!({ "kind": "record", "title": "T", "body": "Words.", "actorName": "tester", "labels": {} }),
      ),
      &[("owner-role", "member")],
    )
    .await;

  assert_eq!(
    (refused.status, refused.json()["code"].clone()),
    (StatusCode::FORBIDDEN, json!("forbidden"))
  );
}
