//! The document wording file (`docs/api.md`, "The document wording"): the lines the service writes into a
//! document a person reads, each with the `{name}` marks it is filled in with. The settings name the file;
//! where they name none, the English default in `lang/` is read. A file holds every key the English default
//! holds and no other.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The English default, as it ships with the repository.
pub const ENGLISH: &str = include_str!("../lang/documents.en.yaml");

/// Every line a document is written with.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Wording {
  pub import: Import,
}

/// What an imported page's document says of itself beside the page's own words.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Import {
  /// A figure of the page that could not be kept, by its number `{n}` and its `{caption}`.
  pub missing: String,
}

/// Why the document wording could not be read, with the file it was read from.
#[derive(Debug, thiserror::Error)]
#[error("the document wording file {place}: {why}")]
pub struct WordingError {
  pub place: String,
  pub why: String,
}

impl Wording {
  /// The English default.
  pub fn english() -> Result<Self, WordingError> {
    Self::from_text(ENGLISH, "lang/documents.en.yaml")
  }

  /// The file the settings name, or the English default where they name none.
  pub fn read(file: Option<&Path>) -> Result<Self, WordingError> {
    let Some(file) = file else { return Self::english() };
    let place = file.display().to_string();
    let text = std::fs::read_to_string(file).map_err(|why| WordingError {
      place: place.clone(),
      why: why.to_string(),
    })?;

    Self::from_text(&text, &place)
  }

  /// A wording file's text, held to the English default's keys. `place` names it in a refusal.
  pub fn from_text(text: &str, place: &str) -> Result<Self, WordingError> {
    let refused = |why: String| WordingError {
      place: place.to_owned(),
      why,
    };
    let read: Value = serde_saphyr::from_str(text).map_err(|why| refused(why.to_string()))?;
    let english: Value = serde_saphyr::from_str(ENGLISH).map_err(|why| refused(why.to_string()))?;
    let held = keys(&read);
    let wanted = keys(&english);
    let missing: Vec<&str> = wanted.difference(&held).map(String::as_str).collect();
    let unknown: Vec<&str> = held.difference(&wanted).map(String::as_str).collect();

    if !missing.is_empty() {
      return Err(refused(format!("it lacks {}", missing.join(", "))));
    }

    if !unknown.is_empty() {
      return Err(refused(format!("no line is read under {}", unknown.join(", "))));
    }

    serde_json::from_value(read).map_err(|why| refused(why.to_string()))
  }
}

/// Every line's key in a wording file read whole, its groups joined by dots: `import.missing`.
pub fn keys(value: &Value) -> BTreeSet<String> {
  let mut found = BTreeSet::new();

  walk(value, "", &mut found);
  found
}

fn walk(value: &Value, at: &str, found: &mut BTreeSet<String>) {
  match value {
    Value::Object(fields) => {
      for (name, value) in fields {
        let key = if at.is_empty() { name.clone() } else { format!("{at}.{name}") };

        walk(value, &key, found);
      }
    }
    _ => {
      found.insert(at.to_owned());
    }
  }
}
