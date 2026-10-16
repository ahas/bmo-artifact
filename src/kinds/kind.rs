//! The shape every kind of document answers (`docs/archive.md`, "Kinds"), and the outline most of them make
//! of their own headings.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use serde_json::{Value, json};

use crate::markup::finding_markdown::finding_markdown;
use crate::markup::markdown_structure::{MarkdownHeading, markdown_headings};
use crate::wording::search::Search;

/// One piece of a document's body: its words, and where it stands among the document's pieces. A search has
/// many; a document the agent writes has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineFinding {
  pub text: String,
  pub place: i64,
}

/// What a heading is in a book-like outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutlineLevel {
  Part,
  Chapter,
  Section,
}

/// One line of the table of contents: what it is called, where it goes, how deep it sits, and — for a kind
/// that names its levels — which level it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OutlineEntry {
  pub slug: String,
  pub text: String,
  pub depth: usize,
  pub level: Option<OutlineLevel>,
}

/// A document's contents, and the marks each piece's headings are stamped with in the text, by the piece's
/// place. The two come back together because they have to be the same marks.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Outline {
  pub entries: Vec<OutlineEntry>,
  pub marks: BTreeMap<i64, Vec<String>>,
}

/// A property a document's head may show. The state, the room and the times stand on every document and
/// are no kind's to choose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HeadProperty {
  Question,
  Request,
  Topic,
  Description,
  Keywords,
  Rule,
  Options,
  Depth,
  Source,
}

/// One kind of document.
pub trait Kind: Sync {
  /// The word the row carries.
  fn id(&self) -> &'static str;
  /// Whether the document opens with the agent's summary.
  fn summary(&self) -> bool;
  /// Whether a document of this kind is the page it keeps, opened as that page wherever it is opened.
  fn hosted(&self) -> bool {
    false
  }
  /// Whether a search run may write its result as this kind, by naming it on the result's `KIND:` line.
  fn searched(&self) -> bool {
    false
  }
  /// The properties its head shows.
  fn head(&self) -> &'static [HeadProperty];
  /// Its table of contents and the marks its headings are stamped with, in the words the wording file gives
  /// a finding's parts.
  fn outline(&self, findings: &[OutlineFinding], words: &Search) -> Outline;

  /// The key the console's language packs name its label by.
  fn label_key(&self) -> String {
    format!("archive.kind.{}", self.id())
  }

  /// The kind as the read API answers it.
  fn view(&self) -> Value {
    json!({
      "id": self.id(),
      "labelKey": self.label_key(),
      "summary": self.summary(),
      "hosted": self.hosted(),
      "searched": self.searched(),
      "head": self.head(),
    })
  }
}

static MORE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^[ \t]*MORE:[ \t]*$").expect("a pattern"));

/// A piece split where its long explanation begins: what stands open, and what is folded under it. What is
/// folded has no entry in any kind's contents.
pub fn folded_parts(text: &str) -> (String, Option<String>) {
  let Some(at) = MORE.find(text) else { return (text.to_owned(), None) };
  let folded = text[at.end()..].trim();

  (
    text[..at.start()].trim_end().to_owned(),
    (!folded.is_empty()).then(|| folded.to_owned()),
  )
}

// Below the deepest heading markdown has, so an authored heading always ranks under a structure one.
const AUTHORED_BELOW: usize = 10;

// The drawn headings, each that an authored line wrote given that line's depth.
fn with_authored_depths(drawn: &[MarkdownHeading], authored: &[MarkdownHeading]) -> Vec<MarkdownHeading> {
  let mut next = 0;

  drawn
    .iter()
    .map(|heading| match authored.get(next) {
      Some(own) if own.text == heading.text => {
        next += 1;

        MarkdownHeading {
          depth: AUTHORED_BELOW + own.depth,
          ..heading.clone()
        }
      }
      _ => heading.clone(),
    })
    .collect()
}

/// What the console draws of a piece, and the headings in it with the marks they are stamped with. The marks
/// carry the piece's place, since two pieces often hold a heading of the same words.
pub fn piece_headings(
  finding: &OutlineFinding,
  words: &Search,
) -> (String, Vec<MarkdownHeading>, Vec<String>) {
  let source = finding_markdown(&folded_parts(&finding.text).0, words);
  let headings = markdown_headings(&source);
  let slugs = headings.iter().map(|heading| format!("{}-{}", finding.place, heading.slug)).collect();

  (source, headings, slugs)
}

/// A contents made of the body's own headings, their levels kept by rank: the shallowest heading depth in
/// the document is the first level. `levels` names the ranks from the top and says how many are kept; a
/// document with fewer ranks takes the lower names. None keeps every rank and names none.
pub fn heading_outline(
  findings: &[OutlineFinding],
  levels: Option<&[OutlineLevel]>,
  words: &Search,
) -> Outline {
  let pieces: Vec<(i64, Vec<MarkdownHeading>, Vec<String>)> = findings
    .iter()
    .map(|finding| {
      let (_, drawn, slugs) = piece_headings(finding, words);
      let authored = markdown_headings(&folded_parts(&finding.text).0);

      (finding.place, with_authored_depths(&drawn, &authored), slugs)
    })
    .collect();
  let ranks: Vec<usize> = pieces
    .iter()
    .flat_map(|(_, headings, _)| headings.iter().map(|heading| heading.depth))
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect();
  let kept: Vec<usize> = match levels {
    None => ranks,
    Some(levels) => ranks.into_iter().take(levels.len()).collect(),
  };
  let names: &[OutlineLevel] = match levels {
    None => &[],
    Some(levels) => &levels[levels.len() - kept.len()..],
  };
  let mut outline = Outline::default();

  for (place, headings, slugs) in pieces {
    for (at, heading) in headings.iter().enumerate() {
      if let Some(rank) = kept.iter().position(|depth| *depth == heading.depth) {
        outline.entries.push(OutlineEntry {
          slug: slugs[at].clone(),
          text: heading.text.clone(),
          depth: rank + 1,
          level: names.get(rank).copied(),
        });
      }
    }

    outline.marks.insert(place, slugs);
  }

  outline
}
