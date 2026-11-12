//! The share head as a console asks it (`docs/api.md`, "Share links"; `docs/artifact.md`, "Where a person reads
//! it"): what a live link reads as before it is opened, in the bundle's words of the language asked, held to the
//! token alone.

use crate::support;

use axum::http::{Method, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use support::service::{CONSOLE_TOKEN, SHARE_ADDRESS, Service};

const MODULE: &[u8] = b"export default { routes: [] };\n";
const WORDS: &[u8] = br#"{
  "en": { "artifact": {
    "kind": { "search": "Research", "page": "Page" },
    "preview": { "described": "{kind} - {description}", "locale": "en_US", "site_name": "Documents" }
  } },
  "ko": { "artifact": {
    "kind": { "search": "Research-ko", "page": "Page-ko" },
    "preview": { "described": "{kind} / {description}", "locale": "ko_KR", "site_name": "Documents-ko" }
  } }
}"#;

async fn built() -> Service {
  Service::with_console(&[("bundle.js", MODULE), ("words.json", WORDS)]).await
}

fn png() -> Vec<u8> {
  let mut png = Vec::new();

  image::RgbaImage::from_pixel(40, 30, image::Rgba([200, 40, 40, 255]))
    .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
    .expect("a png is written");
  png
}

async fn filed(service: &Service, document: Value) -> i64 {
  let made = service.call(Method::POST, "/documents", Some(document)).await;

  assert_eq!(made.status, StatusCode::CREATED, "{}", made.json());
  made.json()["id"].as_i64().expect("an id")
}

async fn shared(service: &Service, id: i64) -> String {
  service.call(Method::POST, "/shares", Some(json!({ "documentId": id }))).await.json()["token"]
    .as_str()
    .expect("a token")
    .to_owned()
}

// The console asks the head as a share read: its own token, and no role and no owner key.
async fn head(service: &Service, path: &str) -> (StatusCode, Value) {
  let answered = service.call_by(Some(CONSOLE_TOKEN), Method::GET, path, None, &[]).await;

  (answered.status, answered.json())
}

#[tokio::test]
async fn a_research_reads_as_its_title_its_kind_led_description_and_its_picture_in_the_language_asked() {
  let service = built().await;
  let id = filed(
    &service,
    json!({ "kind": "search", "title": "Plan \"one\" <two>", "body": "Words.", "description": "What it found",
            "actorName": "tester", "labels": {} }),
  )
  .await;
  let pictured = service
    .call(
      Method::POST,
      &format!("/documents/{id}/attachments"),
      Some(json!({ "name": "cover.png", "mediaType": "image/png", "content": STANDARD.encode(png()) })),
    )
    .await;

  assert_eq!(pictured.status, StatusCode::CREATED, "{}", pictured.json());

  let token = shared(&service, id).await;
  let (status, english) = head(&service, &format!("/shares/{token}/head")).await;

  assert_eq!(status, StatusCode::OK, "{english}");
  assert_eq!(
    english,
    json!({
      "title": "Plan \"one\" <two>",
      "description": "Research - What it found",
      "image": format!("/shares/{token}/picture"),
      "url": format!("{SHARE_ADDRESS}/{token}"),
      "locale": "en_US",
      "siteName": "Documents",
      "type": "article",
      "kind": "Research"
    })
  );

  let (_, korean) = head(&service, &format!("/shares/{token}/head?language=ko")).await;

  assert_eq!(
    (
      korean["description"].clone(),
      korean["locale"].clone(),
      korean["siteName"].clone(),
      korean["kind"].clone()
    ),
    (
      json!("Research-ko / What it found"),
      json!("ko_KR"),
      json!("Documents-ko"),
      json!("Research-ko")
    )
  );

  let (_, unknown) = head(&service, &format!("/shares/{token}/head?language=fr")).await;

  assert_eq!(unknown, english, "a language the words hold none of reads as English");
}

#[tokio::test]
async fn a_page_drawn_whole_names_no_kind_and_a_document_with_no_picture_no_image() {
  let service = built().await;
  let id = filed(
    &service,
    json!({ "kind": "page", "title": "A page", "html": "<p>Body</p>", "description": "Said of it",
            "actorName": "tester", "labels": {} }),
  )
  .await;
  let token = shared(&service, id).await;
  let (status, read) = head(&service, &format!("/shares/{token}/head")).await;

  assert_eq!(status, StatusCode::OK, "{read}");
  assert_eq!(read["description"], "Said of it");
  assert_eq!((read.get("kind"), read.get("image")), (None, None), "{read}");
  assert_eq!(read["type"], "article");
}

#[tokio::test]
async fn a_token_that_opens_nothing_has_no_head() {
  let service = built().await;
  let id = filed(
    &service,
    json!({ "kind": "record", "title": "Ended", "body": "Words.", "actorName": "tester", "labels": {} }),
  )
  .await;
  let token = shared(&service, id).await;

  service.call(Method::DELETE, &format!("/shares/{token}"), None).await;

  let (never, _) = head(&service, &format!("/shares/{}/head", "u".repeat(8))).await;
  let (ended, said) = head(&service, &format!("/shares/{token}/head")).await;

  assert_eq!(never, StatusCode::NOT_FOUND);
  assert_eq!((ended, said["code"].clone()), (StatusCode::GONE, json!("gone")));
}

#[tokio::test]
async fn a_live_link_has_no_head_where_no_bundle_was_built() {
  let service = Service::new().await;
  let id = filed(
    &service,
    json!({ "kind": "record", "title": "Unbuilt", "body": "Words.", "actorName": "tester", "labels": {} }),
  )
  .await;
  let token = shared(&service, id).await;
  let (status, said) = head(&service, &format!("/shares/{token}/head")).await;

  assert_eq!((status, said["code"].clone()), (StatusCode::NOT_FOUND, json!("not-found")));
}
