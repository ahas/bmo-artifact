//! A page imported whole (`docs/artifact.md`, "A page imported whole"): read as served through the page
//! reader, its links and pictures made absolute, its pictures and inline drawings kept as the document's
//! files, its main content written as markdown by a maintained converter, and filed as a document of kind
//! `import` whose references are the page itself, the address asked where the page moved, and each outside
//! page its body links to.

use std::path::Path;

use ego_tree::NodeId;
use lol_html::{RewriteStrSettings, element, rewrite_str};
use scraper::node::Text;
use scraper::{ElementRef, Html, Node, Selector, StrTendril};
use sea_orm::DatabaseConnection;
use url::Url;

use super::filing::{file_markdown, links_of};
use super::keeper::{Handed, keep};
use super::page_reading::collapsed;
use crate::Refusal;
use crate::helpers::marks::fill_marks;
use crate::reader::{PageReader, Picture};
use crate::wire::{Attachment, DocumentInput, Labels};
use crate::wording::Import;

/// The most pictures one page keeps; the rest stand as the line for a figure not kept.
pub const MOST_FIGURES: usize = 40;

/// What a page is filed under: the title asked for, the scope and the actor.
pub struct Asked<'a> {
  pub url: &'a str,
  pub title: Option<&'a str>,
  pub scope: Option<&'a str>,
  pub actor_name: &'a str,
}

/// A page imported: its document's number and the files it keeps.
pub struct Imported {
  pub document_id: i64,
  pub attachments: Vec<Attachment>,
}

/// One figure of the page: where it stands in the page's tree, where its picture comes from (none for an
/// address no page reader reads), and its caption.
struct Figure {
  node: NodeId,
  source: Option<Source>,
  caption: String,
}

enum Source {
  Address(String),
  Drawing(String),
}

/// How one figure stands in the document: a kept file drawn by its name, the line for a figure not kept, or
/// nothing.
enum Standing {
  Kept(String),
  Missing(String),
  Gone,
}

fn absolute(base: &Url, href: &str) -> Option<String> {
  let joined = base.join(href.trim()).ok()?;

  matches!(joined.scheme(), "http" | "https").then(|| joined.to_string())
}

fn host_of(address: &str) -> String {
  Url::parse(address)
    .ok()
    .and_then(|url| url.host_str().map(str::to_owned))
    .unwrap_or_else(|| address.to_owned())
}

// Two writings of one page: a fragment and a trailing slash are the same page.
fn normal(address: &str) -> String {
  match Url::parse(address) {
    Ok(mut url) => {
      url.set_fragment(None);
      url.as_str().trim_end_matches('/').to_owned()
    }
    Err(_) => address.to_owned(),
  }
}

fn text_of(element: ElementRef<'_>) -> String {
  collapsed(&element.text().collect::<String>())
}

// The page with every link and picture address made absolute against the address it was read at.
fn absolutized(html: &str, base: &Url) -> String {
  let rewritten = rewrite_str(
    html,
    RewriteStrSettings::new()
      .append_element_content_handler(element!("a[href]", |element| {
        if let Some(address) = element.get_attribute("href").and_then(|href| absolute(base, &href)) {
          element.set_attribute("href", &address)?;
        }

        Ok(())
      }))
      .append_element_content_handler(element!("img[src]", |element| {
        if let Some(address) = element.get_attribute("src").and_then(|src| absolute(base, &src)) {
          element.set_attribute("src", &address)?;
        }

        Ok(())
      })),
  );

  rewritten.unwrap_or_else(|_| html.to_owned())
}

// The figures of the page in the order they stand: each picture by its address and each outermost inline
// drawing by its own markup.
fn figures_of(page: &Html) -> Vec<Figure> {
  let selector = Selector::parse("body img, body svg").expect("a selector");
  let mut figures = Vec::new();

  for element in page.select(&selector) {
    let svg = element.value().name() == "svg";

    if element
      .ancestors()
      .any(|above| above.value().as_element().is_some_and(|tag| tag.name() == "svg"))
    {
      continue;
    }

    let caption = element
      .value()
      .attr("alt")
      .or_else(|| element.value().attr("aria-label"))
      .map(collapsed)
      .filter(|caption| !caption.is_empty())
      .unwrap_or_else(|| format!("figure {}", figures.len() + 1));
    let source = if svg {
      Some(Source::Drawing(element.html()))
    } else {
      element
        .value()
        .attr("src")
        .filter(|src| src.starts_with("http://") || src.starts_with("https://"))
        .map(|src| Source::Address(src.to_owned()))
    };

    figures.push(Figure {
      node: element.id(),
      source,
      caption,
    });
  }

  figures
}

// The mark a figure stands as while the page is written as markdown: letters and digits alone, which the
// converter writes as they are.
fn mark(n: usize) -> String {
  format!("ArtifactFigureMark{n}X")
}

