//! A document filed and rewritten (`docs/archive.md`, "Filing a document"): its kind decides whether it
//! takes a markdown body or HTML, its row and its body are written together with its file, and each distinct
//! web link its body makes becomes one of its references. Nothing is written for a refusal.

use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};

use super::keeper::{document_folder, write_private};
use super::page_reading::{
  PICTURE_STEM, collapsed, cut_description, markdown_description, page_description, read_html,
};
use crate::Refusal;
use crate::kinds::{self, Kind};
use crate::store::documents::{self, NewRow};
use crate::store::references;
use crate::wire::{DocumentChange, DocumentInput};

/// The file a markdown document reads from, under its folder.
pub const MARKDOWN_FILE: &str = "document.md";
/// The file a page reads from, under its folder.
pub const PAGE_FILE: &str = "page.html";

/// What a document's file and body come to once its input is read: its title, its description, the words
/// its body is searched by, the file's text and name, and the pages it refers to.
struct Written {
  title: String,
  description: Option<String>,
  words: String,
  file: String,
  file_name: &'static str,
  references: Vec<(String, Option<String>)>,
}

fn kind_of(name: &str) -> Result<&'static dyn Kind, Refusal> {
  kinds::known(Some(name)).ok_or_else(|| {
    let names: Vec<&str> = kinds::KINDS.iter().map(|kind| kind.name()).collect();

    Refusal::bad(format!("no kind is named {name:?}; the kinds are {}", names.join(", ")))
  })
}

// A page read for its title, its description and its words; the title given wins over the page's own.
fn page(title: Option<&str>, html: &str) -> Result<Written, Refusal> {
  if html.trim().is_empty() {
    return Err(Refusal::bad("a page needs its HTML"));
  }

  let reading = read_html(html, PICTURE_STEM);
  let title = title
    .map(str::trim)
    .filter(|title| !title.is_empty())
    .map(str::to_owned)
    .or_else(|| reading.title.clone())
    .ok_or_else(|| Refusal::bad("the page has no title element: give it a title"))?;

  Ok(Written {
    title,
    description: page_description(&reading),
    words: reading.lines.join("\n"),
    file: html.to_owned(),
    file_name: PAGE_FILE,
    references: Vec::new(),
  })
}

// A markdown body read for its description and the pages it refers to.
fn markdown(title: &str, body: &str) -> Result<Written, Refusal> {
  if title.trim().is_empty() {
    return Err(Refusal::bad("a document needs a title"));
  }

  if body.trim().is_empty() {
    return Err(Refusal::bad("the body is empty"));
  }

  Ok(Written {
    title: title.trim().to_owned(),
    description: markdown_description(body),
    words: body.to_owned(),
    file: body.to_owned(),
    file_name: MARKDOWN_FILE,
    references: links_of(body),
  })
}

/// Each distinct http or https link a markdown body makes, in its order, with the words it is written as.
pub fn links_of(body: &str) -> Vec<(String, Option<String>)> {
  let mut links: Vec<(String, Option<String>)> = Vec::new();
  let mut open: Option<(String, String)> = None;

  for event in Parser::new(body) {
    match event {
      Event::Start(Tag::Link { dest_url, .. }) => open = Some((dest_url.into_string(), String::new())),
      Event::Text(text) | Event::Code(text) => {
        if let Some((_, words)) = open.as_mut() {
          words.push_str(&text);
        }
      }
      Event::End(TagEnd::Link) => {
        let Some((url, words)) = open.take() else { continue };
        let web = url::Url::parse(&url).is_ok_and(|parsed| matches!(parsed.scheme(), "http" | "https"));

        if web && !links.iter().any(|(seen, _)| *seen == url) {
          let words = collapsed(&words);

          links.push((url, Some(words).filter(|words| !words.is_empty())));
        }
      }
      _ => {}
    }
  }

  links
}

// The file written under the document's folder; its path.
fn write_file(root: &Path, id: i64, written: &Written) -> Result<PathBuf, Refusal> {
  let folder = document_folder(root, id);
  let path = folder.join(written.file_name);

  std::fs::create_dir_all(&folder).map_err(|why| Refusal::failed(why.to_string()))?;
  write_private(&path, written.file.as_bytes()).map_err(|why| Refusal::failed(why.to_string()))?;

  Ok(path)
}

