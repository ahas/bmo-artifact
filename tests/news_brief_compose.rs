//! The morning news page put together through the tool host: each article's picture the writer named or the
//! article's own, each index's closes from the `stock_index` site tool, and what could not be had left plain
//! and said (`docs/news-brief.md`, "How one page is made"; `docs/api.md`, "Read the index closes").

mod support;

use std::collections::HashMap;

use bmo_artifact::clients::tool_host::ToolHost;
use bmo_artifact::verbs::news_brief::brief::{Brief, read_brief};
use bmo_artifact::verbs::news_brief::compose::{article_pictures, compose, pacing, page_picture};
use serde_json::{Value, json};
use support::news_brief::{brief_value, png, words};
use support::units::{self, Book, Units};

// A page's head as `explorer/fetch_page` answers it, naming its own picture where `picture` is some.
fn head(address: &str, picture: Option<&str>) -> Value {
  let meta = picture
    .map_or_else(String::new, |picture| format!(r#"<meta property="og:image" content="{picture}">"#));

  json!({ "url": address, "type": "text/html", "html": format!("<html><head>{meta}</head><body></body></html>") })
}

// The tool host as the morning of one room reads it: the first article names its picture; the second's page
// names none; the third's picture will not come; the rest name their own, relative to the page.
async fn served(closes: Option<Value>) -> (Units, ToolHost) {
  let article = |n: usize| format!("https://news.example/{n}");
  let heads: HashMap<String, Value> = [
    (article(2), head(&article(2), None)),
    (article(3), head(&article(3), Some("/3.og.jpg"))),
    (article(4), head(&article(4), Some("/4.og.jpg"))),
    (article(5), head(&article(5), Some("/5.og.jpg"))),
    (article(6), head(&article(6), Some("/6.og.jpg"))),
  ]
  .into();
  let rasters: HashMap<String, Vec<u8>> = [
    "https://news.example/1.jpg",
    "https://news.example/4.og.jpg",
    "https://news.example/5.og.jpg",
    "https://news.example/6.og.jpg",
  ]
  .iter()
  .map(|address| ((*address).to_owned(), png()))
  .collect();
  let (units, address) = units::serve(Book {
    heads,
    rasters,
    closes,
    ..Book::default()
  })
  .await;

  (units, ToolHost::new(Some(address), pacing("2026-09-28", 7)))
}

fn brief() -> Brief {
  let mut value = brief_value();

  value["items"][0]["image"] = json!("https://news.example/1.jpg");
  read_brief(&value.to_string()).expect("a brief")
}

#[tokio::test]
async fn takes_the_picture_the_writer_named_else_the_articles_own_and_goes_on_without_the_rest() {
  let (units, host) = served(None).await;
  let (pictures, said) = article_pictures(&host, &brief().items).await;
  let mut kept: Vec<&String> = pictures.keys().collect();

  kept.sort();

  assert_eq!(
    kept,
    [
      "https://news.example/1",
      "https://news.example/4",
      "https://news.example/5",
      "https://news.example/6"
    ]
  );
  assert!(pictures.values().all(|picture| picture.starts_with("data:image/jpeg;base64,")));
  assert_eq!(said[0], "no picture for https://news.example/2");
  assert!(said[1].contains("https://news.example/3") && said[1].contains("404"), "{said:?}");
  assert_eq!(said.len(), 2);
  assert!(
    units
      .book()
      .calls
      .iter()
      .all(|call| call["arguments"]["pacing"] == "news-brief:2026-09-28:7")
  );
  assert!(
    !units
      .book()
      .calls
      .iter()
      .any(|call| call["arguments"]["address"] == "https://news.example/1")
  );
}

#[tokio::test]
async fn draws_the_indices_that_answered_and_says_which_did_not() {
  let closes = json!({ "series": [{ "key": "KOSPI", "days": [
    { "date": "2026-09-23", "close": 7080.92, "change": 63.01, "ratio": 0.9 },
    { "date": "2026-09-25", "close": 7100.0, "change": 19.08, "ratio": 0.27 },
  ] }] });
  let (units, host) = served(Some(closes)).await;
  let words = words();
  let composed = compose(&words, "2026-09-28", &brief(), &host).await;
  let asked = units
    .book()
    .calls
    .iter()
    .find(|call| call["tool"] == "web/stock_index")
    .cloned()
    .expect("asked");

  assert_eq!(
    asked["arguments"],
    json!({ "indices": ["KOSPI", "KOSDAQ", "DJI", "SPX", "NASDAQ"], "days": 20 })
  );
  assert!(composed.html.contains(r#"<span class="idx-close">7,100.00</span>"#));
  assert_eq!(composed.html.matches(words.no_close.as_str()).count(), 4);
  assert!(composed.said.contains(&"KOSDAQ could not be read".to_owned()));
  assert_eq!(composed.said.last().map(String::as_str), Some("4 of 6 pictures; 1 of 5 indices"));

  let (_, unanswered) = served(None).await;
  let composed = compose(&words, "2026-09-28", &brief(), &unanswered).await;

  assert!(
    composed
      .said
      .iter()
      .any(|line| line.starts_with("the index closes could not be read: "))
  );
  assert_eq!(composed.html.matches(words.no_close.as_str()).count(), 5);
}

#[test]
fn a_pages_own_picture_is_read_off_its_tags_and_resolved_against_it() {
  let base = url::Url::parse("https://news.example/a/1").expect("an address");
  let page = r#"<html><head><meta name="twitter:image" content="/t.jpg"><meta property="og:image" content="/pic/1.jpg"></head><body></body></html>"#;

  assert_eq!(page_picture(page, &base).as_deref(), Some("https://news.example/pic/1.jpg"));
  assert_eq!(page_picture("<html><head></head></html>", &base), None);
  assert_eq!(page_picture(r#"<meta property="og:image" content="javascript:x">"#, &base), None);
}
