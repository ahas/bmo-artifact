//! The document wording file: every key of the English default renders with the marks it is filled in
//! with, every key is listed in `docs/api.md`, and a file the settings name is held to the same keys
//! (`docs/api.md`, "Wording").

mod support;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use bmo_artifact::helpers::marks::fill_marks;
use bmo_artifact::plumbing::verb::Shell;
use bmo_artifact::settings::Settings;
use bmo_artifact::wording::{ENGLISH, Wording, keys};
use serde_json::Value;
use support::ROOT;

// Every key a wording file holds, with the marks its line is filled in with.
const KEYS: &[(&str, &[&str])] = &[
  ("archive_docs.header.kind", &["kind"]),
  ("archive_docs.header.request", &["request"]),
  ("archive_docs.header.written", &["time"]),
  ("archive_docs.header.attached", &["names"]),
  ("archive_docs.kinds.record", &[]),
  ("archive_docs.kinds.learning", &[]),
  ("archive_docs.kinds.import", &[]),
  ("archive_docs.kinds.page", &[]),
  ("archive_docs.time.line", &["year", "month", "day", "half", "hour", "minute"]),
  ("archive_docs.time.morning", &[]),
  ("archive_docs.time.afternoon", &[]),
  ("search.archive_name", &[]),
  ("search.import_query", &["source"]),
  ("search.head.untitled", &["query"]),
  ("search.head.query", &["value"]),
  ("search.head.topic", &["value"]),
  ("search.head.keywords", &["value"]),
  ("search.head.rule", &["value"]),
  ("search.head.option", &["name", "value"]),
  ("search.head.depth", &["value"]),
  ("search.head.status", &["value"]),
  ("search.head.started", &["value"]),
  ("search.head.finished", &["value"]),
  ("search.head.runs", &["value"]),
  ("search.result.heading", &[]),
  ("search.result.empty", &[]),
  ("search.result.branch", &["branch"]),
  ("search.result.sources", &[]),
  ("search.trail.heading", &[]),
  ("search.trail.summary", &[]),
  ("search.trail.own", &[]),
  ("search.trail.run", &["run"]),
  ("search.trail.asked", &[]),
  ("search.trail.opened", &[]),
  ("search.trail.found", &[]),
  ("search.parts.VERDICT", &[]),
  ("search.parts.SUMMARY", &[]),
  ("search.parts.INTRO", &[]),
  ("search.parts.BODY", &[]),
  ("search.parts.CONCLUSION", &[]),
  ("search.parts.UNCONFIRMED", &[]),
  ("search.parts.MORE", &[]),
  ("search.fields.ADDRESS", &["value"]),
  ("search.fields.MAP", &["value"]),
  ("search.fields.ROADVIEW", &["value"]),
  ("search.fields.MENU", &["value"]),
  ("search.fields.menu_alone", &[]),
  ("search.fields.RELEASED", &["value"]),
  ("search.fields.NEWER", &["value"]),
  ("search.fields.STOCK", &["value"]),
  ("search.fields.deal", &["now", "usual", "real"]),
  ("search.fields.deal_usual", &["usual"]),
  ("search.fields.deal_real", &["real"]),
  ("search.fields.review_link", &["page"]),
  ("search.fields.no_rating", &[]),
  ("search.fields.original", &["address"]),
  ("search.reading.address", &[]),
  ("search.reading.map", &[]),
  ("search.reading.street_view", &[]),
  ("search.reading.menu", &[]),
  ("search.reading.dish", &[]),
  ("search.reading.price", &[]),
  ("search.reading.notes", &[]),
  ("search.reading.released", &[]),
  ("search.reading.newer", &[]),
  ("search.reading.stock", &[]),
  ("search.reading.product_page", &[]),
  ("search.reading.deal", &[]),
  ("search.reading.usual", &["usual"]),
  ("search.reading.review", &[]),
  ("search.reading.untitled", &["place"]),
  ("search.import.answered", &[]),
  ("search.import.topic", &["host"]),
  ("search.import.missing", &["n", "caption"]),
  ("search.import.kept", &[]),
  ("search.import.moved", &[]),
  ("search.import.cited", &[]),
  ("search.import.cited_text", &["text"]),
  ("news_page.language", &[]),
  ("news_page.weekdays.sunday", &[]),
  ("news_page.weekdays.monday", &[]),
  ("news_page.weekdays.tuesday", &[]),
  ("news_page.weekdays.wednesday", &[]),
  ("news_page.weekdays.thursday", &[]),
  ("news_page.weekdays.friday", &[]),
  ("news_page.weekdays.saturday", &[]),
  ("news_page.day", &["year", "month", "date", "weekday"]),
  ("news_page.title", &["day"]),
  ("news_page.news", &[]),
  ("news_page.korea_market", &[]),
  ("news_page.us_market", &[]),
  ("news_page.real_estate", &[]),
  ("news_page.no_close", &[]),
  ("news_page.close_of", &["date"]),
  ("news_page.chart_label", &["name", "first", "last", "count", "low", "high"]),
  ("news_page.original", &["host"]),
  ("news_page.sources", &[]),
  ("news_page.footer", &[]),
  ("news_page.indices.KOSPI", &[]),
  ("news_page.indices.KOSDAQ", &[]),
  ("news_page.indices.DJI", &[]),
  ("news_page.indices.SPX", &[]),
  ("news_page.indices.NASDAQ", &[]),
];

// Each line of a wording file read whole, by its key.
fn lines(value: &Value, at: &str, found: &mut BTreeMap<String, String>) {
  match value {
    Value::Object(fields) => {
      for (name, value) in fields {
        lines(value, &if at.is_empty() { name.clone() } else { format!("{at}.{name}") }, found);
      }
    }
    Value::String(line) => {
      found.insert(at.to_owned(), line.clone());
    }
    other => panic!("{at} holds {other}, not a line"),
  }
}

