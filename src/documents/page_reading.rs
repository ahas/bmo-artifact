//! An HTML page read for what it says (`docs/archive.md`, "An HTML page"): its title, its description and
//! its words a block to a line, the pictures it carries inside itself taken out. A page document's title and
//! description are read here, and so is `bmo-artifact html-read`; a markdown body's description is read from
//! its opening prose.

use std::sync::LazyLock;

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use regex::Regex;
use scraper::{Html, Node};

/// About how long a description runs when it is taken from a document's words.
pub const DESCRIPTION_LENGTH: usize = 160;

/// What a markdown body is said to be where it is listed: its opening prose as plain words, headings,
/// pictures and code left out, cut at a space near the length and marked as cut. None for a body with no
/// prose.
pub fn markdown_description(body: &str) -> Option<String> {
  let mut words = String::new();
  let mut skipped = 0usize;
  let mut in_paragraph = false;

  for event in Parser::new(body) {
    match event {
      Event::Start(Tag::Heading { .. } | Tag::Image { .. } | Tag::CodeBlock(_) | Tag::Table(_)) => {
        skipped += 1
      }
      Event::End(TagEnd::Heading(_) | TagEnd::Image | TagEnd::CodeBlock | TagEnd::Table) => {
        skipped = skipped.saturating_sub(1);
      }
      Event::Start(Tag::Paragraph) => in_paragraph = true,
      Event::End(TagEnd::Paragraph) => {
        in_paragraph = false;
        words.push(' ');
      }
      Event::Text(text) | Event::Code(text) if skipped == 0 && in_paragraph => words.push_str(&text),
      Event::SoftBreak | Event::HardBreak if skipped == 0 && in_paragraph => words.push(' '),
      _ => {}
    }

    if words.chars().count() > DESCRIPTION_LENGTH * 2 {
      break;
    }
  }

  cut_description(&collapsed(&words))
}

/// Words cut at a space near the description's length and marked as cut; none for no words.
pub fn cut_description(words: &str) -> Option<String> {
  let words = words.trim();

  if words.is_empty() {
    return None;
  }

  if words.chars().count() <= DESCRIPTION_LENGTH {
    return Some(words.to_owned());
  }

  let cut: String = words.chars().take(DESCRIPTION_LENGTH).collect();
  let space = cut.char_indices().rev().find(|(_, character)| *character == ' ').map(|(at, _)| at);
  let kept = match space {
    Some(at) if cut[..at].chars().count() > DESCRIPTION_LENGTH / 2 => &cut[..at],
    _ => cut.as_str(),
  };

  Some(format!("{}…", kept.trim_end()))
}

/// An HTML page read for what it says: its title, what its description meta says of it, and its words a
/// block to a line, a heading keeping its level and a list item its dash, a picture standing as a markdown
/// picture where it stood; scripts and styles are not words. A picture the page carries inside itself as a data
/// address is taken out as a file of its own, named after the reading's stem in the order the pictures stood, and
/// stands in the words under that name.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Reading {
  pub title: Option<String>,
  pub description: Option<String>,
  pub lines: Vec<String>,
  pub pictures: Vec<Picture>,
}

/// A picture a page carried inside itself: the name it stands under in the reading and is kept by, its type, and
/// its bytes.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Picture {
  pub name: String,
  pub kind: String,
  pub bytes: Vec<u8>,
}

/// What the pictures taken out of a page are named after where nothing else is asked.
pub const PICTURE_STEM: &str = "picture";

// The ending a picture's file takes by its type; any other type ends in its own subtype.
const PICTURE_ENDINGS: [(&str, &str); 6] = [
  ("jpeg", "jpg"),
  ("png", "png"),
  ("gif", "gif"),
  ("webp", "webp"),
  ("avif", "avif"),
  ("svg+xml", "svg"),
];

// The tags that start a line of their own, and those whose insides are not words.
const BLOCKS: [&str; 31] = [
  "address",
  "article",
  "aside",
  "blockquote",
  "br",
  "button",
  "dd",
  "details",
  "div",
  "dl",
  "dt",
  "figcaption",
  "figure",
  "footer",
  "form",
  "header",
  "hr",
  "li",
  "main",
  "nav",
  "ol",
  "p",
  "pre",
  "section",
  "small",
  "summary",
  "table",
  "td",
  "th",
  "tr",
  "ul",
];
const UNREAD: [&str; 6] = ["script", "style", "noscript", "template", "svg", "head"];

#[derive(Default)]
struct Walk {
  reading: Reading,
  stem: String,
  line: String,
  prefix: String,
  title: Option<String>,
  in_title: bool,
  unread: usize,
}

