//! A search as it is asked (`docs/archive.md`, "What the search is given"): the depths, the options, the
//! request file and the run's prompt.

use bmo_artifact::helpers::lines::Trees;
use bmo_artifact::verbs::search::request::{
  Asker, DEPTHS, Options, Request, asked_at_text, asker_text, branch_allowance, depth, depth_names,
  is_depth, options, options_text, read_request, task,
};
use chrono_tz::Asia::Seoul;

// The search agent's request document, as a deployment ships it.
const REQUEST_TEMPLATE: &str = "Web research task. Find out, by actually searching the web: {{query}}\n\nWho asked: {{asker}}\n\nAsked at: {{askedAt}}. Anything the answer judges \"now\" (whether a place is open, what is on sale today) is judged at this moment.\n\nOptions for this search: {{options}}\n\nThis run is {{depth}} and may open {{branches}}.\n";

#[test]
fn an_unknown_depth_is_the_shallowest_and_the_ladder_reads_in_order() {
  assert_eq!(depth(Some("thorough")).name, "thorough");
  assert!(depth(Some("thorough")).deep_dive);
  assert_eq!(depth(Some("very-deep")).name, "basic");
  assert_eq!(depth(None).name, "basic");
  assert_eq!(depth_names(), "basic|thorough|exhaustive");
  assert!(is_depth("exhaustive") && !is_depth("deep"));
}

#[test]
fn an_option_outside_its_words_is_refused_and_a_blank_one_left_out() {
  assert_eq!(
    options(None, Some("asia"), None, None, None),
    Err("--region must be one of domestic|global".to_owned())
  );
  assert_eq!(
    options(None, None, None, Some("essay"), None),
    Err("--shape must be one of brief|table|full|learning".to_owned())
  );
  assert_eq!(
    options(None, None, None, None, Some("ftp://x")),
    Err("--import must be a web address (http or https)".to_owned())
  );
  assert_eq!(
    options(Some(" 1y "), Some("global"), Some("  "), Some("table"), Some(" https://a.example/p "))
      .unwrap(),
    Options {
      period: Some("1y".to_owned()),
      region: Some("global".to_owned()),
      exclude: None,
      shape: Some("table".to_owned()),
      source: Some("https://a.example/p".to_owned()),
    }
  );
}

#[test]
fn a_request_file_reads_back_and_plain_text_is_the_question_alone() {
  let written = serde_json::to_string(&Request {
    query: "rate".to_owned(),
    asker: Some(Asker {
      id: 7,
      name: "tester".to_owned(),
    }),
    asked_at: Some(1_700_000_000),
    options: Options {
      shape: Some("brief".to_owned()),
      ..Options::default()
    },
  })
  .unwrap();

  assert_eq!(
    written,
    r#"{"query":"rate","asker":{"id":7,"name":"tester"},"askedAt":1700000000,"options":{"shape":"brief"}}"#
  );
  assert_eq!(read_request(&written).asker.unwrap().name, "tester");
  assert_eq!(
    read_request("  what is the rate  "),
    Request {
      query: "what is the rate".to_owned(),
      ..Request::default()
    }
  );
}

#[test]
fn the_prompt_says_when_in_the_settings_zone_and_the_options_given() {
  assert_eq!(asked_at_text(1_700_000_000, Seoul), "2023-11-15 07:13 KST, Wednesday");
  assert_eq!(asked_at_text(1_700_000_000, chrono_tz::UTC), "2023-11-14 22:13 UTC, Tuesday");
  assert_eq!(options_text(&Options::default()), "none given; decide them from the question.");
  assert_eq!(
    options_text(&Options {
      region: Some("domestic".to_owned()),
      exclude: Some("blogs".to_owned()),
      ..Options::default()
    }),
    "region: domestic material first; excluded sources: blogs. Any option not named here is decided from the question."
  );
  assert_eq!(branch_allowance(0), "no branch at all");
  assert_eq!(branch_allowance(1), "one branch");
  assert_eq!(branch_allowance(4), "up to 4 branches");
}

// The run's prompt, made from the request document under a code tree of the test's own.
fn asked(query: &str, depth: &str, max_children: i64, request: Request) -> String {
  let code = tempfile::tempdir().unwrap();
  let agent = code.path().join("prompts").join("agents").join("search");

  std::fs::create_dir_all(&agent).unwrap();
  std::fs::write(agent.join("request.md"), REQUEST_TEMPLATE).unwrap();

  let trees = Trees {
    code: code.path().to_path_buf(),
  };

  task(
    &trees,
    &Request {
      query: query.to_owned(),
      ..request
    },
    depth,
    max_children,
    1_789_659_000,
    Seoul,
  )
  .unwrap()
}

