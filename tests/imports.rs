//! A page imported as a caller asks for one (`docs/api.md`, "Imports"; `docs/artifact.md`, "A page imported
//! whole"): answered at once as fetching, read through the page reader, its figures kept or named as not
//! kept, its links made absolute, and filed with its references; or failed with the reader's sentence.

mod support;

use std::time::Duration;

use axum::http::{Method, StatusCode};
use bmo_artifact::reader::{PageMarkup, Picture};
use serde_json::{Value, json};
use support::service::Service;

const PAGE: &str = r#"<!doctype html><html><head><title>Head title</title></head><body>
<nav><a href="/menu">Menu</a></nav>
<article>
  <h1>The article</h1>
  <p>Text with a <a href="/local">local link</a> and an <a href="https://other.example/x">outside one</a>.</p>
  <img src="/pic.png" alt="A picture">
  <svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="20" height="10" fill="teal"/></svg>
  <img src="https://gone.example/missing.png" alt="Lost one">
</article>
</body></html>"#;

fn png() -> Vec<u8> {
  let mut png = Vec::new();

  image::RgbImage::from_pixel(8, 8, image::Rgb([10, 120, 200]))
    .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
    .expect("a png is written");
  png
}

// The import once it is no longer fetching, asked again until it is, a few hundred times at most.
async fn settled(service: &Service, id: i64) -> Value {
  for _ in 0..500 {
    let import = service.call(Method::GET, &format!("/imports/{id}"), None).await.json();

    if import["state"] != "fetching" {
      return import;
    }

    tokio::time::sleep(Duration::from_millis(10)).await;
  }

  panic!("import {id} is still fetching");
}

#[tokio::test]
async fn a_page_is_read_its_figures_kept_and_filed_as_an_import_with_its_references() {
  let service = Service::new().await;

  service.web.pages.lock().expect("the book").insert(
    "https://site.example/a".to_owned(),
    PageMarkup {
      url: "https://site.example/article".to_owned(),
      html: PAGE.to_owned(),
    },
  );
  service.web.pictures.lock().expect("the book").insert(
    "https://site.example/pic.png".to_owned(),
    Picture {
      bytes: png(),
      media_type: "image/png".to_owned(),
    },
  );

  let asked = service
    .call(
      Method::POST,
      "/imports",
      Some(json!({ "url": "https://site.example/a", "scope": "team-a", "actorName": "tester" })),
    )
    .await;

  assert_eq!(asked.status, StatusCode::CREATED, "{}", asked.json());
  assert_eq!(
    (asked.json()["state"].as_str(), asked.json()["documentId"].clone()),
    (Some("fetching"), Value::Null)
  );

  let import = settled(&service, asked.json()["id"].as_i64().expect("an id")).await;

  assert_eq!(import["state"], "filed", "{import}");

  let id = import["documentId"].as_i64().expect("a document");
  let document = service.call(Method::GET, &format!("/documents/{id}"), None).await.json();

  assert_eq!(
    (
      document["kind"].as_str(),
      document["title"].as_str(),
      document["scope"].as_str(),
      document["attachmentCount"].as_i64()
    ),
    (Some("import"), Some("The article"), Some("team-a"), Some(2))
  );

  let body =
    String::from_utf8(service.call(Method::GET, &format!("/documents/{id}/content"), None).await.bytes)
      .expect("markdown");

  assert!(body.contains("![figure 1](figure-01.png)"), "{body}");
  assert!(body.contains("![figure 2](figure-02.svg)"), "{body}");
  assert!(body.contains("Figure 3 could not be kept: Lost one"), "{body}");
  assert!(body.contains("(https://site.example/local)"), "{body}");
  assert!(!body.contains("Menu"), "{body}");

  let references: Vec<String> =
    service.call(Method::GET, &format!("/documents/{id}/references"), None).await.json()["items"]
      .as_array()
      .expect("items")
      .iter()
      .map(|item| item["url"].as_str().unwrap_or_default().to_owned())
      .collect();

  assert_eq!(
    references,
    ["https://site.example/article", "https://site.example/a", "https://other.example/x"]
  );
  assert_eq!(
    service.event_kinds().await,
    [
      "import.created",
      "document.created",
      "attachment.created",
      "attachment.created",
      "import.changed"
    ]
  );
}

#[tokio::test]
async fn a_page_the_reader_cannot_read_fails_the_import_and_an_address_that_is_no_web_page_is_refused() {
  let service = Service::new().await;
  let asked = service
    .call(
      Method::POST,
      "/imports",
      Some(json!({ "url": "https://nowhere.example/", "actorName": "tester" })),
    )
    .await
    .json();
  let import = settled(&service, asked["id"].as_i64().expect("an id")).await;

  assert_eq!(
    (import["state"].as_str(), import["error"].as_str(), import["documentId"].clone()),
    (Some("failed"), Some("https://nowhere.example/ answered 404"), Value::Null)
  );

  for url in ["file:///etc/hosts", "not an address"] {
    let refused = service
      .call(Method::POST, "/imports", Some(json!({ "url": url, "actorName": "tester" })))
      .await;

    assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{url}");
  }

  assert_eq!(service.call(Method::GET, "/imports/999", None).await.status, StatusCode::NOT_FOUND);
}
