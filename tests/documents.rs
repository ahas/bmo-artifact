//! The documents as a caller reaches them (`docs/api.md`, "Documents", "Attachments" and "References"):
//! filed by kind, listed and searched, read, rewritten, their files sent, their attachments kept and sent,
//! the pages they refer to, and the kinds and readiness the service answers.

use crate::support;

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
  png_in([200, 40, 40])
}

fn png_in([red, green, blue]: [u8; 3]) -> Vec<u8> {
  let mut png = Vec::new();

  image::RgbaImage::from_pixel(40, 30, image::Rgba([red, green, blue, 255]))
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
  assert_eq!(document["hasPicture"], false);
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

  for phrase in [format!("{id}"), format!("%23{id}")] {
    let page = service.call(Method::GET, &format!("/documents?q={phrase}"), None).await.json();

    assert_eq!(page["items"][0]["title"], "Moving plan", "{phrase}");
    assert_eq!(page["items"].as_array().map(Vec::len), Some(1), "{phrase}");
  }

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
async fn a_description_the_filer_gives_stands_over_the_one_read_from_the_body() {
  let service = Service::new().await;
  let mut body = record("Rates", "# Rates\n\nThe won closed at 1,323.1 to the dollar.");
  body["description"] = json!("  Today's won/dollar closing rate  ");
  let filed = service.call(Method::POST, "/documents", Some(body)).await;

  assert_eq!(filed.status, StatusCode::CREATED, "{}", filed.json());
  assert_eq!(filed.json()["description"], "Today's won/dollar closing rate");

  let mut blank = record("Rates again", "# Rates\n\nThe won closed at 1,323.1 to the dollar.");
  blank["description"] = json!("   ");
  let filed = service.call(Method::POST, "/documents", Some(blank)).await;

  assert_eq!(filed.status, StatusCode::CREATED, "{}", filed.json());
  assert_eq!(filed.json()["description"], "The won closed at 1,323.1 to the dollar.");
}

#[tokio::test]
async fn a_document_is_refused_where_its_input_is_not_the_shape_its_kind_takes() {
  let service = Service::new().await;
  let refused = |body: Value| {
    let service = &service;

    async move { service.call(Method::POST, "/documents", Some(body)).await }
  };

  for body in [
    json!({ "kind": "nobody-knows", "title": "x", "body": "y", "actorName": "tester", "labels": {} }),
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

  let pictured = || {
    let service = &service;

    async move {
      let listed = service.call(Method::GET, "/documents?q=Pictured", None).await.json();
      let only_pictured =
        service.call(Method::GET, "/documents?q=Pictured&picture=true", None).await.json();

      (
        service.call(Method::GET, &format!("/documents/{id}"), None).await.json()["hasPicture"].clone(),
        listed["items"][0]["hasPicture"].clone(),
        json!(only_pictured["items"].as_array().is_some_and(|items| !items.is_empty())),
      )
    }
  };

  assert_eq!(
    pictured().await,
    (json!(false), json!(false), json!(false)),
    "a sound is no picture, and the pictured list holds no such document"
  );

  let picture = attach("plan.png", "image/png", png(), None).await.json();

  assert_eq!(
    (picture["kind"].as_str(), picture["mediaType"].as_str()),
    (Some("image"), Some("image/avif"))
  );

  let again = attach("plan.png", "image/png", png(), None).await;

  assert_eq!(again.status, StatusCode::BAD_REQUEST);
  assert_eq!(
    pictured().await,
    (json!(true), json!(true), json!(true)),
    "the document and the list say it keeps a picture, and the pictured list holds it"
  );

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
async fn a_picture_the_body_draws_by_a_path_ending_in_its_name_is_the_cover() {
  let service = Service::new().await;
  let body =
    "![a figure](/elsewhere/searches/1/images/second.png) and ![away](https://example.com/first.png)";
  let id = service
    .call(Method::POST, "/documents", Some(record("Pictured by path", body)))
    .await
    .json()["id"]
    .as_i64()
    .expect("an id");

  for (name, colour) in [("first.png", [200, 40, 40]), ("second.png", [40, 40, 200])] {
    let kept = service
      .call(
        Method::POST,
        &format!("/documents/{id}/attachments"),
        Some(json!({ "name": name, "mediaType": "image/png", "content": STANDARD.encode(png_in(colour)) })),
      )
      .await;

    assert_eq!(kept.status, StatusCode::CREATED, "{}", kept.json());
  }

  let cover = service.call(Method::GET, &format!("/documents/{id}/picture"), None).await;

  assert_eq!(cover.status, StatusCode::OK);

  let drawn = image::load_from_memory(&cover.bytes).expect("a picture").to_rgb8();
  let image::Rgb([red, _, blue]) = *drawn.get_pixel(drawn.width() / 2, drawn.height() / 2);

  assert!(blue > red, "the cover is the picture the body draws by its path, not the first kept");
}

#[tokio::test]
async fn a_row_that_left_no_document_is_neither_listed_nor_read() {
  let service = Service::new().await;
  let filed = service
    .call(Method::POST, "/documents", Some(record("Kept", "Words kept.")))
    .await
    .json()["id"]
    .as_i64()
    .expect("an id");

  // A research run that failed with no mark of its own, and one marked as having written nothing.
  for (status, documented) in [("failed", None::<i64>), ("done", Some(0))] {
    service
      .scratch
      .run(
        r#"INSERT INTO "searches" ("query", "started_depth", "status", "started_at", "documented", "kind")
           VALUES ('A run that found nothing', 'search', $1, 1700000000, $2, 'search')"#,
        vec![status.into(), documented.into()],
      )
      .await;
  }

  let listed = service.call(Method::GET, "/documents", None).await.json();
  let ids: Vec<i64> = listed["items"]
    .as_array()
    .expect("a page")
    .iter()
    .filter_map(|item| item["id"].as_i64())
    .collect();

  assert_eq!(ids, [filed]);

  for left in [filed + 1, filed + 2] {
    for route in ["", "/content", "/picture", "/attachments", "/references"] {
      let answered = service.call(Method::GET, &format!("/documents/{left}{route}"), None).await;

      assert_eq!(answered.status, StatusCode::NOT_FOUND, "/documents/{left}{route}");
    }
  }

  assert_eq!(
    service.call(Method::GET, &format!("/documents/{filed}/content"), None).await.status,
    StatusCode::OK
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
async fn a_labels_values_stand_most_carried_first() {
  let service = Service::new().await;

  for (title, category) in [("One", "audit"), ("Two", "patch"), ("Three", "audit"), ("Four", "")] {
    let mut filed = record(title, "Words.");

    if !category.is_empty() {
      filed["labels"]["category"] = json!(category);
    }

    service.call(Method::POST, "/documents", Some(filed)).await;
  }

  assert_eq!(
    service.call(Method::GET, "/label-values?key=category", None).await.json(),
    json!({ "items": [{ "value": "audit", "count": 2 }, { "value": "patch", "count": 1 }], "more": false })
  );
  assert_eq!(
    service.call(Method::GET, "/label-values?key=category&limit=1", None).await.json(),
    json!({ "items": [{ "value": "audit", "count": 2 }], "more": true })
  );
  assert_eq!(
    service.call(Method::GET, "/label-values?key=unused", None).await.json(),
    json!({ "items": [], "more": false })
  );

  let blank = service.call(Method::GET, "/label-values?key=%20", None).await;

  assert_eq!(
    (blank.status, blank.json()["code"].clone()),
    (StatusCode::BAD_REQUEST, json!("bad-request"))
  );
}

#[tokio::test]
async fn the_kinds_answer_for_themselves_and_the_service_is_ready() {
  let service = Service::new().await;
  let kinds = service.call(Method::GET, "/document-kinds", None).await.json();
  let named: Vec<(&str, bool, i64, &str)> = kinds["items"]
    .as_array()
    .expect("items")
    .iter()
    .map(|kind| {
      (
        kind["name"].as_str().unwrap_or_default(),
        kind["hosted"].as_bool().unwrap_or_default(),
        kind["contentsDepth"].as_i64().unwrap_or(-1),
        kind["icon"].as_str().unwrap_or_default(),
      )
    })
    .collect();

  assert_eq!(
    named,
    [
      ("search", false, 2, "travel-explore"),
      ("record", false, 2, "description-outline"),
      ("learning", false, 3, "menu-book-outline"),
      ("import", false, 3, "public"),
      ("page", true, 0, "web")
    ]
  );
  assert_eq!(
    service.call(Method::GET, "/document-kinds?after=4", None).await.json()["items"][0]["name"],
    "page"
  );

  let health = service.call(Method::GET, "/health", None).await.json();

  assert_eq!(health, json!({ "ready": true, "reason": null, "version": bmo_artifact::VERSION }));
}
