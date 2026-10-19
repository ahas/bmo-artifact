//! What the writer hands back (`docs/news-brief.md`, "How one page is made"): one JSON object holding the
//! summary, the items and the three fixed sections, read out of the answer and checked against the limits
//! the page is drawn to. An answer that is not that object, or breaks a limit, is no page, and the refusal
//! names where, up to five places.

use serde_json::{Map, Value};

use crate::helpers::js;

/// One article's card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
  pub tag: String,
  pub title: String,
  pub lines: Vec<String>,
  pub url: String,
  /// The picture the writer named for it, where it named one.
  pub image: Option<String>,
}

/// A stock that moved, and by how much, as the writer put it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mover {
  pub name: String,
  pub moved: String,
}

/// A fixed section: its lines, its sources, and for a market its movers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Section {
  pub lines: Vec<String>,
  pub sources: Vec<String>,
  pub movers: Vec<Mover>,
}

/// The page's words, as the writer wrote them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brief {
  pub summary: Vec<String>,
  pub items: Vec<Item>,
  pub real_estate: Section,
  pub korea_market: Section,
  pub us_market: Section,
}

// How many places a refusal names.
const NAMED_FAULTS: usize = 5;

// A line of the page, and the bounds of each field, as the page is drawn to them.
const LINE_CHARS: (usize, usize) = (1, 160);
const TAG_CHARS: (usize, usize) = (1, 8);
const TITLE_CHARS: (usize, usize) = (1, 60);
const NAME_CHARS: (usize, usize) = (1, 24);
const MOVE_CHARS: (usize, usize) = (1, 16);

// The faults found so far, each by where it stands.
#[derive(Default)]
struct Faults(Vec<String>);

impl Faults {
  fn at(&mut self, path: &[String], why: impl std::fmt::Display) {
    self.0.push(format!("{} {why}", path.join(".")));
  }
}

fn joined(path: &[String], name: impl ToString) -> Vec<String> {
  path.iter().cloned().chain(std::iter::once(name.to_string())).collect()
}

// Characters as the page's limits count them: the UTF-16 units a browser counts.
fn length(text: &str) -> usize {
  text.encode_utf16().count()
}

fn text(
  value: Option<&Value>,
  path: &[String],
  bounds: (usize, usize),
  faults: &mut Faults,
) -> Option<String> {
  match value {
    None | Some(Value::Null) => {
      faults.at(path, "Required");
      None
    }
    Some(Value::String(raw)) => {
      let trimmed = js::trim(raw).to_owned();
      let count = length(&trimmed);

      if count < bounds.0 {
        faults.at(path, format!("String must contain at least {} character(s)", bounds.0));
        None
      } else if count > bounds.1 {
        faults.at(path, format!("String must contain at most {} character(s)", bounds.1));
        None
      } else {
        Some(trimmed)
      }
    }
    Some(_) => {
      faults.at(path, "Expected string");
      None
    }
  }
}

fn address(value: Option<&Value>, path: &[String], faults: &mut Faults) -> Option<String> {
  let read = text(value, path, (0, usize::MAX), faults)?;

  if read.starts_with("http://") || read.starts_with("https://") {
    Some(read)
  } else {
    faults.at(path, "an address opens with http:// or https://");
    None
  }
}

// A list, its length within bounds, each entry read; a list left out is the empty one where `optional`.
fn list<T>(
  value: Option<&Value>,
  path: &[String],
  bounds: (usize, usize),
  optional: bool,
  faults: &mut Faults,
  mut entry: impl FnMut(&Value, &[String], &mut Faults) -> Option<T>,
) -> Option<Vec<T>> {
  let entries = match value {
    None | Some(Value::Null) if optional => return Some(vec![]),
    None | Some(Value::Null) => {
      faults.at(path, "Required");
      return None;
    }
    Some(Value::Array(entries)) => entries,
    Some(_) => {
      faults.at(path, "Expected array");
      return None;
    }
  };

  if entries.len() < bounds.0 {
    faults.at(path, format!("Array must contain at least {} element(s)", bounds.0));
    return None;
  }

  if entries.len() > bounds.1 {
    faults.at(path, format!("Array must contain at most {} element(s)", bounds.1));
    return None;
  }

  let read: Vec<Option<T>> = entries
    .iter()
    .enumerate()
    .map(|(at, one)| entry(one, &joined(path, at), faults))
    .collect();

  read.into_iter().collect()
}