// The body as the document's one finding, and its references beside it.
async fn write_body<C: ConnectionTrait>(db: &C, id: i64, written: &Written) -> Result<(), Refusal> {
  let finding = documents::set_body(db, id, &written.words).await?;

  for (url, title) in &written.references {
    references::insert(db, id, finding, url, title.as_deref()).await?;
  }

  Ok(())
}

/// A document filed: its input read by its kind, its row, body, references and file written together. Its
/// number.
pub async fn file(db: &DatabaseConnection, root: &Path, input: &DocumentInput) -> Result<i64, Refusal> {
  let kind = kind_of(&input.kind)?;
  let body = input.body.as_deref().unwrap_or_default();
  let html = input.html.as_deref().unwrap_or_default();

  if input.actor_name.trim().is_empty() {
    return Err(Refusal::bad("a document needs an actorName"));
  }

  let written = if kind.hosted() {
    if !body.trim().is_empty() {
      return Err(Refusal::bad("a page is filed as its HTML, with no body"));
    }

    page(Some(&input.title), html)?
  } else {
    if !html.trim().is_empty() {
      return Err(Refusal::bad(format!("a {} is filed as a markdown body, with no HTML", kind.name())));
    }

    markdown(&input.title, body)?
  };

  file_written(db, root, kind, input, &written).await
}

/// A document filed out of what is already read of it, as an import files the page it read.
pub async fn file_markdown(
  db: &DatabaseConnection,
  root: &Path,
  input: &DocumentInput,
  references: Vec<(String, Option<String>)>,
) -> Result<i64, Refusal> {
  let kind = kind_of(&input.kind)?;
  let mut written = markdown(&input.title, input.body.as_deref().unwrap_or_default())?;

  written.references = references;
  file_written(db, root, kind, input, &written).await
}

async fn file_written(
  db: &DatabaseConnection,
  root: &Path,
  kind: &dyn Kind,
  input: &DocumentInput,
  written: &Written,
) -> Result<i64, Refusal> {
  // The description the filer gives stands over the one read out of the body or the page.
  let given = input.description.as_deref().and_then(cut_description);
  let tx = db.begin().await?;
  let id = documents::insert(
    &tx,
    NewRow {
      kind: kind.name(),
      title: &written.title,
      scope: input.scope.as_deref().filter(|scope| !scope.is_empty()),
      actor_name: input.actor_name.trim(),
      labels: &input.labels,
      description: given.as_deref().or(written.description.as_deref()),
    },
  )
  .await?;

  write_body(&tx, id, written).await?;

  let path = write_file(root, id, written)?;

  documents::set_file(&tx, id, &path.to_string_lossy()).await?;
  tx.commit().await?;

  Ok(id)
}

/// A document rewritten under its number: the title, the body or the HTML given replace their own, the labels
/// replace the labels whole; its attachments and links stay. A body for a page or HTML for any other kind is
/// refused as the kind's conflict.
pub async fn rewrite(
  db: &DatabaseConnection,
  root: &Path,
  id: i64,
  change: &DocumentChange,
) -> Result<(), Refusal> {
  let row = documents::read(db, id)
    .await?
    .ok_or_else(|| Refusal::absent(format!("there is no document {id}")))?;
  let kind = kinds::kind(Some(&row.document.kind));

  if change.title.as_deref().is_some_and(|title| title.trim().is_empty()) {
    return Err(Refusal::bad("a title, where one is given, is not empty"));
  }

  let written = match (kind.hosted(), change.body.as_deref(), change.html.as_deref()) {
    (true, Some(_), _) => {
      return Err(Refusal::conflict(format!("document {id} is a page: it takes HTML, not a body")));
    }
    (false, _, Some(_)) => {
      return Err(Refusal::conflict(format!(
        "document {id} is a {}: it takes a body, not HTML",
        kind.name()
      )));
    }
    (true, None, Some(html)) => Some(page(change.title.as_deref().or(Some(&row.document.title)), html)?),
    (false, Some(body), None) => {
      Some(markdown(change.title.as_deref().unwrap_or(&row.document.title), body)?)
    }
    _ => None,
  };
  let tx = db.begin().await?;

  documents::rewrite(
    &tx,
    id,
    change.title.as_deref().map(str::trim),
    &change.labels,
    written.as_ref().map(|written| written.description.as_deref()),
  )
  .await?;

  if let Some(written) = &written {
    write_body(&tx, id, written).await?;

    let path = write_file(root, id, written)?;

    documents::set_file(&tx, id, &path.to_string_lossy()).await?;
  }

  tx.commit().await?;

  Ok(())
}