#[test]
fn a_missing_or_stale_depth_is_the_narrowest_never_the_widest() {
  assert_eq!(depth(None).name, "basic");
  assert_eq!(depth(Some("quick")).name, "basic");
  assert_eq!(depth(Some("exhaustive")).name, "exhaustive");
}

#[test]
fn only_the_two_deeper_rungs_are_deep_dives() {
  assert_eq!(
    DEPTHS.iter().filter(|rung| rung.deep_dive).map(|rung| rung.name).collect::<Vec<_>>(),
    ["thorough", "exhaustive"]
  );
}

#[test]
fn names_the_asker_and_the_options_in_the_task() {
  let task = asked(
    "delivery near the station",
    "basic",
    0,
    Request {
      asker: Some(Asker {
        id: 200_000_002,
        name: "Bob".to_owned(),
      }),
      options: Options {
        region: Some("domestic".to_owned()),
        shape: Some("table".to_owned()),
        period: Some("the last month".to_owned()),
        ..Options::default()
      },
      ..Request::default()
    },
  );

  assert!(task.contains("Who asked: Bob (Telegram id 200000002)"));
  assert!(task.contains("period: the last month; region: domestic material first; result shape: table"));
  assert!(!task.contains("{{"));
}

#[test]
fn names_no_duration_at_all_and_names_the_branch_allowance_it_does_have() {
  let task = asked("a face rigging tool", "exhaustive", 8, Request::default());

  assert!(task.contains("Find out, by actually searching the web: a face rigging tool"));
  assert!(task.contains("This run is exhaustive and may open up to 8 branches."));
  assert!(!task.contains("minutes in all"));
}

#[test]
fn with_nobody_named_the_rulebooks_default_applies_and_the_options_are_the_runs_judgement() {
  let task = asked("silver price", "basic", 0, Request::default());

  assert!(task.contains("Who asked: not known from the launch"));
  assert!(task.contains("Options for this search: none given; decide them from the question."));
}

#[test]
fn the_request_file_is_read_as_json_and_plain_text_is_the_question_alone() {
  let request = r#"{"query":" silver ","asker":{"id":1,"name":"tester"},"options":{"shape":"brief"}}"#;

  assert_eq!(
    read_request(request),
    Request {
      query: "silver".to_owned(),
      asker: Some(Asker {
        id: 1,
        name: "tester".to_owned(),
      }),
      asked_at: None,
      options: Options {
        shape: Some("brief".to_owned()),
        ..Options::default()
      },
    }
  );
  assert_eq!(read_request("  silver\n").query, "silver");
  assert_eq!(read_request(r#"{"query":"x","asker":{"id":"a"}}"#).asker, None);
  assert_eq!(read_request(r#"{"query":"x","askedAt":1789659000}"#).asked_at, Some(1_789_659_000));
  assert_eq!(read_request(r#"{"query":"x","askedAt":"soon"}"#).asked_at, None);
}

#[test]
fn the_run_is_told_when_the_question_was_asked_on_the_settings_clock_with_its_weekday() {
  // 2026-09-17T14:40:07Z is a Thursday at 23:40 in the zone the test names.
  assert_eq!(asked_at_text(1_789_656_007, Seoul), "2026-09-17 23:40 KST, Thursday");

  let task = asked(
    "delivery near the station",
    "basic",
    0,
    Request {
      asked_at: Some(1_789_656_007),
      ..Request::default()
    },
  );

  assert!(task.contains("Asked at: 2026-09-17 23:40 KST, Thursday."));
}

#[test]
fn the_asker_is_told_not_to_bring_a_rulebook_into_play_by_itself() {
  let named = Asker {
    id: 5,
    name: "tester".to_owned(),
  };

  assert!(asker_text(Some(&named)).contains("Who asked never brings a rulebook into play by itself"));
  assert!(asker_text(None).contains("Where a rulebook below applies to this search"));
}

#[test]
fn options_outside_the_known_words_are_refused_and_blank_ones_are_left_out() {
  assert_eq!(
    options(Some(" "), Some("domestic"), None, Some("brief"), None).unwrap(),
    Options {
      region: Some("domestic".to_owned()),
      shape: Some("brief".to_owned()),
      ..Options::default()
    }
  );
  assert!(options(None, Some("moon"), None, None, None).unwrap_err().contains("--region"));
  assert!(options(None, None, None, Some("long"), None).unwrap_err().contains("--shape"));
  assert_eq!(
    options(None, None, None, Some("learning"), None).unwrap().shape.as_deref(),
    Some("learning")
  );
}

#[test]
fn a_page_to_import_is_a_web_address_and_nothing_else_is_taken_for_one() {
  assert_eq!(
    options(None, None, None, None, Some(" https://r.example/a "))
      .unwrap()
      .source
      .as_deref(),
    Some("https://r.example/a")
  );
  assert!(
    options(None, None, None, None, Some("file:///etc/passwd"))
      .unwrap_err()
      .contains("--import")
  );
}
