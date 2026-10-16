//! A page brought into the record whole (`docs/archive.md`, "A page imported whole"): somebody hands over one
//! address and asks for the page as it stands, so nothing is researched and no model runs. The page is read
//! through the page reader, and the reading it takes of itself becomes the search's result in the result's
//! own structure lines — its title as the head, its sections as sections, its pictures in their places — with
//! nothing beyond what the structure needs to be read.

use crate::helpers::marks::fill_marks;
use crate::wording::search::Import;

use super::web::{Figure, Page};

/// The page as a result: the page it came from, the address asked where it moved, what the page itself cites,
/// each once, and the figures `missing` could not be captured, in the wording's words.
pub fn imported_result(asked: &str, page: &Page, missing: &[&Figure], words: &Import) -> String {
  let host = host_of(&page.url);
  let moved = normal(asked) != normal(&page.url);
  let mut lines = vec![
    format!("ANSWERED: yes :: {}", words.answered),
    format!("TITLE: {}", title_of(page)),
    format!("TOPIC: {}", fill_marks(&words.topic, &[("host", &host)])),
  ];

  if !page.keywords.is_empty() {
    lines.push(format!(
      "KEYWORDS: {}",
      page.keywords.iter().map(|word| one_line(word)).collect::<Vec<_>>().join(", ")
    ));
  }

  lines.extend([String::new(), page.body.clone(), String::new()]);

  if !missing.is_empty() {
    lines.push("UNCONFIRMED:".to_owned());

    for (at, figure) in missing.iter().enumerate() {
      lines.push(format!(
        "{}. {}",
        at + 1,
        fill_marks(&words.missing, &[("n", &figure.n.to_string()), ("caption", &figure.caption)])
      ));
    }

    lines.push(String::new());
  }

  lines.push(format!("SOURCE: {} :: {}", page.url, words.kept));

  if moved {
    lines.push(format!("SOURCE: {asked} :: {}", words.moved));
  }

  let mut cited: Vec<String> = vec![];

  for reference in &page.references {
    let at = normal(&reference.url);

    if cited.contains(&at) || at == normal(&page.url) {
      continue;
    }

    cited.push(at);

    let text = one_line(&reference.text);

    lines.push(if text.is_empty() {
      format!("SOURCE: {} :: {}", reference.url, words.cited)
    } else {
      format!("SOURCE: {} :: {}", reference.url, fill_marks(&words.cited_text, &[("text", &text)]))
    });
  }

  lines.join("\n")
}

/// The page's title as the room is told it: its own, or its host where it has none.
pub fn title_of(page: &Page) -> String {
  Some(one_line(&page.title))
    .filter(|title| !title.is_empty())
    .unwrap_or_else(|| host_of(&page.url))
}

/// The address a figure the page drew is named by: the page's own with `#figure-<n>`.
pub fn figure_address(page: &Page, figure: &Figure) -> String {
  format!("{}#figure-{}", page.url.split('#').next().unwrap_or_default(), figure.n)
}

fn one_line(text: &str) -> String {
  text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn host_of(address: &str) -> String {
  url::Url::parse(address)
    .ok()
    .and_then(|url| url.host_str().map(str::to_owned))
    .unwrap_or_else(|| address.to_owned())
}

// Two writings of one page: a fragment and a trailing slash are the same page.
fn normal(address: &str) -> String {
  match url::Url::parse(address) {
    Ok(mut url) => {
      url.set_fragment(None);
      url.as_str().trim_end_matches('/').to_owned()
    }
    Err(_) => address.to_owned(),
  }
}