// The English default's lines as the service reads them: through its own types.
fn english_lines() -> BTreeMap<String, String> {
  let wording = Wording::english().expect("the English default reads");
  let mut found = BTreeMap::new();

  lines(&serde_json::to_value(&wording).expect("the wording writes back"), "", &mut found);
  found
}

#[test]
fn every_key_of_the_english_default_renders_with_its_marks() {
  let lines = english_lines();
  let wanted: BTreeSet<&str> = KEYS.iter().map(|(key, _)| *key).collect();

  assert_eq!(lines.keys().map(String::as_str).collect::<BTreeSet<_>>(), wanted);

  for &(key, marks) in KEYS {
    let values: Vec<(&str, String)> = marks.iter().map(|mark| (*mark, format!("<{mark}>"))).collect();
    let named: Vec<(&str, &str)> = values.iter().map(|(mark, value)| (*mark, value.as_str())).collect();
    let line = &lines[key];
    let rendered = fill_marks(line, &named);

    assert!(!rendered.trim().is_empty(), "{key} renders as nothing");
    assert!(!rendered.contains(['{', '}']), "{key} leaves a mark unfilled: {rendered}");

    for (mark, value) in &values {
      assert!(rendered.contains(value.as_str()), "{key} does not say {{{mark}}}: {line}");
    }
  }
}

#[test]
fn a_field_s_words_are_read_by_its_label() {
  let wording = Wording::english().expect("the English default reads");

  assert_eq!(
    fill_marks(wording.search.field("ADDRESS"), &[("value", "1 Main St")]),
    "Address: 1 Main St"
  );
  assert_eq!(wording.search.field("NO_SUCH_FIELD"), "");
  assert_eq!(wording.archive_docs.kinds["record"], "Record");
}

#[test]
fn every_key_is_listed_in_the_api_document_with_its_marks() {
  let api = fs::read_to_string(Path::new(ROOT).join("docs/api.md")).expect("the API document reads");

  for &(key, marks) in KEYS {
    let row = api
      .lines()
      .find(|line| line.starts_with(&format!("| `{key}` |")))
      .unwrap_or_else(|| panic!("docs/api.md lists no row for {key}"));
    let listed: BTreeSet<String> = row
      .split('|')
      .nth(2)
      .unwrap_or_default()
      .split(',')
      .map(|mark| mark.trim().trim_matches('`').trim_matches(['{', '}']).to_owned())
      .filter(|mark| !mark.is_empty() && mark != "none")
      .collect();

    assert_eq!(listed, marks.iter().map(|mark| (*mark).to_owned()).collect::<BTreeSet<_>>(), "{key}");
  }
}

#[test]
fn the_keys_a_file_is_held_to_are_the_english_default_s() {
  let english: Value = serde_saphyr::from_str(ENGLISH).expect("the English default is YAML");

  assert_eq!(keys(&english), english_lines().keys().cloned().collect());
}

fn written(text: &str) -> (tempfile::TempDir, std::path::PathBuf) {
  let dir = tempfile::tempdir().expect("a folder");
  let path = dir.path().join("documents.yaml");

  fs::write(&path, text).expect("written");
  (dir, path)
}

#[test]
fn a_file_the_settings_name_is_read_in_place_of_the_default() {
  let (_dir, path) = written(&ENGLISH.replace("\"Archive\"", "\"Collection\""));
  let wording = Wording::read(Some(&path)).expect("the file reads");

  assert_eq!(wording.search.archive_name, "Collection");
  assert_eq!(Wording::read(None).expect("the default reads").search.archive_name, "Archive");
}

#[test]
fn a_file_lacking_a_key_is_refused_with_the_key() {
  let (_dir, path) = written(&ENGLISH.replace("    MORE: \"In detail\"\n", ""));
  let refused = Wording::read(Some(&path)).expect_err("refused").to_string();

  assert!(refused.ends_with(": it lacks search.parts.MORE"), "{refused}");
  assert!(refused.contains(&path.display().to_string()), "{refused}");
}

#[test]
fn a_file_holding_a_key_no_line_reads_is_refused_with_the_key() {
  let (_dir, path) =
    written(&ENGLISH.replace("    MORE: \"In detail\"\n", "    MORE: \"In detail\"\n    LATER: \"x\"\n"));
  let refused = Wording::read(Some(&path)).expect_err("refused").to_string();

  assert!(refused.ends_with(": no line is read under search.parts.LATER"), "{refused}");
}

#[test]
fn a_file_that_is_not_yaml_or_not_there_is_refused() {
  let (dir, path) = written("search: [");

  assert!(Wording::read(Some(&path)).is_err());
  assert!(Wording::read(Some(&dir.path().join("absent.yaml"))).is_err());
}

#[test]
fn a_shell_reads_the_wording_its_settings_name() {
  let (_dir, path) = written(&ENGLISH.replace("\"Archive\"", "\"Collection\""));
  let environment = HashMap::from([("BMO_ARTIFACT_WORDING__FILE".to_owned(), path.display().to_string())]);
  let settings = Settings::read(Path::new(ROOT), Some(environment)).expect("the settings read");

  assert_eq!(
    Shell::new(settings).expect("the shell stands").wording.search.archive_name,
    "Collection"
  );

  let environment = HashMap::from([("BMO_ARTIFACT_WORDING__FILE".to_owned(), "absent.yaml".to_owned())]);
  let settings = Settings::read(Path::new(ROOT), Some(environment)).expect("the settings read");

  assert!(Shell::new(settings).is_err());
}
