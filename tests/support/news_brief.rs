//! What the morning news page's tests share: a writer's answer that reads as a brief, the English wording,
//! and a small picture to fetch.

use std::io::Cursor;

use bmo_artifact::wording::Wording;
use bmo_artifact::wording::news_page::NewsPage;
use serde_json::{Value, json};

/// The page's English words.
pub fn words() -> NewsPage {
  Wording::english().expect("the English default reads").news_page
}

/// One article's card as the writer writes it, its address `https://news.example/<n>`.
pub fn item_value(n: usize) -> Value {
  json!({
    "tag": "economy",
    "title": format!("Title {n}"),
    "lines": [format!("First line {n}"), format!("Second line {n}")],
    "url": format!("https://news.example/{n}"),
    "image": null,
  })
}

/// A writer's whole answer: two summary lines, six cards and the three fixed sections.
pub fn brief_value() -> Value {
  json!({
    "summary": ["First summary", "Second summary"],
    "items": (1..=6).map(item_value).collect::<Vec<_>>(),
    "realEstate": { "lines": ["Apartment prices rose less"], "sources": ["https://www.example.org/weekly"] },
    "koreaMarket": { "lines": ["Foreign buying lifted the market"], "movers": [{ "name": "Samsung", "move": "+2.1%" }], "sources": [] },
    "usMarket": { "lines": ["Rates weighed on stocks"], "movers": [], "sources": ["https://www.example.com/us"] },
  })
}

/// A small PNG picture's bytes.
pub fn png() -> Vec<u8> {
  let mut png = Cursor::new(Vec::new());

  image::DynamicImage::ImageRgb8(image::RgbImage::new(40, 30))
    .write_to(&mut png, image::ImageFormat::Png)
    .expect("the picture is written");
  png.into_inner()
}