pub fn collapsed(text: &str) -> String {
  text.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Walk {
  // What a picture's source stands as in the words: a data address taken out under the next name, anything else
  // as it is given.
  fn picture(&mut self, source: &str, alt: &str) -> String {
    let taken = data_url::DataUrl::process(source).ok().and_then(|data| {
      let kind = data.mime_type();

      (kind.type_ == "image").then(|| (format!("{}/{}", kind.type_, kind.subtype), data.decode_to_vec()))
    });
    let Some((kind, Ok((bytes, _)))) = taken else {
      return format!("![{alt}]({source})");
    };
    let subtype = kind.trim_start_matches("image/");
    let ending = PICTURE_ENDINGS
      .iter()
      .find(|(named, _)| *named == subtype)
      .map_or_else(|| subtype.split('+').next().unwrap_or(subtype), |(_, ending)| ending);
    let name = format!("{}-{:02}.{ending}", self.stem, self.reading.pictures.len() + 1);

    self.reading.pictures.push(Picture {
      name: name.clone(),
      kind,
      bytes,
    });

    format!("![{alt}]({name})")
  }

  fn flush(&mut self) {
    let words = collapsed(&std::mem::take(&mut self.line));

    if !words.is_empty() {
      self.reading.lines.push(format!("{}{words}", std::mem::take(&mut self.prefix)));
    }
  }

  fn children(&mut self, node: ego_tree::NodeRef<'_, Node>) {
    for child in node.children() {
      self.node(child);
    }
  }

  fn node(&mut self, node: ego_tree::NodeRef<'_, Node>) {
    match node.value() {
      Node::Text(text) => {
        if self.in_title {
          self.title.get_or_insert_with(String::new).push_str(text);
        } else if self.unread == 0 {
          self.line.push_str(text);
        }
      }
      Node::Element(element) => {
        let tag = element.name().to_ascii_lowercase();

        if tag == "meta" {
          let named = element.attr("name").or_else(|| element.attr("property")).unwrap_or_default();

          if self.reading.description.is_none()
            && (named.eq_ignore_ascii_case("description") || named.eq_ignore_ascii_case("og:description"))
          {
            let content = collapsed(element.attr("content").unwrap_or_default());

            if !content.is_empty() {
              self.reading.description = Some(content);
            }
          }

          return;
        }

        if tag == "title" {
          self.in_title = true;
          self.children(node);
          self.in_title = false;
          return;
        }

        if UNREAD.contains(&tag.as_str()) {
          self.unread += 1;
          self.children(node);
          self.unread -= 1;
          return;
        }

        if self.unread > 0 {
          return self.children(node);
        }

        // Words in two neighbouring elements are two words, however the page set them side by side.
        self.line.push(' ');

        let heading = tag
          .strip_prefix('h')
          .and_then(|level| level.parse::<usize>().ok())
          .filter(|level| (1..=6).contains(level));

        if tag == "img" {
          self.flush();

          let line =
            self.picture(element.attr("src").unwrap_or_default(), element.attr("alt").unwrap_or_default());

          self.reading.lines.push(line);
          return;
        }

        if heading.is_some() || BLOCKS.contains(&tag.as_str()) {
          self.flush();

          if let Some(level) = heading {
            self.prefix = format!("{} ", "#".repeat(level));
          }

          if tag == "li" {
            self.prefix = "- ".to_owned();
          }

          self.children(node);

          if tag != "br" && tag != "hr" && self.unread == 0 {
            self.flush();
          }

          return;
        }

        self.children(node);
      }
      _ => self.children(node),
    }
  }
}

/// The reading of one page, the pictures inside it named after `stem`.
pub fn read_html(html: &str, stem: &str) -> Reading {
  let document = Html::parse_document(html);
  let mut walk = Walk {
    stem: stem.to_owned(),
    ..Walk::default()
  };

  walk.node(document.tree.root());
  walk.flush();

  let title = walk.title.map(|title| collapsed(&title)).filter(|title| !title.is_empty());

  Reading {
    title,
    ..walk.reading
  }
}

/// What a page is said to be where it is listed: its own description, and failing that the start of its
/// words — neither its title again, nor a picture, nor a heading's or a list's mark — cut at a space near the
/// length and marked as cut. None for a page with no words at all.
pub fn page_description(reading: &Reading) -> Option<String> {
  static MARK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:#{1,6} |- )").expect("a pattern"));

  if let Some(description) = &reading.description {
    return Some(description.clone());
  }

  let words: Vec<String> = reading
    .lines
    .iter()
    .filter(|line| !line.starts_with("!["))
    .map(|line| MARK.replace(line, "").into_owned())
    .enumerate()
    .filter(|(index, line)| !(*index == 0 && Some(line) == reading.title.as_ref()))
    .map(|(_, line)| line)
    .collect();
  cut_description(&words.join(" "))
}
