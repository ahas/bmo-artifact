//! A page's sounds as the page is handed them (`docs/artifact.md`, "An HTML page" and "Readings"; `docs/api.md`,
//! "Readings"): the console's page handed a reading, a share link's page handed its share route, each read
//! answering any origin, the player's script on its own address, the player's words in the reader's language,
//! and a reading token that opens nothing once forged or ended.

use crate::support;

use axum::http::{Method, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use bmo_artifact::readings::{READING_SECONDS, ReadingKey};
use serde_json::json;
use sha2::{Digest, Sha256};
use support::service::{CONSOLE_TOKEN, Service};

const PLAYER: &[u8] = b"window.BmoAudioPlayer = {};";
const WORDS: &[u8] =
  br#"{"en":{"common":{"action":{"play":"Play"}}},"ko":{"common":{"action":{"play":"Play in ko"}}}}"#;

// The page a research run files: its own band under its title, a sound in it by the name it is kept under.
const PAGE: &str = r#"<!doctype html><html><head><title>Read</title><style data-archive>.band{}</style></head><body><h1>Read</h1><section class="band"><p>Listen while you read</p><audio src="attachment:narration-ko.ogg" data-language="ko"></audio></section></body></html>"#;

fn player_name() -> String {
  let digest: String = Sha256::digest(PLAYER).iter().map(|byte| format!("{byte:02x}")).collect();

  format!("bmo-audio-player.{}.js", &digest[..16])
}

async fn service() -> Service {
  Service::with_player(&[("words.json", WORDS), ("bmo-audio-player.js", PLAYER)]).await
}

// A page with its narration attached; the page's number and the sound's.
async fn filed(service: &Service) -> (i64, i64) {
  let id = service
    .call(
      Method::POST,
      "/documents",
      Some(json!({ "kind": "page", "title": "", "html": PAGE, "actorName": "tester", "labels": {} })),
    )
    .await
    .json()["id"]
    .as_i64()
    .expect("a page");
  let sound = service
    .call(
      Method::POST,
      &format!("/documents/{id}/attachments"),
      Some(json!({ "name": "narration-ko.ogg", "mediaType": "audio/ogg", "content": STANDARD.encode(b"OggS sound bytes") })),
    )
    .await
    .json()["id"]
    .as_i64()
    .expect("a sound");

  (id, sound)
}

// The reading token the page's lines hand its sound under.
fn reading_of(page: &str, sound: i64) -> String {
  let tail = format!("/attachments/{sound}\"");
  let end = page.find(&tail).expect("the sound's address");
  let start = page[..end].rfind("readings/").expect("a reading") + "readings/".len();

  page[start..end].to_owned()
}

#[tokio::test]
async fn the_consoles_page_plays_its_sounds_from_a_reading_and_loads_the_player_by_its_digest() {
  let service = service().await;
  let (id, sound) = filed(&service).await;
  let page = service.call(Method::GET, &format!("/documents/{id}/content?scheme=dark"), None).await;
  let text = String::from_utf8(page.bytes.clone()).expect("text");
  let token = reading_of(&text, sound);

  assert_eq!(page.header("cache-control"), "no-store");
  assert!(text.contains(&format!(r#"src="../../player/{}""#, player_name())), "{text}");
  assert!(
    text.contains(&format!(r#""play":"../../readings/{token}/attachments/{sound}""#)),
    "{text}"
  );
  assert!(text.contains(r#""words":{"common":{"action":{"play":"Play"}}}"#), "{text}");
  assert!(text.contains(r#""scheme":"dark""#), "{text}");
  // The band stands as the page wrote it.
  assert!(text.contains(r#"<section class="band"><p>Listen while you read</p><audio src="attachment:narration-ko.ogg" data-language="ko"></audio></section>"#));

  // The sandboxed page reads it with no session, from an origin of its own.
  let read = service
    .call_by(
      Some(CONSOLE_TOKEN),
      Method::GET,
      &format!("/readings/{token}/attachments/{sound}"),
      None,
      &[("range", "bytes=0-3")],
    )
    .await;

  assert_eq!(read.status, StatusCode::PARTIAL_CONTENT);
  assert_eq!(read.bytes, b"OggS");
  assert_eq!(read.header("access-control-allow-origin"), "*");

  let player = service
    .call_by(Some(CONSOLE_TOKEN), Method::GET, &format!("/player/{}", player_name()), None, &[])
    .await;

  assert_eq!(player.status, StatusCode::OK);
  assert_eq!(player.bytes, PLAYER);
  assert!(player.header("content-type").starts_with("text/javascript"));
  assert!(player.header("cache-control").contains("immutable"));
  assert_eq!(
    service
      .call_by(Some(CONSOLE_TOKEN), Method::GET, "/player/bmo-audio-player.js", None, &[])
      .await
      .status,
    StatusCode::NOT_FOUND
  );
}

#[tokio::test]
async fn the_player_speaks_the_language_the_reader_names_else_english() {
  let service = service().await;
  let (id, _) = filed(&service).await;
  let spoken = |language: &'static str| {
    let service = &service;

    async move {
      String::from_utf8(
        service
          .call(Method::GET, &format!("/documents/{id}/content?language={language}"), None)
          .await
          .bytes,
      )
      .expect("text")
    }
  };

  assert!(spoken("ko").await.contains(r#""locale":"ko""#));
  assert!(spoken("ko").await.contains("Play in ko"));
  assert!(spoken("fr").await.contains(r#""locale":"en""#));
}

#[tokio::test]
async fn a_share_links_page_plays_its_sounds_from_the_share_route() {
  let service = service().await;
  let (id, sound) = filed(&service).await;
  let token =
    service.call(Method::POST, "/shares", Some(json!({ "documentId": id }))).await.json()["token"]
      .as_str()
      .expect("a token")
      .to_owned();
  let page = service
    .call_by(Some(CONSOLE_TOKEN), Method::GET, &format!("/shares/{token}/content"), None, &[])
    .await;
  let text = String::from_utf8(page.bytes.clone()).expect("text");

  assert!(
    text.contains(&format!(r#""play":"../../shares/{token}/attachments/{sound}""#)),
    "{text}"
  );
  assert!(!text.contains("readings/"));

  let read = service
    .call_by(
      Some(CONSOLE_TOKEN),
      Method::GET,
      &format!("/shares/{token}/attachments/{sound}"),
      None,
      &[],
    )
    .await;

  assert_eq!(read.status, StatusCode::OK);
  assert_eq!(read.header("access-control-allow-origin"), "*");
}

#[tokio::test]
async fn a_reading_opens_its_one_document_and_nothing_once_forged_or_ended() {
  let service = service().await;
  let (id, sound) = filed(&service).await;
  let (other, elsewhere) = filed(&service).await;
  let key = &service.standing.readings;
  let now = bmo_artifact::readings::now_seconds();
  let token = key.issue(id, now);
  let read = |path: String| {
    let service = &service;

    async move { service.call_by(Some(CONSOLE_TOKEN), Method::GET, &path, None, &[]).await.status }
  };

  assert_eq!(key.opened(&token, now), Some(id));
  assert_eq!(read(format!("/readings/{token}/attachments/{sound}")).await, StatusCode::OK);
  // Another document's file is not the reading's.
  assert_eq!(read(format!("/readings/{token}/attachments/{elsewhere}")).await, StatusCode::NOT_FOUND);
  assert_eq!(key.opened(&key.issue(other, now), now), Some(other));

  // Signed with another key, or changed by a byte, it opens nothing.
  let forged = ReadingKey::of("another service's token").issue(id, now);

  assert_eq!(key.opened(&forged, now), None);
  assert_eq!(read(format!("/readings/{forged}/attachments/{sound}")).await, StatusCode::NOT_FOUND);

  let mut changed = token.clone().into_bytes();

  changed[2] = if changed[2] == b'A' { b'B' } else { b'A' };

  assert_eq!(key.opened(&String::from_utf8(changed).expect("text"), now), None);
  assert_eq!(key.opened("short", now), None);

  // It lasts its twelve hours and no longer.
  assert_eq!(key.opened(&token, now + READING_SECONDS - 1), Some(id));
  assert_eq!(key.opened(&token, now + READING_SECONDS), None);
  assert_eq!(key.opened(&key.issue(id, now - READING_SECONDS), now), None);
}

#[tokio::test]
async fn a_key_drawn_from_the_services_token_outlives_a_restart() {
  assert_eq!(ReadingKey::of("token").opened(&ReadingKey::of("token").issue(5, 100), 100), Some(5));
}
