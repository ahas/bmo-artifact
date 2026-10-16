//! The tool host as the page reader (`docs/api.md`, "Read a page and fetch a picture"): each read is one
//! `POST /calls` naming the entry and what it is handed, its tool result read back, and every way it can
//! be refused answered with its sentence. The tool host is a stand-in the test serves on a local port.

use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use bmo_artifact::clients::tool_host::{
  Captured, DrawnFigure, PageReader, Raster, ReadPage, ReadReference, Reading, ToolHost,
};
use serde_json::{Value, json};

const PACING: &str = "artifact-7";

// Bytes as a picture would carry them; the client hands them on as they came.
const PICTURE: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

// A stand-in tool host: every call it is asked, and the one answer it gives each.
#[derive(Clone)]
struct FakeHost {
  asked: Arc<Mutex<Vec<Value>>>,
  answer: (StatusCode, Value),
}

async fn called(State(host): State<FakeHost>, Json(asked): Json<Value>) -> (StatusCode, Json<Value>) {
  host.asked.lock().expect("the record").push(asked);

  (host.answer.0, Json(host.answer.1.clone()))
}

impl FakeHost {
  // Listening on a port of its own, answering every call as given; its address, written with a trailing
  // slash as a setting may be.
  async fn start(answer: (StatusCode, Value)) -> (Self, String) {
    let host = Self {
      asked: Arc::default(),
      answer,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = format!("http://{}/", listener.local_addr().expect("an address"));
    let app = axum::Router::new().route("/calls", post(called)).with_state(host.clone());

    tokio::spawn(async move { axum::serve(listener, app).await });

    (host, address)
  }

  fn asked(&self) -> Vec<Value> {
    self.asked.lock().expect("the record").clone()
  }
}

// The tool host at a stand-in answering every call as given, and the stand-in.
async fn served(answer: (StatusCode, Value)) -> (ToolHost, FakeHost) {
  let (host, address) = FakeHost::start(answer).await;

  (ToolHost::new(Some(address), PACING), host)
}

#[tokio::test]
async fn a_picture_is_fetched_with_its_address_and_pacing_and_read_from_the_one_image_block() {
  let (reader, host) = served((
    StatusCode::OK,
    json!({ "content": [
      { "type": "text", "text": "fetched" },
      { "type": "image", "data": STANDARD.encode(PICTURE), "mimeType": "image/png" },
    ] }),
  ))
  .await;

  assert_eq!(
    reader.picture("https://img.example/a.png").await,
    Ok(Raster {
      kind: "image/png".to_owned(),
      bytes: PICTURE.to_vec(),
    })
  );
  assert_eq!(
    host.asked(),
    [json!({
      "tool": "explorer/fetch_raster",
      "arguments": { "address": "https://img.example/a.png", "pacing": PACING },
    })]
  );
}

#[tokio::test]
async fn a_picture_answer_with_no_image_or_bytes_that_are_not_base64_is_refused() {
  let (reader, _) =
    served((StatusCode::OK, json!({ "content": [{ "type": "text", "text": "fetched" }] }))).await;

  assert_eq!(
    reader.picture("https://img.example/a.png").await,
    Err("the picture's answer carries no image".to_owned())
  );

  let (reader, _) = served((
    StatusCode::OK,
    json!({ "content": [{ "type": "image", "data": "not base64!", "mimeType": "image/png" }] }),
  ))
  .await;

  assert!(
    reader
      .picture("https://img.example/a.png")
      .await
      .unwrap_err()
      .starts_with("the picture is not base64: ")
  );
}

#[tokio::test]
async fn a_page_is_imported_with_its_address_and_read_with_each_captured_figure_by_its_number() {
  let (reader, host) = served((
    StatusCode::OK,
    json!({
      "content": [{ "type": "text", "text": "read" }],
      "structuredContent": {
        "page": {
          "url": "https://pages.example/lesson#top",
          "title": "Course book",
          "keywords": ["probability"],
          "body": "SECTION: 1 data",
          "references": [{ "text": "the paper", "url": "https://papers.example/grpo" }],
          "figures": [
            { "n": 1, "caption": "figure 1", "svg": "<svg width=\"1\"/>" },
            { "n": 2, "caption": "figure 2" },
          ],
        },
        "captured": [{ "n": 1, "png": STANDARD.encode(PICTURE) }],
      },
    }),
  ))
  .await;

  assert_eq!(
    reader.page("https://pages.example/lesson").await,
    Ok(Reading {
      page: ReadPage {
        url: "https://pages.example/lesson#top".to_owned(),
        title: "Course book".to_owned(),
        keywords: vec!["probability".to_owned()],
        body: "SECTION: 1 data".to_owned(),
        references: vec![ReadReference {
          text: "the paper".to_owned(),
          url: "https://papers.example/grpo".to_owned(),
        }],
        figures: vec![
          DrawnFigure {
            n: 1,
            caption: "figure 1".to_owned(),
            svg: Some("<svg width=\"1\"/>".to_owned()),
          },
          DrawnFigure {
            n: 2,
            caption: "figure 2".to_owned(),
            svg: None,
          },
        ],
      },
      captured: vec![Captured {
        n: 1,
        png: PICTURE.to_vec(),
      }],
    })
  );
  assert_eq!(
    host.asked(),
    [json!({ "tool": "explorer/import_page", "arguments": { "address": "https://pages.example/lesson" } })]
  );
}

#[tokio::test]
async fn a_page_answer_that_is_no_reading_is_refused() {
  let (reader, _) =
    served((StatusCode::OK, json!({ "content": [{ "type": "text", "text": "read" }] }))).await;

  assert!(
    reader
      .page("https://pages.example/lesson")
      .await
      .unwrap_err()
      .starts_with("the page's answer is not a reading: ")
  );
}

#[tokio::test]
async fn a_result_marked_is_error_is_refused_with_its_text() {
  let (reader, _) = served((
    StatusCode::OK,
    json!({
      "isError": true,
      "content": [
        { "type": "text", "text": "the address resolves inside this network" },
        { "type": "image", "data": STANDARD.encode(PICTURE), "mimeType": "image/png" },
      ],
    }),
  ))
  .await;

  assert_eq!(
    reader.picture("http://10.0.0.1/a.png").await,
    Err("the address resolves inside this network".to_owned())
  );
  assert_eq!(
    reader.page("http://10.0.0.1/").await,
    Err("the address resolves inside this network".to_owned())
  );

  let (reader, _) = served((StatusCode::OK, json!({ "isError": true, "content": [] }))).await;

  assert_eq!(
    reader.picture("https://img.example/a.png").await,
    Err("explorer/fetch_raster failed".to_owned())
  );
}

#[tokio::test]
async fn an_answer_other_than_2xx_is_refused_with_its_error() {
  let (reader, _) = served((
    StatusCode::BAD_GATEWAY,
    json!({ "error": "bmo-explorer is not answering", "code": "unavailable" }),
  ))
  .await;

  assert_eq!(
    reader.picture("https://img.example/a.png").await,
    Err("bmo-explorer is not answering".to_owned())
  );
  assert_eq!(
    reader.page("https://pages.example/lesson").await,
    Err("bmo-explorer is not answering".to_owned())
  );

  let (reader, _) = served((StatusCode::NOT_FOUND, json!("no such entry"))).await;

  assert_eq!(
    reader.picture("https://img.example/a.png").await,
    Err("the tool host answered 404 Not Found".to_owned())
  );
}

#[tokio::test]
async fn with_no_address_set_every_read_is_refused_with_the_settings_sentence() {
  let reader = ToolHost::new(None, PACING);
  let sentence =
    "the tool host's address is not set: set tool_host under [units] in the settings".to_owned();

  assert_eq!(reader.picture("https://img.example/a.png").await, Err(sentence.clone()));
  assert_eq!(reader.page("https://pages.example/lesson").await, Err(sentence));
}

#[tokio::test]
async fn a_call_that_gets_no_answer_is_refused_with_the_failures_own_words() {
  let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
  let address = format!("http://{}", listener.local_addr().expect("an address"));

  drop(listener);

  let refused = ToolHost::new(Some(address), PACING)
    .picture("https://img.example/a.png")
    .await
    .unwrap_err();

  assert!(!refused.is_empty());
}
