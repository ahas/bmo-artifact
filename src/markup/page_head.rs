//! What a kept HTML page declares of the head a link previews by (`docs/archive.md`, "The share page"): its
//! title, its description and robots, its OpenGraph and Twitter tags and its canonical address. A share page
//! of a page document carries each in place of the link's own tag of the same name.

use scraper::{Html, Node, Selector};
use serde::Serialize;

/// One preview tag a page writes itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum DeclaredMeta {
  Named { name: String, content: String },
  Property { property: String, content: String },
}

/// The tags of the head a link previews by that a page writes itself.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DeclaredHead {
  pub title: Option<String>,
  pub meta: Vec<DeclaredMeta>,
  pub canonical: Option<String>,
}

fn selector(css: &str) -> Selector {
  Selector::parse(css).expect("a selector")
}

fn declared(name: &str) -> bool {
  matches!(name, "description" | "robots")
    || name.strip_prefix("og:").is_some_and(|rest| !rest.is_empty())
    || name.strip_prefix("twitter:").is_some_and(|rest| !rest.is_empty())
}

/// What a page declares of the head a link previews by: its first title, each preview tag it writes by its
/// name or its property in lower case, and its first canonical address.
pub fn page_head(html: &str) -> DeclaredHead {
  let page = Html::parse_document(html);
  let mut head = DeclaredHead {
    title: page.select(&selector("title")).next().map(|title| {
      title
        .children()
        .filter_map(|child| match child.value() {
          Node::Text(text) => Some(text.to_string()),
          _ => None,
        })
        .collect::<String>()
        .trim()
        .to_owned()
    }),
    ..DeclaredHead::default()
  };

  for meta in page.select(&selector("meta")) {
    let attributes = meta.value();
    let Some(content) = attributes.attr("content") else { continue };
    let named = attributes.attr("name").unwrap_or("").to_lowercase();
    let property = attributes.attr("property").unwrap_or("").to_lowercase();

    if declared(&named) {
      head.meta.push(DeclaredMeta::Named {
        name: named,
        content: content.to_owned(),
      });
    } else if declared(&property) {
      head.meta.push(DeclaredMeta::Property {
        property,
        content: content.to_owned(),
      });
    }
  }

  head.canonical = page
    .select(&selector("link"))
    .find(|link| {
      let rel = link.value().attr("rel").unwrap_or("").to_lowercase();

      rel.split_whitespace().any(|one| one == "canonical")
        && link.value().attr("href").is_some_and(|href| !href.is_empty())
    })
    .and_then(|link| link.value().attr("href").map(str::to_owned));

  head
}
