//! `search`: what a search found, one finding at a time. It opens with the key answer and the summary, and its
//! contents go down to the sections inside each finding — each section, each part label, each candidate's
//! name.

use std::sync::LazyLock;

use regex::Regex;

use super::kind::{HeadProperty, Kind, Outline, OutlineEntry, OutlineFinding, piece_headings};
use crate::helpers::marks::fill_marks;
use crate::markup::finding_markdown::{part_name, replace_footnotes};
use crate::wording::search::Search;

static NAMED: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[ \t]*(?:TITLE|NAME):[ \t]*(.+?)[ \t]*$").expect("a pattern"));
static SECTION: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[ \t]*SECTION:[ \t]*(.+?)[ \t]*$").expect("a pattern"));
static PART: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^[ \t]*(VERDICT|SUMMARY|INTRO|BODY|CONCLUSION|UNCONFIRMED):[ \t]*$").expect("a pattern")
});
static STRUCTURE_OPENING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z]+:").expect("a pattern"));
// Written emphasis and code marks around words, which a line standing as a title drops. Kept as the console
// reads a title, with look-around and a back-reference, since a markdown library leaves emphasis open beside
// CJK punctuation.
static EMPHASIS: LazyLock<fancy_regex::Regex> =
  LazyLock::new(|| fancy_regex::Regex::new(r"(\*\*|__|\*|`)(?=\S)(.+?)(?<=\S)\1").expect("a pattern"));

// The longest a finding's own line runs before it is cut.
const TITLE_CHARS: usize = 60;

/// The name a finding gave itself on its title or name line, and none where it gave none.
pub fn finding_name(text: &str) -> Option<String> {
  text.split('\n').find_map(|line| NAMED.captures(line).map(|found| found[1].to_owned()))
}

/// What a finding is called where it stands as a line of its own: its own title or name line, else the part
/// or the section it opens with, else the first thing it says, cut to a line.
pub fn finding_title(finding: &OutlineFinding, words: &Search) -> String {
  let lines: Vec<&str> = finding.text.split('\n').collect();
  let opening = lines.iter().find(|line| !line.trim().is_empty()).copied().unwrap_or("");
  let part = PART.captures(opening).and_then(|found| part_name(words, &found[1]).map(str::to_owned));
  let first = finding_name(&finding.text)
    .or(part)
    .or_else(|| SECTION.captures(opening).map(|found| found[1].to_owned()))
    .or_else(|| {
      lines
        .iter()
        .map(|line| line.trim())
        .find(|line| !line.is_empty() && !STRUCTURE_OPENING.is_match(line))
        .map(str::to_owned)
    });
  let first =
    first.unwrap_or_else(|| fill_marks(&words.reading.untitled, &[("place", &finding.place.to_string())]));
  let unmarked = replace_footnotes(&first, |_| String::new());
  let title = EMPHASIS.replace_all(&unmarked, "$2").into_owned();
  let chars: Vec<char> = title.chars().collect();

  if chars.len() > TITLE_CHARS {
    format!("{}…", chars[..TITLE_CHARS].iter().collect::<String>())
  } else {
    title
  }
}

/// What one finding puts in the table of contents, and the marks its headings are stamped with.
///
/// The finding's own line is its first heading where the text opens with one — an entry's name line, a part
/// label — since a line above it saying the same words twice is what a contents list is meant to save the
/// reader. Where it opens with something else, the line stands for the finding as a whole and the headings
/// sit under it.
pub fn finding_contents(finding: &OutlineFinding, words: &Search) -> (Vec<OutlineEntry>, Vec<String>) {
  let (source, headings, slugs) = piece_headings(finding, words);
  let own_line = || OutlineEntry {
    slug: finding.place.to_string(),
    text: finding_title(finding, words),
    depth: 1,
    level: None,
  };

  let Some(first) = headings.first() else { return (vec![own_line()], slugs) };
  let opens_with = source.split('\n').take(first.line - 1).all(|line| line.trim().is_empty());
  let top = if opens_with {
    OutlineEntry {
      slug: slugs[0].clone(),
      text: first.text.clone(),
      depth: 1,
      level: None,
    }
  } else {
    own_line()
  };
  let (under, from) = if opens_with { (&headings[1..], 1) } else { (&headings[..], 0) };

  if under.is_empty() {
    return (vec![top], slugs);
  }

  // What sits under it keeps its own nesting, measured from the shallowest of them rather than from the
  // finding's line.
  let shallowest = under.iter().map(|heading| heading.depth).min().unwrap_or(1);
  let mut entries = vec![top];

  for (at, heading) in under.iter().enumerate() {
    entries.push(OutlineEntry {
      slug: slugs[at + from].clone(),
      text: heading.text.clone(),
      depth: 2 + heading.depth - shallowest,
      level: None,
    });
  }

  (entries, slugs)
}

pub struct SearchKind;

impl Kind for SearchKind {
  fn id(&self) -> &'static str {
    "search"
  }

  fn summary(&self) -> bool {
    true
  }

  fn searched(&self) -> bool {
    true
  }

  fn head(&self) -> &'static [HeadProperty] {
    &[
      HeadProperty::Question,
      HeadProperty::Topic,
      HeadProperty::Keywords,
      HeadProperty::Rule,
      HeadProperty::Options,
      HeadProperty::Depth,
    ]
  }

  fn outline(&self, findings: &[OutlineFinding], words: &Search) -> Outline {
    let mut outline = Outline::default();

    for finding in findings {
      let (entries, slugs) = finding_contents(finding, words);

      outline.entries.extend(entries);
      outline.marks.insert(finding.place, slugs);
    }

    outline
  }
}
