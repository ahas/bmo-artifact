//! The structure of a markdown text, read off its source: its headings and the sections they open. The
//! outline of every kind and the marks the console stamps on a document's headings both read the
//! headings here, so a heading is one thing to both.
//!
//! The headings are the ATX headings CommonMark reads at the top of the text, a line inside a code block
//! being code whatever it starts with; a setext heading, and a heading inside a quotation or a list, is
//! none.

use std::sync::LazyLock;

use fancy_regex::Regex;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use unicode_normalization::UnicodeNormalization;

/// One heading of a text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownHeading {
  /// 1 for `#`, 6 for `######`.
  pub depth: usize,
  /// What the heading says, with its inline marks taken off.
  pub text: String,
  /// A mark no other heading of the same text carries, made from its words: the first of a text takes the
  /// words alone, the ones after it a number as well.
  pub slug: String,
  /// The source line the heading stands on, counted from 1.
  pub line: usize,
}

// The inline marks a heading's words lose: a link or a picture stands as its words, and emphasis, strikes,
// code marks and escapes fall away. These are kept as the patterns the console reads a heading's words
// with, since CommonMark leaves emphasis open beside CJK punctuation and the words must be the ones the
// console shows.
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"!?\[([^\]]*)\]\([^)]*\)").expect("a pattern"));
static CODE_MARKS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`+").expect("a pattern"));
static STRONG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\*\*|__|~~)(.+?)\1").expect("a pattern"));
static EMPHASIS: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"(^|[^A-Za-z0-9_])[*_](\S(?:.*?\S)?)[*_](?=[^A-Za-z0-9_]|$)").expect("a pattern")
});
static ESCAPE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"\\([\\`*_{}\[\]()#+\-.!~])").expect("a pattern"));

/// A heading's words without the marks around them.
pub fn plain_heading(text: &str) -> String {
  let text = LINK.replace_all(text, "$1");
  let text = CODE_MARKS.replace_all(&text, "");
  let text = STRONG.replace_all(&text, "$2");
  let text = EMPHASIS.replace_all(&text, "$1$2");
  let text = ESCAPE.replace_all(&text, "$1");

  text.trim().to_owned()
}

/// The words of a heading as a mark: lower-case, letters and digits of any script kept, runs of space made
/// one hyphen, everything else dropped.
pub fn heading_slug(text: &str) -> String {
  let kept: String = text
    .nfc()
    .collect::<String>()
    .to_lowercase()
    .chars()
    .filter(|one| one.is_alphanumeric() || one.is_whitespace() || *one == '_' || *one == '-')
    .collect();
  let slug = kept.split_whitespace().collect::<Vec<_>>().join("-");

  if slug.is_empty() { "section".to_owned() } else { slug }
}

// Whether a heading's source is an ATX heading: up to three spaces, then its hashes.
fn is_atx(source: &str) -> bool {
  source.trim_start_matches(' ').starts_with('#')
    && source.len() - source.trim_start_matches(' ').len() <= 3
}

// The heading being read: its depth, its line, and the stretch of the source its words stand in.
struct Reading {
  depth: usize,
  line: usize,
  words: Option<std::ops::Range<usize>>,
}

/// Every heading of a text, in the order it stands. A heading with no words is no entry.
pub fn markdown_headings(source: &str) -> Vec<MarkdownHeading> {
  let source = source.replace("\r\n", "\n").replace('\r', "\n");
  let mut headings = Vec::new();
  let mut taken = std::collections::HashSet::new();
  let mut counts = std::collections::HashMap::<String, usize>::new();
  let mut nested = 0usize;
  let mut open: Option<Reading> = None;

  for (event, range) in Parser::new_ext(&source, Options::ENABLE_STRIKETHROUGH).into_offset_iter() {
    match event {
      Event::Start(Tag::BlockQuote(_) | Tag::List(_) | Tag::Item | Tag::FootnoteDefinition(_)) => {
        nested += 1
      }
      Event::End(TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Item | TagEnd::FootnoteDefinition) => {
        nested = nested.saturating_sub(1);
      }
      Event::Start(Tag::Heading { level, .. }) if nested == 0 && is_atx(&source[range.clone()]) => {
        open = Some(Reading {
          depth: level as usize,
          line: source[..range.start].matches('\n').count() + 1,
          words: None,
        });
      }
      Event::End(TagEnd::Heading(_)) => {
        let Some(reading) = open.take() else { continue };
        let text = plain_heading(reading.words.map_or("", |words| &source[words]));

        if text.is_empty() {
          continue;
        }

        let base = heading_slug(&text);
        let mut slug = base.clone();
        let mut count = counts.get(&base).copied().unwrap_or(0);

        while taken.contains(&slug) {
          count += 1;
          slug = format!("{base}-{count}");
        }

        counts.insert(base, count);
        taken.insert(slug.clone());
        headings.push(MarkdownHeading {
          depth: reading.depth,
          text,
          slug,
          line: reading.line,
        });
      }
      _ => {
        if let Some(reading) = &mut open {
          reading.words = Some(match reading.words.take() {
            None => range,
            Some(words) => words.start.min(range.start)..words.end.max(range.end),
          });
        }
      }
    }
  }

  headings
}

/// A text as its sections: the lines before the first heading, then each heading with the lines under it up
/// to the next heading of any depth. Joined back, the sections are the text.
pub fn markdown_sections(source: &str) -> Vec<String> {
  let lines: Vec<&str> = source.split('\n').collect();
  let starts: Vec<usize> = markdown_headings(source)
    .iter()
    .map(|heading| heading.line - 1)
    .filter(|start| *start > 0)
    .collect();
  let mut sections = Vec::new();
  let mut from = 0;

  for start in starts {
    sections.push(format!("{}\n", lines[from..start].join("\n")));
    from = start;
  }

  sections.push(lines[from..].join("\n"));

  sections
}
