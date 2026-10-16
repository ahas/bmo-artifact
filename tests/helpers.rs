//! The copied helpers and the Rust twins, held to the cases they were held to where they came from
//! (`docs/inventory.md`, sections 4 and 5).

use std::path::PathBuf;

use axum::http::StatusCode;
use bmo_artifact::helpers::ceiling::{
  MAX_ANSWER_CHARS, MAX_VALUE_CHARS, Said, cap_items, cap_value, count_text,
};
use bmo_artifact::helpers::js::{chars, is_space, trim};
use bmo_artifact::helpers::lines::{Trees, read_required, render};
use bmo_artifact::helpers::marks::fill_marks;
use bmo_artifact::helpers::run_log::json::Json;
use bmo_artifact::helpers::run_log::lines::{BRANCH_TOOLS, Line, record_lines, stream_lines};
use bmo_artifact::helpers::toolbook::reached;
use bmo_artifact::helpers::web_address::{address_as_written, markdown_link_at, web_addresses_in};
use serde::Deserialize;

// The ceiling's cases, which its TypeScript twin answers alike.
const CASES: &str = include_str!("cases/answer-cap-cases.json");

#[test]
fn white_space_is_javascript_s() {
  assert!(is_space('\u{feff}'));
  assert!(!is_space('\u{85}'));
  assert_eq!(trim("\u{feff} a \u{3000}"), "a");
}

#[test]
fn an_address_ends_where_the_sentence_takes_over() {
  assert_eq!(address_as_written("https://example.com/a)."), "https://example.com/a");
  assert_eq!(
    address_as_written("https://en.wikipedia.org/wiki/A_(b))"),
    "https://en.wikipedia.org/wiki/A_(b)"
  );
  assert_eq!(address_as_written("https://example.com/\u{c8fc}\u{c18c}"), "https://example.com/");
}

#[test]
fn a_link_and_a_bare_address_are_found_in_order() {
  let found = web_addresses_in("see [docs](https://a.example/x) and https://b.example/y.");

  assert_eq!(
    found.iter().map(|one| one.url.as_str()).collect::<Vec<_>>(),
    ["https://a.example/x", "https://b.example/y"]
  );
  assert_eq!(found[0].start, 11);
}

#[test]
fn a_link_needs_its_parentheses_and_no_space() {
  assert!(markdown_link_at(&chars("[a](b c)"), 0).is_none());
  assert!(markdown_link_at(&chars("[a] (b)"), 0).is_none());
  assert_eq!(markdown_link_at(&chars("[a [b]](c(d))"), 0).map(|link| link.url), Some("c(d)".into()));
}

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

#[test]
fn a_mark_is_filled_where_it_is_named_and_left_where_it_is_not() {
  assert_eq!(render("a {{tools}} b {{other}}", &[("tools", "x")]), "a x b {{other}}");
}

#[test]
fn a_required_document_is_read_trimmed_and_its_absence_is_a_fault() {
  let dir = tempfile::tempdir().expect("a folder");
  let path = dir.path().join("agent.md");

  std::fs::write(&path, "\n  the document \n\n").expect("written");

  assert_eq!(read_required(&path).expect("read"), "the document");

  let refusal = read_required(&dir.path().join("absent.md")).expect_err("refused");

  assert_eq!(refusal.status, StatusCode::INTERNAL_SERVER_ERROR);
  assert!(refusal.detail.contains("absent.md: "), "{}", refusal.detail);
}

#[test]
fn the_trees_name_an_agent_s_documents_and_a_rulebook() {
  let trees = Trees {
    code: PathBuf::from("/code"),
  };

  assert_eq!(trees.agent_dir("scout"), PathBuf::from("/code/prompts/agents/scout"));
  assert_eq!(trees.rulebook_path("research"), PathBuf::from("/code/prompts/rules/research.md"));
}

#[test]
fn an_object_is_written_back_in_the_order_it_came() {
  let text = r#"{"z":1,"a":{"y":[true,null,"é\n"],"b":2.5},"m":-3}"#;

  assert_eq!(Json::parse(text).expect("JSON").stringify(), text);
}

