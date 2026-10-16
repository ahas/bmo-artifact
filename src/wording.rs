//! The document wording file (`docs/api.md`, "Wording"): what a document this service writes says of
//! itself, and what a search leaves for a person to read, each line with the `{name}` marks it is filled
//! in with. The settings name the file; where they name none, the English default in `lang/` is read. A
//! file holds every key the English default holds and no other.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The English default, as it ships with the repository.
pub const ENGLISH: &str = include_str!("../lang/documents.en.yaml");

/// Every line a document or a search's notice is written with.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Wording {
  pub archive_docs: archive_docs::ArchiveDocs,
  pub search: search::Search,
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

/// Every line's key in a wording file read whole, its groups joined by dots: `search.head.query`.
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

/// What a document this service writes says of itself: the header its file opens with under its title,
/// what each kind of document is called, and how its moment is told.
pub mod archive_docs {
  use super::*;

  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(deny_unknown_fields)]
  pub struct ArchiveDocs {
    pub header: Header,
    /// What each kind of document is called, by the kind's name.
    pub kinds: BTreeMap<String, String>,
    pub time: Time,
  }

  /// The lines a document's file opens with under its title.
  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(deny_unknown_fields)]
  pub struct Header {
    pub kind: String,
    pub request: String,
    pub written: String,
    pub attached: String,
  }

  /// A moment as the documents tell it, in the time zone the settings name.
  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(deny_unknown_fields)]
  pub struct Time {
    pub line: String,
    pub morning: String,
    pub afternoon: String,
  }
}

/// What a search leaves for a person to read: its document's head, result and trail, the words each part
/// and field of a result is written as, and the result a page brought in whole is written as.
pub mod search {
  use super::*;

  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(deny_unknown_fields)]
  pub struct Head {
    pub untitled: String,
    pub query: String,
    pub topic: String,
    pub keywords: String,
    pub rule: String,
    pub option: String,
    pub depth: String,
    pub status: String,
    pub started: String,
    pub finished: String,
    pub runs: String,
  }

  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(deny_unknown_fields)]
  pub struct Result {
    pub heading: String,
    pub empty: String,
    pub branch: String,
    pub sources: String,
  }

  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(deny_unknown_fields)]
  pub struct Trail {
    pub heading: String,
    pub summary: String,
    pub own: String,
    pub run: String,
    pub asked: String,
    pub opened: String,
    pub found: String,
  }

  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(deny_unknown_fields)]
  pub struct Import {
    pub answered: String,
    pub topic: String,
    pub missing: String,
    pub kept: String,
    pub moved: String,
    pub cited: String,
    pub cited_text: String,
  }

  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(deny_unknown_fields)]
  pub struct Search {
    pub archive_name: String,
    pub import_query: String,
    pub head: Head,
    pub result: Result,
    pub trail: Trail,
    /// What each fixed part of a body is called, by its label.
    pub parts: BTreeMap<String, String>,
    /// What each line of an entry is written as, by its label, beside the pieces a deal, a review and a
    /// kept picture are made of.
    pub fields: BTreeMap<String, String>,
    pub import: Import,
  }

  impl Search {
    /// One field's words by its name; every name is in every file, since a file is held to the English
    /// default's keys.
    pub fn field(&self, name: &str) -> &str {
      self.fields.get(name).map_or("", String::as_str)
    }
  }
}
