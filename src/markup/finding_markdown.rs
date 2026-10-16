//! A finding's structure lines turned into the markdown they stand for (`docs/archive.md`, "The shape of a
//! search's result"), as the console draws a finding: a heading for each part and each name, a gallery for
//! pictures written together, a fold for a menu and a map above a place's link. The outline of every kind
//! reads its headings out of this markdown, and a share link's card reads a document's opening words out of
//! the same lines. The structure lines are the result's own syntax; every word drawn beside them is the
//! wording file's.

use std::sync::LazyLock;

use percent_encoding::{percent_decode_str, utf8_percent_encode};
use regex::Regex;

use super::markdown_structure::plain_heading;
use crate::helpers::js::URI_COMPONENT;
use crate::helpers::marks::fill_marks;
use crate::wording::search::Search;

/// The parts a label opens, in the order a result writes them.
pub const PART_LABELS: [&str; 7] =
  ["VERDICT", "SUMMARY", "INTRO", "BODY", "CONCLUSION", "UNCONFIRMED", "MORE"];

static STRUCTURE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r"^[ \t]*(TITLE|NAME|VERDICT|SUMMARY|SECTION|INTRO|BODY|CONCLUSION|UNCONFIRMED|MORE|ADDRESS|MAP|ROADVIEW|MENU|RELEASED|NEWER|STOCK|DEAL|IMAGE|REVIEW|SOURCE(?:[ \t]*\[[^\]]*\])?|ANSWERED|TOPIC|KEYWORDS|RULE|KIND|DOCUMENT):[ \t]*(.*?)[ \t]*$",
  )
  .expect("a pattern")
});
static FOOTNOTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[(\d{1,3}†?)\]").expect("a pattern"));
static SEPARATOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t]*::[ \t]*").expect("a pattern"));
static AUTHORED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(#{1,6})([ \t].*)$").expect("a pattern"));
static FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[ \t]*(```|~~~)").expect("a pattern"));
static ITEM: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[ \t]*===ITEM===[ \t]*$").expect("a pattern"));
static IMAGE_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[ \t]*IMAGE:").expect("a pattern"));
static BLANK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[ \t]*$").expect("a pattern"));
static BLOCK_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([ \t]*):").expect("a pattern"));
static OWN_BLOCK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[ \t]*:::").expect("a pattern"));
static LIST_MARK: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[ \t]*(?:[-*+]|\d+\.)[ \t]+").expect("a pattern"));
static SEARCHED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/search/([^/?#]+)").expect("a pattern"));
static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("a pattern"));
static NOT_PROSE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r"^[ \t]*(?:\||!\[|:::|-(?:[ \t]*-){2,}[ \t]*$|\*(?:[ \t]*\*){2,}[ \t]*$|_(?:[ \t]*_){2,}[ \t]*$)",
  )
  .expect("a pattern")
});
static LEAD: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[ \t]*(?:>[ \t]*)*(?:(?:[-*+]|\d{1,3}[.)])[ \t]+)?").expect("a pattern"));

/// Each footnote mark found in a line — `[3]`, or `[3†]` for a value read only in a search snippet — that
/// is no link of its own, a bracket followed by an address or a colon being one: where it stands, how long
/// it is, and its mark.
pub fn footnotes(line: &str) -> Vec<(usize, usize, String)> {
  FOOTNOTE
    .captures_iter(line)
    .filter_map(|found| {
      let whole = found.get(0)?;
      let next = line[whole.end()..].chars().next();

      (!matches!(next, Some('(' | ':'))).then(|| (whole.start(), whole.end(), found[1].to_owned()))
    })
    .collect()
}

/// A line with each footnote mark replaced by what `to` makes of it.
pub fn replace_footnotes(line: &str, to: impl Fn(&str) -> String) -> String {
  let mut out = String::new();
  let mut at = 0;

  for (start, end, mark) in footnotes(line) {
    out.push_str(&line[at..start]);
    out.push_str(&to(&mark));
    at = end;
  }

  out.push_str(&line[at..]);
  out
}

/// A line's footnote marks as the in-page links they become.
pub fn footnote_marks(line: &str) -> String {
  replace_footnotes(line, |mark| format!("[{mark}](#mark-{})", utf8_percent_encode(mark, URI_COMPONENT)))
}

// What one field line carries, cut at the separator the search writes between its parts.
fn fields(value: &str) -> Vec<String> {
  SEPARATOR.split(value).map(str::to_owned).collect()
}

/// What a picture's line is captioned with where it is shown, out of the fields after its address: what it
/// shows, then its shot and whose picture it is where the line names them. The entry, the third field, is
/// the heading the picture already stands under.
pub fn picture_caption(captioned: &str) -> String {
  let fields = fields(captioned.trim());
  let field = |at: usize| fields.get(at).map_or("", String::as_str);

  [field(0), field(2), field(3)]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" · ")
}

/// What each part a label opens is called.
pub fn part_name<'w>(words: &'w Search, key: &'w str) -> Option<&'w str> {
  PART_LABELS.contains(&key).then(|| words.parts.get(key).map_or(key, String::as_str))
}

// The level the shallowest heading the search wrote itself stands at, under structure headings and where
// there are none.
const AUTHORED_FROM: usize = 5;
const AUTHORED_ALONE_FROM: usize = 3;

// Whether a structure line is drawn as a heading: a name, a section that says what it holds, or a part
// label — every part but a verdict or a summary that carries its words on the same line.
fn draws_heading(key: &str, value: &str) -> bool {
  key == "TITLE"
    || key == "NAME"
    || (key == "SECTION" && !value.is_empty())
    || (PART_LABELS.contains(&key) && !(inline(key) && !value.is_empty()))
}

fn inline(key: &str) -> bool {
  key == "VERDICT" || key == "SUMMARY"
}

// How far the headings a finding wrote itself move down: from its shallowest to the fifth level under
// structure headings, to the third where there are none. A heading already deeper stays.
fn authored_shift(lines: &[&str]) -> usize {
  let mut fenced = false;
  let mut shallowest = 7;
  let mut structured = false;

  for line in lines {
    if FENCE.is_match(line) {
      fenced = !fenced;
    } else if !fenced {
      shallowest = shallowest.min(AUTHORED.captures(line).map_or(7, |found| found[1].len()));

      if let Some(structure) = STRUCTURE.captures(line)
        && draws_heading(&structure[1], &structure[2])
      {
        structured = true;
      }
    }
  }

  if shallowest == 7 {
    0
  } else {
    (if structured { AUTHORED_FROM } else { AUTHORED_ALONE_FROM }).saturating_sub(shallowest)
  }
}

// The part of an entry a line stands in: its address and its name, read out of the whole of it.
#[derive(Clone, Default)]
struct EntryPlace {
  address: Option<String>,
  name: Option<String>,
}

fn entry_places(lines: &[&str]) -> Vec<std::rc::Rc<std::cell::RefCell<EntryPlace>>> {
  let mut places = Vec::new();
  let mut current = std::rc::Rc::new(std::cell::RefCell::new(EntryPlace::default()));

  for line in lines {
    let structure = STRUCTURE.captures(line);
    let key = structure.as_ref().map(|found| found[1].to_owned());

    if ITEM.is_match(line) || key.as_deref() == Some("NAME") {
      current = std::rc::Rc::new(std::cell::RefCell::new(EntryPlace::default()));
    }

    if let Some(found) = &structure {
      let mut place = current.borrow_mut();

      if &found[1] == "ADDRESS" && place.address.is_none() {
        place.address = Some(found[2].to_owned());
      }

      if &found[1] == "NAME" {
        place.name = Some(found[2].to_owned());
      }
    }

    places.push(std::rc::Rc::clone(&current));
  }

  places
}

// The words the map of a place is found by: the entry's address, else what its map link searches for, else
// the entry's name. Footnote marks are not part of any of them.
fn map_query(link: &str, place: Option<&EntryPlace>) -> Option<String> {
  let address = place
    .and_then(|place| place.address.as_deref())
    .map(|address| replace_footnotes(address, |_| String::new()).trim().to_owned())
    .unwrap_or_default();

  if !address.is_empty() {
    return Some(address);
  }

  if let Some(searched) = SEARCHED.captures(link)
    && let Ok(words) = percent_decode_str(&searched[1]).decode_utf8()
    && !words.trim().is_empty()
  {
    return Some(words.trim().to_owned());
  }

  place
    .and_then(|place| place.name.as_deref())
    .map(str::trim)
    .filter(|name| !name.is_empty())
    .map(str::to_owned)
}

// A menu as a table: a dish and its price to a row, with a column for notes where a dish has one, and a
// line with no price a group row in bold. A cell keeps a bar the search wrote as a bar.
fn menu_table(items: &[&str], words: &Search) -> Vec<String> {
  let rows: Vec<Vec<String>> = items
    .iter()
    .map(|item| {
      fields(LIST_MARK.replace(item, "").trim())
        .iter()
        .map(|cell| footnote_marks(&cell.replace('|', "\\|")))
        .collect()
    })
    .collect();
  let noted = rows.iter().any(|row| row.len() > 2);
  let reading = &words.reading;
  let mut columns = vec![reading.dish.clone(), reading.price.clone()];

  if noted {
    columns.push(reading.notes.clone());
  }

  let cells = |row: &[String]| format!("| {} |", row.join(" | "));
  let mut table = vec![cells(&columns), format!("| --- | --- |{}", if noted { " --- |" } else { "" })];

  for row in rows {
    let dish = row.first().cloned().unwrap_or_default();

    table.push(match row.get(1) {
      None => {
        let mut group = vec![format!("**{dish}**"), String::new()];

        if noted {
          group.push(String::new());
        }

        cells(&group)
      }
      Some(price) => {
        let mut line = vec![dish, price.clone()];

        if noted {
          line.push(row[2..].join(" · "));
        }

        cells(&line)
      }
    });
  }

  table
}

/// A finding's structure lines as markdown. A picture is a paragraph of its own and a set's divider is a
/// rule, so both are set apart by blank lines. Pictures written together are one gallery, a menu is a fold
/// of its own, and a map line draws the place's map above its link.
pub fn finding_markdown(text: &str, words: &Search) -> String {
  let lines: Vec<&str> = text.split('\n').collect();
  let shift = authored_shift(&lines);
  let places = entry_places(&lines);
  let mut out: Vec<String> = Vec::new();
  let mut fenced = false;
  let mut at = 0;

  while at < lines.len() {
    let line = lines[at];
    let index = at;

    at += 1;

    if FENCE.is_match(line) {
      fenced = !fenced;
      out.push(line.to_owned());
      continue;
    }

    if fenced {
      out.push(line.to_owned());
      continue;
    }

    // Pictures written one after another, blank lines between them allowed, are one gallery.
    if IMAGE_LINE.is_match(line) {
      let mut run = vec![line];
      let mut next = at;

      while next < lines.len() && (IMAGE_LINE.is_match(lines[next]) || BLANK.is_match(lines[next])) {
        if IMAGE_LINE.is_match(lines[next]) {
          run.push(lines[next]);
          at = next + 1;
        }

        next += 1;
      }

      let pictures: Vec<String> = run
        .iter()
        .map(|picture| {
          let value = STRUCTURE.captures(picture).map(|found| found[2].to_owned()).unwrap_or_default();

          structure_line("IMAGE", &value, words).unwrap_or_default().trim().to_owned()
        })
        .collect();

      out.push(if pictures.len() == 1 {
        format!("\n{}\n", pictures[0])
      } else {
        format!("\n::: gallery\n{}\n:::\n", pictures.join("\n"))
      });
      continue;
    }

    let opening = STRUCTURE.captures(line);
    let key = opening.as_ref().map(|found| found[1].to_owned());
    let value = opening.as_ref().map(|found| found[2].to_owned()).unwrap_or_default();

    // A menu runs to the next structure line, divider, heading or fence, and is folded whole.
    if key.as_deref() == Some("MENU") {
      let mut items = Vec::new();

      while at < lines.len()
        && !STRUCTURE.is_match(lines[at])
        && !ITEM.is_match(lines[at])
        && !AUTHORED.is_match(lines[at])
        && !FENCE.is_match(lines[at])
      {
        if !BLANK.is_match(lines[at]) {
          items.push(lines[at]);
        }

        at += 1;
      }

      let mut body = Vec::new();

      if !value.is_empty() {
        body.extend([footnote_marks(&value), String::new()]);
      }

      if !items.is_empty() {
        body.extend(menu_table(&items, words));
      }

      if !body.is_empty() {
        out.push(format!("\n::: fold {}\n{}\n:::\n", words.reading.menu, body.join("\n")));
      }

      continue;
    }

    if key.as_deref() == Some("MAP") {
      let place = places[index].borrow().clone();
      let link = structure_line("MAP", &value, words).unwrap_or_default();

      out.push(match map_query(&value, Some(&place)) {
        None => link,
        Some(query) => format!("\n::: map {}\n\n{link}", SPACES.replace_all(&query, " ")),
      });
      continue;
    }

    if let Some(drawn) = line_markdown(line, shift, words) {
      out.push(drawn);
    }
  }

  out.join("\n")
}

// One line of a finding that stands on its own: a divider, a heading the search wrote, a line of its words,
// or a structure line. None for a line that is not drawn.
fn line_markdown(line: &str, shift: usize, words: &Search) -> Option<String> {
  if ITEM.is_match(line) {
    return Some("\n***\n".to_owned());
  }

  // A line the search wrote in the form a finding block takes stays its own words.
  if OWN_BLOCK.is_match(line) {
    return Some(footnote_marks(&BLOCK_LINE.replace(line, "$1\\:")));
  }

  if let Some(authored) = AUTHORED.captures(line) {
    let depth = (authored[1].len() + shift).min(6);

    return Some(footnote_marks(&format!("{}{}", "#".repeat(depth), &authored[2])));
  }

  match STRUCTURE.captures(line) {
    None => Some(footnote_marks(line)),
    Some(structure) => structure_line(&structure[1], &structure[2], words),
  }
}

// A structure line as the markdown it stands for, and none for one that is not drawn.
fn structure_line(key: &str, value: &str, words: &Search) -> Option<String> {
  let reading = &words.reading;

  if let Some(part) = part_name(words, key)
    && value.is_empty()
  {
    return Some(format!("\n#### {part}\n"));
  }

  let part = || part_name(words, key).unwrap_or(key);

  Some(match key {
    "TITLE" | "NAME" => format!("\n### {value}\n"),
    // A section of the middle, headed by the name the search gave it for what it holds.
    "SECTION" => return (!value.is_empty()).then(|| format!("\n#### {value}\n")),
    "VERDICT" | "SUMMARY" => format!("\n**{}** {}", part(), footnote_marks(value)),
    "INTRO" | "BODY" | "CONCLUSION" | "UNCONFIRMED" | "MORE" => {
      format!("\n#### {}\n\n{}", part(), footnote_marks(value))
    }
    "ADDRESS" => format!("**{}** {value}", reading.address),
    "MAP" => format!("[{}](<{value}>)", reading.map),
    "ROADVIEW" => format!("[{}](<{value}>)", reading.street_view),
    "RELEASED" => format!("**{}** {}", reading.released, footnote_marks(value)),
    "NEWER" => format!("**{}** {}", reading.newer, footnote_marks(value)),
    // What the product page said of itself, when it was read and where.
    "STOCK" => {
      let fields = fields(value);
      let field = |at: usize| fields.get(at).map_or("", String::as_str);
      let when = if field(1).is_empty() { String::new() } else { format!(" · {}", field(1)) };
      let page = field(2);
      let where_ = if page.is_empty() {
        String::new()
      } else {
        format!(" [{}](<{page}>)", reading.product_page)
      };

      format!("**{}** {}{when}{where_}", reading.stock, footnote_marks(field(0)))
    }
    // An offer against what the thing usually costs, and the search's own word on that gap.
    "DEAL" => {
      let fields = fields(value);
      let field = |at: usize| fields.get(at).map_or("", String::as_str);
      let against = if field(1).is_empty() {
        String::new()
      } else {
        format!(" ({})", fill_marks(&reading.usual, &[("usual", field(1))]))
      };
      let real = if field(2).is_empty() {
        String::new()
      } else {
        format!(" · {}", footnote_marks(field(2)))
      };

      format!("**{}** {}{against}{real}", reading.deal, footnote_marks(field(0)))
    }
    "IMAGE" => {
      let fields = fields(value);
      let href = fields.first().cloned().unwrap_or_default();
      let caption = picture_caption(&fields[1.min(fields.len())..].join(" :: "));
      let escaped = caption.replace('[', "\\[").replace(']', "\\]");

      format!("\n![{escaped}](<{href}>)\n")
    }
    // One review as it was written: its rating, the reviewer's own words and the page they stand on, set as
    // a quotation so it reads as somebody else's words.
    "REVIEW" => {
      let fields = fields(value);
      let field = |at: usize| fields.get(at).map_or("", String::as_str);
      let rating = field(0);
      let scored = if rating.is_empty() || rating == words.field("no_rating") {
        String::new()
      } else {
        format!("**{rating}** ")
      };
      let page = field(2);
      let where_ = if page.is_empty() { String::new() } else { format!(" [{}](<{page}>)", reading.review) };

      format!("\n> {scored}{}{where_}\n", footnote_marks(field(1)))
    }
    _ => return None,
  })
}

// The prose lines of one finding, in order and as plain words. A part label carrying its words on its own
// line gives those words; every other structure line, a heading, a menu and whatever else is drawn rather
// than said gives none. What is folded under the fold line is past the opening.
fn prose_lines(text: &str) -> Vec<String> {
  let mut prose = Vec::new();
  let mut fenced = false;
  let mut menu = false;

  for line in text.split('\n') {
    if FENCE.is_match(line) {
      fenced = !fenced;
      continue;
    }

    let structure = if fenced { None } else { STRUCTURE.captures(line) };

    if menu && structure.is_none() && !ITEM.is_match(line) && !AUTHORED.is_match(line) {
      continue;
    }

    let key = structure.as_ref().map(|found| found[1].to_owned());

    menu = key.as_deref() == Some("MENU");

    if key.as_deref() == Some("MORE") {
      break;
    }

    if fenced
      || BLANK.is_match(line)
      || ITEM.is_match(line)
      || AUTHORED.is_match(line)
      || NOT_PROSE.is_match(line)
    {
      continue;
    }

    let said = match &structure {
      None => line.to_owned(),
      Some(found) if PART_LABELS.contains(&&found[1]) => found[2].to_owned(),
      Some(_) => String::new(),
    };
    let unmarked = replace_footnotes(&LEAD.replace(&said, ""), |_| String::new());
    let plain = SPACES.replace_all(&plain_heading(&unmarked), " ").trim().to_owned();

    if !plain.is_empty() {
      prose.push(plain);
    }
  }

  prose
}

/// The words a set of findings opens with, as one line of plain prose no longer than `limit` characters:
/// what a place that shows a document by a few words and draws no markup is given, such as the card a chat
/// app draws for a link to it. A line too long is cut at the last space before the limit and closed with an
/// ellipsis. Empty where the findings say nothing in prose.
pub fn finding_excerpt(texts: &[&str], limit: usize) -> String {
  let mut lines = Vec::new();
  let mut length = 0;

  for line in texts.iter().flat_map(|text| prose_lines(text)) {
    if length > limit {
      break;
    }

    length += line.chars().count() + 1;
    lines.push(line);
  }

  let joined = lines.join(" ");
  let chars: Vec<char> = joined.chars().collect();

  if chars.len() <= limit {
    return joined;
  }

  let cut: String = chars[..limit.saturating_sub(1)].iter().collect();
  let kept = match cut.rfind(' ') {
    Some(space) if cut[..space].chars().count() > limit / 2 => &cut[..space],
    _ => cut.as_str(),
  };

  format!("{}…", kept.trim_end())
}