// The page's main content as markdown, each figure standing where it stood: its article, else its main, else
// its body.
fn main_markdown(html: &str, standings: &[Standing]) -> Result<String, String> {
  let mut page = Html::parse_document(html);

  for (at, figure) in figures_of(&page).iter().enumerate() {
    if let Some(mut node) = page.tree.get_mut(figure.node) {
      node.insert_before(Node::Text(Text {
        text: StrTendril::from(format!(" {} ", mark(at + 1)).as_str()),
      }));
      node.detach();
    }
  }

  let main = ["article", "main", "[role=main]", "body"]
    .iter()
    .find_map(|css| page.select(&Selector::parse(css).expect("a selector")).next())
    .map(|element| element.html())
    .unwrap_or_else(|| page.html());
  let converter = htmd::HtmlToMarkdown::builder()
    .skip_tags(vec!["script", "style", "noscript", "template", "nav", "footer", "form", "button"])
    .build();
  let mut markdown = converter.convert(&main).map_err(|why| why.to_string())?;

  // The marks are replaced from the last, so mark 1 is never found inside mark 10.
  for (at, standing) in standings.iter().enumerate().rev() {
    let written = match standing {
      Standing::Kept(name) => format!("![figure {}]({name})", at + 1),
      Standing::Missing(line) => line.clone(),
      Standing::Gone => String::new(),
    };

    markdown = markdown.replace(&mark(at + 1), &written);
  }

  Ok(markdown.trim().to_owned())
}

/// A page imported and filed: read through `reader`, each figure kept as it could be, the document filed
/// under what was asked. Refused with the sentence why where the page could not be read or the document not
/// filed.
pub async fn import_page(
  db: &DatabaseConnection,
  root: &Path,
  reader: &dyn PageReader,
  words: &Import,
  asked: Asked<'_>,
) -> Result<Imported, String> {
  let markup = reader.page(asked.url).await?;
  let base = Url::parse(&markup.url).map_err(|why| format!("{} is not an address: {why}", markup.url))?;
  let html = absolutized(&markup.html, &base);
  // What is read of the parsed page is taken out of it at once: the page's tree does not cross a wait.
  let (title, figures) = {
    let page = Html::parse_document(&html);
    let heading = page.select(&Selector::parse("h1").expect("a selector")).next().map(text_of);
    let head_title = page.select(&Selector::parse("title").expect("a selector")).next().map(text_of);
    let title = asked
      .title
      .map(collapsed)
      .filter(|title| !title.is_empty())
      .or(heading.filter(|title| !title.is_empty()))
      .or(head_title.filter(|title| !title.is_empty()))
      .unwrap_or_else(|| host_of(&markup.url));
    let figures: Vec<(Option<Source>, String)> =
      figures_of(&page).into_iter().map(|figure| (figure.source, figure.caption)).collect();

    (title, figures)
  };
  let mut pictures: Vec<(String, Picture, Option<String>)> = Vec::new();
  let mut standings = Vec::new();

  for (at, (source, caption)) in figures.into_iter().enumerate() {
    let n = at + 1;
    let read = match source {
      None => {
        standings.push(Standing::Gone);
        continue;
      }
      Some(_) if at >= MOST_FIGURES => Err(format!("past the {MOST_FIGURES} figures a page keeps")),
      Some(Source::Address(address)) => {
        reader.picture(&address).await.map(|picture| (picture, Some(address)))
      }
      Some(Source::Drawing(drawn)) => Ok((
        Picture {
          bytes: drawn.into_bytes(),
          media_type: "image/svg+xml".to_owned(),
        },
        None,
      )),
    };

    standings.push(match read {
      Ok((picture, address)) => {
        let name = format!("figure-{n:02}.{}", ending_of(&picture.media_type));

        pictures.push((name.clone(), picture, address));
        Standing::Kept(name)
      }
      Err(_) => {
        Standing::Missing(fill_marks(&words.missing, &[("n", &n.to_string()), ("caption", &caption)]))
      }
    });
  }

  let body = main_markdown(&html, &standings)?;
  let body = if body.is_empty() { format!("# {title}") } else { body };
  let mut references = vec![(markup.url.clone(), Some(title.clone()))];

  if normal(asked.url) != normal(&markup.url) {
    references.push((asked.url.to_owned(), None));
  }

  for (url, words) in links_of(&body) {
    if host_of(&url) != host_of(&markup.url)
      && !references.iter().any(|(seen, _)| normal(seen) == normal(&url))
    {
      references.push((url, words));
    }
  }

  let input = DocumentInput {
    kind: "import".to_owned(),
    title,
    body: Some(body),
    html: None,
    scope: asked.scope.map(str::to_owned),
    actor_name: asked.actor_name.to_owned(),
    labels: Labels::new(),
    description: None,
  };
  let document_id = file_markdown(db, root, &input, references).await.map_err(|why: Refusal| why.detail)?;
  let mut attachments = Vec::new();

  for (name, picture, address) in pictures {
    let handed = Handed {
      name: &name,
      bytes: picture.bytes,
      media_type: Some(&picture.media_type),
      role: None,
      url: address.as_deref(),
    };

    if let Ok(kept) = keep(db, root, document_id, handed).await {
      attachments.push(kept);
    }
  }

  Ok(Imported {
    document_id,
    attachments,
  })
}

// The ending a kept picture's name takes by its type.
fn ending_of(media_type: &str) -> &'static str {
  match media_type {
    "image/jpeg" => "jpg",
    "image/png" => "png",
    "image/gif" => "gif",
    "image/webp" => "webp",
    "image/avif" => "avif",
    "image/svg+xml" => "svg",
    "image/tiff" => "tiff",
    _ => "bin",
  }
}