fn object<'v>(
  value: Option<&'v Value>,
  path: &[String],
  faults: &mut Faults,
) -> Option<&'v Map<String, Value>> {
  match value {
    Some(Value::Object(fields)) => Some(fields),
    None | Some(Value::Null) => {
      faults.at(path, "Required");
      None
    }
    Some(_) => {
      faults.at(path, "Expected object");
      None
    }
  }
}

fn lines(value: Option<&Value>, path: &[String], most: usize, faults: &mut Faults) -> Option<Vec<String>> {
  list(value, path, (1, most), false, faults, |one, at, faults| {
    text(Some(one), at, LINE_CHARS, faults)
  })
}

fn item(value: &Value, path: &[String], faults: &mut Faults) -> Option<Item> {
  let fields = object(Some(value), path, faults)?;
  let field = |name: &str| fields.get(name);
  let tag = text(field("tag"), &joined(path, "tag"), TAG_CHARS, faults);
  let title = text(field("title"), &joined(path, "title"), TITLE_CHARS, faults);
  let lines = lines(field("lines"), &joined(path, "lines"), 3, faults);
  let url = address(field("url"), &joined(path, "url"), faults);
  let image = match field("image") {
    None | Some(Value::Null) => Some(None),
    named => address(named, &joined(path, "image"), faults).map(Some),
  };

  Some(Item {
    tag: tag?,
    title: title?,
    lines: lines?,
    url: url?,
    image: image?,
  })
}

fn mover(value: &Value, path: &[String], faults: &mut Faults) -> Option<Mover> {
  let fields = object(Some(value), path, faults)?;
  let name = text(fields.get("name"), &joined(path, "name"), NAME_CHARS, faults);
  let moved = text(fields.get("move"), &joined(path, "move"), MOVE_CHARS, faults);

  Some(Mover {
    name: name?,
    moved: moved?,
  })
}

fn section(value: Option<&Value>, path: &[String], market: bool, faults: &mut Faults) -> Option<Section> {
  let fields = object(value, path, faults)?;
  let lines = lines(fields.get("lines"), &joined(path, "lines"), 4, faults);
  let sources = list(
    fields.get("sources"),
    &joined(path, "sources"),
    (0, 4),
    true,
    faults,
    |one, at, faults| address(Some(one), at, faults),
  );
  let movers = if market {
    list(fields.get("movers"), &joined(path, "movers"), (0, 6), true, faults, mover)
  } else {
    Some(vec![])
  };

  Some(Section {
    lines: lines?,
    sources: sources?,
    movers: movers?,
  })
}

/// The writer's answer as a brief, or what is wrong with it. The answer is one JSON object; one wrapped in a
/// fence or a sentence still has it read, from its first brace to its last.
pub fn read_brief(answer: &str) -> Result<Brief, String> {
  let (Some(start), Some(end)) = (answer.find('{'), answer.rfind('}')) else {
    return Err("the answer holds no JSON object".to_owned());
  };

  if end <= start {
    return Err("the answer holds no JSON object".to_owned());
  }

  let parsed: Value = serde_json::from_str(&answer[start..=end])
    .map_err(|why| format!("the answer's JSON does not parse: {why}"))?;
  let mut faults = Faults::default();
  let read = (|| {
    let fields = object(Some(&parsed), &[], &mut faults)?;
    let at = |name: &str| vec![name.to_owned()];
    let summary = lines(fields.get("summary"), &at("summary"), 4, &mut faults);
    let items = list(fields.get("items"), &at("items"), (6, 12), false, &mut faults, item);
    let real_estate = section(fields.get("realEstate"), &at("realEstate"), false, &mut faults);
    let korea_market = section(fields.get("koreaMarket"), &at("koreaMarket"), true, &mut faults);
    let us_market = section(fields.get("usMarket"), &at("usMarket"), true, &mut faults);

    Some(Brief {
      summary: summary?,
      items: items?,
      real_estate: real_estate?,
      korea_market: korea_market?,
      us_market: us_market?,
    })
  })();

  match read {
    Some(brief) if faults.0.is_empty() => Ok(brief),
    _ => Err(format!(
      "the answer is not a brief: {}",
      faults.0.iter().take(NAMED_FAULTS).cloned().collect::<Vec<_>>().join("; ")
    )),
  }
}