#[test]
fn a_name_written_twice_keeps_its_place_and_its_last_value() {
  assert_eq!(Json::parse(r#"{"a":1,"b":2,"a":3}"#).expect("JSON").stringify(), r#"{"a":3,"b":2}"#);
}

#[test]
fn a_proxy_call_reads_as_the_tool_it_reached_in_its_road_s_spelling() {
  assert_eq!(reached("mcp__toolbook__call", "web/web_fetch").as_deref(), Some("mcp__web__web_fetch"));
  assert_eq!(reached("toolbook/act", "web/web_fetch").as_deref(), Some("web/web_fetch"));
  assert_eq!(reached("mcp__web__web_fetch", "web/web_fetch"), None);
}

fn event(text: &str) -> Json {
  Json::parse(text).expect("an event")
}

#[test]
fn an_assistant_line_leaves_its_words_its_thinking_and_its_calls() {
  let lines = record_lines(&event(
    r#"{"type":"assistant","message":{"id":"m1","content":[{"type":"thinking","thinking":""},{"type":"text","text":"hi"},{"type":"tool_use","id":"t1","name":"Agent","input":{"subagent_type":"scout","prompt":"look"}}]}}"#,
  ));

  assert_eq!(lines.len(), 2);
  assert_eq!((lines[0].kind, lines[0].text.as_str(), lines[0].source_block), ("text", "hi", Some(1)));
  assert_eq!(lines[1].name.as_deref(), Some("Agent(scout)"));
  assert_eq!(lines[1].text, r#"{"subagent_type":"scout","prompt":"look"}"#);
  assert_eq!((lines[1].state, lines[1].source_id.as_deref()), (Some("requested"), Some("m1")));
}

#[test]
fn a_compaction_is_a_line_of_its_own_with_how_it_was_set_off_and_how_full_the_session_was() {
  let lines = stream_lines(&event(
    r#"{"type":"system","subtype":"compact_boundary","compact_metadata":{"trigger":"auto","pre_tokens":167000}}"#,
  ));

  assert_eq!(lines.len(), 1);
  assert_eq!((lines[0].kind, lines[0].text.as_str()), ("compact", "auto · 167000 tokens before"));
}

#[test]
fn a_toolbook_call_is_logged_as_the_tool_it_reached() {
  let lines = record_lines(&event(
    r#"{"type":"assistant","uuid":"u","message":{"content":[{"type":"tool_use","id":"t","name":"mcp__toolbook__call","input":{"tool":"web/fetch","arguments":{"url":"x"}}}]}}"#,
  ));

  assert_eq!(lines[0].name.as_deref(), Some("mcp__web__fetch"));
  assert_eq!(lines[0].text, r#"{"url":"x"}"#);
  assert!(lines[0].metadata.is_some());
}

#[test]
fn a_result_line_carries_its_answer_its_state_and_its_evidence() {
  let lines = record_lines(&event(
    r#"{"type":"user","parent_tool_use_id":"p","toolUseResult":{"filePath":"/a","type":"update"},"message":{"content":[{"type":"tool_result","tool_use_id":"t","content":[{"type":"text","text":"ok"},{"type":"image"}]}]}}"#,
  ));

  assert_eq!(lines[0].text, "ok\n{\"type\":\"image\"}");
  assert_eq!((lines[0].state, lines[0].parent.as_deref()), (Some("succeeded"), Some("p")));
  assert_eq!(
    lines[0].metadata.as_ref().map(Json::stringify).as_deref(),
    Some(r#"{"toolUseResult":{"filePath":"/a","type":"update"}}"#)
  );
}

#[test]
fn a_run_s_start_and_end_are_lines_of_their_own() {
  let start =
    stream_lines(&event(r#"{"type":"system","subtype":"init","model":"m","tools":["Read","Agent"]}"#));

  assert_eq!((start[0].kind, start[0].text.as_str()), ("start", "m · tools: Read, Agent"));

  let end = stream_lines(&event(r#"{"type":"result","is_error":false,"result":"　answered\n"}"#));

  assert_eq!(
    end,
    vec![Line {
      kind: "end",
      text: "done\nanswered".into(),
      state: Some("succeeded"),
      ..Line::default()
    }]
  );
  assert!(BRANCH_TOOLS.contains(&"Agent"));
}
