//! The documents as a caller reaches them (`docs/api.md`, "Documents", "Attachments" and "References"):
//! filed by kind, listed and searched, read, rewritten, their files sent, their attachments kept and sent,
//! the pages they refer to, and the kinds and readiness the service answers.

mod support;

use axum::http::{Method, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use bmo_artifact::reader::Preview;
use serde_json::{Value, json};
use support::service::Service;

fn record(title: &str, body: &str) -> Value {
  json!({
    "kind": "record",
    "title": title,
    "body": body,
    "scope": "team-a",
    "actorName": "tester",
    "labels": { "topic": "plans" },
  })
}

fn png() -> Vec<u8> {
  let mut png = Vec::new();

  image::RgbaImage::from_pixel(40, 30, image::Rgba([200, 40, 40, 255]))
    .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
    .expect("a png is written");
  png
}

#[tokio::test]
async fn a_record_is_filed_read_listed_and_searched_by_its_words_its_labels_and_its_kind() {
  let service = Service::new().await;
  let filed = service
    .call(
      Method::POST,
      "/documents",
      Some(record("Moving plan", "# Plan\n\nThe boxes go on *Monday*.")),
    )
    .await;

  assert_eq!(filed.status, StatusCode::CREATED, "{}", filed.json());

  let document = filed.json();
  let id = document["id"].as_i64().expect("an id");

  assert_eq!(document["kind"], "record");
  assert_eq!(document["title"], "Moving plan");
  assert_eq!(document["scope"], "team-a");
  assert_eq!(document["actorName"], "tester");
  assert_eq!(document["labels"], json!({ "topic": "plans" }));
  assert_eq!(document["description"], "The boxes go on Monday.");
  assert_eq!(document["attachmentCount"], 0);
  assert!(document["createdAt"].as_i64().is_some_and(|at| at > 1_700_000_000_000));

  assert_eq!(service.call(Method::GET, &format!("/documents/{id}"), None).await.json(), document);

  service
    .call(Method::POST, "/documents", Some(record("Other", "Nothing about it.")))
    .await;

  let listed = |query: &'static str| {
    let service = &service;

    async move {
      let page = service.call(Method::GET, &format!("/documents?{query}"), None).await.json();

      page["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| item["title"].clone())
        .collect::<Vec<_>>()
    }
  };

  assert_eq!(listed("").await, [json!("Other"), json!("Moving plan")]);
  assert_eq!(listed("q=MONDAY").await, [json!("Moving plan")]);
  assert_eq!(listed("label=topic:plans").await.len(), 2);
  assert_eq!(listed("label=topic:other").await.len(), 0);
  assert_eq!(listed("kind=learning").await.len(), 0);
  assert_eq!(listed("scope=team-b").await.len(), 0);

  let first = service.call(Method::GET, "/documents?limit=1", None).await.json();

  assert_eq!(first["more"], true);
  assert_eq!(
    service
      .call(Method::GET, &format!("/documents?before={}", first["items"][0]["id"]), None)
      .await
      .json()["items"][0]["title"],
    "Moving plan"
  );

  let content = service.call(Method::GET, &format!("/documents/{id}/content"), None).await;

  assert_eq!(content.status, StatusCode::OK);
  assert!(content.header("content-type").starts_with("text/markdown"));
  assert_eq!(content.bytes, b"# Plan\n\nThe boxes go on *Monday*.");
}

#[tokio::test]
async fn a_document_is_refused_where_its_input_is_not_the_shape_its_kind_takes() {
  let service = Service::new().await;
  let refused = |body: Value| {
    let service = &service;

    async move { service.call(Method::POST, "/documents", Some(body)).await }
  };

  for body in [
    json!({ "kind": "search", "title": "x", "body": "y", "actorName": "tester", "labels": {} }),
    json!({ "kind": "record", "title": " ", "body": "y", "actorName": "tester", "labels": {} }),
    json!({ "kind": "record", "title": "x", "body": "", "actorName": "tester", "labels": {} }),
    json!({ "kind": "record", "title": "x", "html": "<p>y</p>", "actorName": "tester", "labels": {} }),
    json!({ "kind": "page", "title": "x", "body": "y", "html": "<p>y</p>", "actorName": "tester", "labels": {} }),
    json!({ "kind": "page", "title": "", "html": "<p>no title</p>", "actorName": "tester", "labels": {} }),
    json!({ "kind": "record", "title": "x", "body": "y", "actorName": " ", "labels": {} }),
    json!({ "title": "x" }),
  ] {
    let answered = refused(body.clone()).await;

    assert_eq!(answered.status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(answered.json()["code"], "bad-request");
  }

  let missing = service.call(Method::GET, "/documents/999", None).await;

  assert_eq!(
    (missing.status, missing.json()["code"].clone()),
    (StatusCode::NOT_FOUND, json!("not-found"))
  );
  assert_eq!(service.call(Method::GET, "/documents/x", None).await.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_page_is_filed_as_its_html_and_goes_out_as_a_hosted_page() {
  let service = Service::new().await;
  let html = "<!doctype html><html><head><title>Compare</title><meta name=description content=\"Two plans\"></head><body><h1>Compare</h1></body></html>";
  let filed = service
    .call(
      Method::POST,
      "/documents",
      Some(json!({ "kind": "page", "title": "", "html": html, "actorName": "tester", "labels": {} })),
    )
    .await
    .json();
  let id = filed["id"].as_i64().expect("an id");

  assert_eq!((filed["kind"].as_str(), filed["title"].as_str()), (Some("page"), Some("Compare")));
  assert_eq!(filed["description"], "Two plans");

  let hosted = service.call(Method::GET, &format!("/documents/{id}/content?scheme=light"), None).await;
  let text = String::from_utf8(hosted.bytes.clone()).expect("text");

  assert!(hosted.header("content-security-policy").starts_with("sandbox"));
  assert!(text.starts_with("<!doctype html>") && text.contains("data-archive-sheet"), "{text}");

  let kept = service.call(Method::GET, &format!("/documents/{id}/content?download=true"), None).await;

  assert_eq!(kept.bytes, html.as_bytes());
  assert!(kept.header("content-disposition").starts_with("attachment"));

  let conflict = service
    .call(
      Method::PATCH,
      &format!("/documents/{id}"),
      Some(json!({ "body": "words", "labels": {} })),
    )
    .await;

  assert_eq!(
    (conflict.status, conflict.json()["code"].clone()),
    (StatusCode::CONFLICT, json!("conflict"))
  );
}

#[tokio::test]
async fn a_rewrite_keeps_the_number_and_reads_the_references_again() {
  let service = Service::new().await;
  let id = service
    .call(
      Method::POST,
      "/documents",
      Some(record("Links", "See [one](https://one.example/) and [two](https://two.example/).")),
    )
    .await
    .json()["id"]
    .as_i64()
    .expect("an id");
  let urls = |page: Value| -> Vec<String> {
    page["items"]
      .as_array()
      .expect("items")
      .iter()
      .map(|item| item["url"].as_str().unwrap_or_default().to_owned())
      .collect()
  };

  assert_eq!(
    urls(service.call(Method::GET, &format!("/documents/{id}/references"), None).await.json()),
    ["https://one.example/", "https://two.example/"]
  );

  let changed = service
    .call(
      Method::PATCH,
      &format!("/documents/{id}"),
      Some(json!({ "title": "Links again", "body": "Only [three](https://three.example/).", "labels": { "state": "done" } })),
    )
    .await;

  assert_eq!(changed.status, StatusCode::OK, "{}", changed.json());
  assert_eq!(changed.json()["title"], "Links again");
  assert_eq!(changed.json()["labels"], json!({ "state": "done" }));
  assert_eq!(
    urls(service.call(Method::GET, &format!("/documents/{id}/references"), None).await.json()),
    ["https://three.example/"]
  );
  assert_eq!(service.event_kinds().await, ["document.created", "document.changed"]);
}

#[tokio::test]
async fn an_attachment_is_kept_listed_and_sent_with_its_range_and_its_picture_is_the_cover() {
  let service = Service::new().await;
  let id = service
    .call(Method::POST, "/documents", Some(record("Pictured", "![the plan](plan.png)\n\nWords.")))
    .await
    .json()["id"]
    .as_i64()
    .expect("an id");
  let attach = |name: &'static str,
                media_type: &'static str,
                bytes: Vec<u8>,
                role: Option<&'static str>| {
    let service = &service;

    async move {
      service
        .call(
          Method::POST,
          &format!("/documents/{id}/attachments"),
          Some(json!({ "name": name, "mediaType": media_type, "role": role, "content": STANDARD.encode(bytes) })),
        )
        .await
    }
  };
  let sound = attach("voice.mp3", "audio/mpeg", b"ID3\x04\0\0\0\0\0\0sound".to_vec(), Some("head")).await;

  assert_eq!(sound.status, StatusCode::CREATED, "{}", sound.json());
  assert_eq!(
    (
      sound.json()["kind"].clone(),
      sound.json()["role"].clone(),
      sound.json()["mediaType"].clone()
    ),
    (json!("audio"), json!("head"), json!("audio/mpeg"))
  );

  let picture = attach("plan.png", "image/png", png(), None).await.json();

  assert_eq!(
    (picture["kind"].as_str(), picture["mediaType"].as_str()),
    (Some("image"), Some("image/avif"))
  );

  let again = attach("plan.png", "image/png", png(), None).await;

  assert_eq!(again.status, StatusCode::BAD_REQUEST);

  let listed = service.call(Method::GET, &format!("/documents/{id}/attachments"), None).await.json();

  assert_eq!(listed["items"].as_array().map(Vec::len), Some(2));
  assert_eq!(
    service.call(Method::GET, &format!("/documents/{id}"), None).await.json()["attachmentCount"],
    2
  );

  let sound_id = sound.json()["id"].as_i64().expect("an id");
  let part = service
    .call_with(
      Method::GET,
      &format!("/documents/{id}/attachments/{sound_id}"),
      None,
      &[("range", "bytes=0-2")],
    )
    .await;

  assert_eq!((part.status, part.bytes.as_slice()), (StatusCode::PARTIAL_CONTENT, b"ID3".as_slice()));
  assert_eq!(
    service
      .call(Method::GET, &format!("/documents/{id}/attachments/999"), None)
      .await
      .status,
    StatusCode::NOT_FOUND
  );

  let cover = service.call(Method::GET, &format!("/documents/{id}/picture"), None).await;

  assert_eq!((cover.status, cover.header("content-type")), (StatusCode::OK, "image/jpeg".to_owned()));
  assert_eq!(&cover.bytes[..2], [0xff, 0xd8]);
  assert_eq!(
    service.event_kinds().await,
    ["document.created", "attachment.created", "attachment.created"]
  );
}

#[tokio::test]
async fn a_reference_is_read_with_its_preview_and_its_picture_through_the_reader() {
  let service = Service::new().await;
  let id = service
    .call(
      Method::POST,
      "/documents",
      Some(record("Cited", "From [the source](https://source.example/a).")),
    )
    .await
    .json()["id"]
    .as_i64()
    .expect("an id");

  service.web.previews.lock().expect("the book").insert(
    "https://source.example/a".to_owned(),
    Preview {
      title: Some("Source page".to_owned()),
      description: Some("What the source says".to_owned()),
      image: Some("https://source.example/card.png".to_owned()),
    },
  );
  service.web.pictures.lock().expect("the book").insert(
    "https://source.example/card.png".to_owned(),
    bmo_artifact::reader::Picture {
      bytes: png(),
      media_type: "image/png".to_owned(),
    },
  );

  let listed = service.call(Method::GET, &format!("/documents/{id}/references"), None).await.json();
  let reference = &listed["items"][0];

  // The list answers what is kept, and nothing is kept yet: the words the body wrote stand as the title.
  assert_eq!(
    (reference["title"].as_str(), reference["preview"].clone()),
    (Some("the source"), Value::Null)
  );

  let reference_id = reference["id"].as_i64().expect("an id");
  let read = service
    .call(Method::GET, &format!("/documents/{id}/references/{reference_id}"), None)
    .await
    .json();

  assert_eq!(read["preview"], "What the source says");

  let image = service
    .call(Method::GET, &format!("/documents/{id}/references/{reference_id}/image"), None)
    .await;

  assert_eq!((image.status, image.header("content-type")), (StatusCode::OK, "image/png".to_owned()));
  assert_eq!(
    service.call(Method::GET, &format!("/documents/{id}/references"), None).await.json()["items"][0]["preview"],
    "What the source says"
  );
}

#[tokio::test]
async fn the_kinds_answer_for_themselves_and_the_service_is_ready() {
  let service = Service::new().await;
  let kinds = service.call(Method::GET, "/document-kinds", None).await.json();
  let named: Vec<(&str, bool, i64)> = kinds["items"]
    .as_array()
    .expect("items")
    .iter()
    .map(|kind| {
      (
        kind["name"].as_str().unwrap_or_default(),
        kind["hosted"].as_bool().unwrap_or_default(),
        kind["contentsDepth"].as_i64().unwrap_or(-1),
      )
    })
    .collect();

  assert_eq!(
    named,
    [("record", false, 2), ("learning", false, 3), ("import", false, 3), ("page", true, 0)]
  );
  assert_eq!(
    service.call(Method::GET, "/document-kinds?after=3", None).await.json()["items"][0]["name"],
    "page"
  );

  let health = service.call(Method::GET, "/health", None).await.json();

  assert_eq!(health, json!({ "ready": true, "reason": null, "version": bmo_artifact::VERSION }));
}
