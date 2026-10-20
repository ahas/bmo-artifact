//! The shared helpers, held to the cases their TypeScript twins answer alike: the answer ceiling a reading is
//! cut at, and a line's `{name}` marks filled in.

use bmo_artifact::helpers::ceiling::{
  MAX_ANSWER_CHARS, MAX_VALUE_CHARS, Said, cap_items, cap_value, count_text,
};
use bmo_artifact::helpers::marks::fill_marks;
use serde::Deserialize;

// The ceiling's cases, which its TypeScript twin answers alike.
const CASES: &str = include_str!("cases/answer-cap-cases.json");

#[test]
fn a_mark_is_filled_once_with_what_is_named() {
  assert_eq!(fill_marks("<{tag}>", &[("tag", "b")]), "<b>");
  assert_eq!(fill_marks("{a} {b} {c}", &[("a", "{b}"), ("b", "2")]), "{b} 2 {c}");
  assert_eq!(fill_marks("{ a } {} {a", &[("a", "x")]), "{ a } {} {a");
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Piece {
  Text(String),
  Repeated { text: String, times: usize },
}

fn joined(pieces: &[Piece]) -> String {
  pieces
    .iter()
    .map(|piece| match piece {
      Piece::Text(text) => text.clone(),
      Piece::Repeated { text, times } => text.repeat(*times),
    })
    .collect()
}

#[derive(Deserialize)]
struct ItemCase {
  name: String,
  items: Vec<Piece>,
  copies: Option<usize>,
  limit: Option<usize>,
  render: String,
  unit: String,
  how: String,
  kept: usize,
  left: usize,
  note: Option<String>,
}

#[derive(Deserialize)]
struct ValueCase {
  name: String,
  text: Vec<Piece>,
  limit: Option<usize>,
  expect: Vec<Piece>,
}

#[derive(Deserialize)]
struct Cases {
  items: Vec<ItemCase>,
  values: Vec<ValueCase>,
}

fn rendering(kind: &str) -> impl Fn(&[String]) -> String {
  let json = kind == "json";

  move |kept| {
    if json {
      serde_json::to_string_pretty(kept).expect("a list writes as JSON")
    } else {
      kept.join("\n")
    }
  }
}

#[test]
fn a_list_is_cut_as_the_typescript_ceiling_cuts_it() {
  let cases: Cases = serde_json::from_str(CASES).expect("the cases read");

  for case in &cases.items {
    let pieces: Vec<String> = case.items.iter().map(|piece| joined(std::slice::from_ref(piece))).collect();
    let items: Vec<String> =
      pieces.iter().cloned().cycle().take(pieces.len() * case.copies.unwrap_or(1)).collect();
    let said = Said {
      unit: &case.unit,
      how: &case.how,
      limit: case.limit.unwrap_or(MAX_ANSWER_CHARS),
    };
    let capped = cap_items(&items, rendering(&case.render), said);

    assert_eq!(
      (capped.kept.len(), capped.left, capped.note.clone()),
      (case.kept, case.left, case.note.clone()),
      "{}",
      case.name
    );
  }
}

#[test]
fn a_value_is_cut_as_the_typescript_ceiling_cuts_it() {
  let cases: Cases = serde_json::from_str(CASES).expect("the cases read");

  for case in &cases.values {
    assert_eq!(
      cap_value(&joined(&case.text), case.limit.unwrap_or(MAX_VALUE_CHARS)),
      joined(&case.expect),
      "{}",
      case.name
    );
  }
}

#[test]
fn counts_carry_the_thousands_mark() {
  assert_eq!(count_text(0), "0");
  assert_eq!(count_text(999), "999");
  assert_eq!(count_text(1_000), "1,000");
  assert_eq!(count_text(30_000), "30,000");
  assert_eq!(count_text(1_234_567), "1,234,567");
}

#[test]
fn a_said_ceiling_is_the_standing_one() {
  assert_eq!(Said::new("lines", "read on.").limit, MAX_ANSWER_CHARS);
}
