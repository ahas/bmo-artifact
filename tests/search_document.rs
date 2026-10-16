//! The record a search stands as and the document a person reads (`docs/archive.md`, "`searches`"): a
//! result read as findings, a run's stream read for where it went, and the document written from both.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use bmo_artifact::helpers::marks::fill_marks;
use bmo_artifact::verbs::search::document::{
  AskedQuery, CitedSource, HeldFinding, HeldSource, OpenedPage, Record, ResultHead, Trace,
  attribute_branches, leaves_no_document, read_stream, render, result_findings, result_head,
};
use bmo_artifact::wording::Wording;

static WORDING: LazyLock<Wording> = LazyLock::new(|| Wording::english().unwrap());

fn event(body: serde_json::Value) -> String {
  body.to_string()
}

fn tool_use(id: &str, name: &str, input: serde_json::Value, parent: Option<&str>) -> String {
  event(serde_json::json!({
    "type": "assistant",
    "parent_tool_use_id": parent,
    "message": { "content": [{ "type": "tool_use", "id": id, "name": name, "input": input }] },
  }))
}

// One thorough run as its stream reads: two co-workers fanned out, each searching and opening pages of its
// own, and one page the run opened itself after they came back.
fn fanned_out() -> String {
  [
    event(serde_json::json!({ "type": "system", "subtype": "init", "model": "claude-opus-5", "tools": ["Task", "mcp__web__web_search"] })),
    tool_use("toolu_1", "Task", serde_json::json!({ "subagent_type": "scout", "prompt": "list price" }), None),
    tool_use("toolu_2", "Task", serde_json::json!({ "subagent_type": "scout", "prompt": "used price" }), None),
    tool_use("toolu_a", "mcp__web__web_search", serde_json::json!({ "query": "m4 mac mini education" }), Some("toolu_1")),
    tool_use("toolu_b", "mcp__web__web_fetch", serde_json::json!({ "url": "https://apple.com/kr-edu" }), Some("toolu_1")),
    tool_use("toolu_c", "mcp__web__web_fetch", serde_json::json!({ "url": "https://apple.com/kr-edu/" }), Some("toolu_1")),
    tool_use("toolu_d", "mcp__web__web_fetch", serde_json::json!({ "url": "https://joonggo.example/m4" }), Some("toolu_2")),
    tool_use("toolu_e", "mcp__web__web_fetch", serde_json::json!({ "url": "https://apple.com/kr/shop/mac" }), None),
    "not json at all".to_owned(),
    event(serde_json::json!({ "type": "result", "is_error": false, "result": "done" })),
  ]
  .join("\n")
}

const TWO_ITEMS: &str = "NAME: education discount\nThe education price is 8% below list.\nSOURCE: https://apple.com/kr-edu :: discount\nSOURCE: https://apple.com/kr/shop/mac :: list price\n===ITEM===\nNAME: used\nUsed ones go for about 70% of launch price.\nSOURCE: https://joonggo.example/m4\n===ITEM===\nTITLE: what to buy\nSUMMARY: buying new with the discount is better.";

fn source(url: &str, supports: Option<&str>, mark: Option<&str>) -> CitedSource {
  CitedSource {
    url: url.to_owned(),
    supports: supports.map(str::to_owned),
    mark: mark.map(str::to_owned),
  }
}

// a result read as findings

#[test]
fn is_one_finding_per_entry_each_with_the_pages_cited_under_it_and_without_the_source_lines() {
  let findings = result_findings(TWO_ITEMS);

  assert_eq!(findings.len(), 3);
  assert_eq!(findings[0].text, "NAME: education discount\nThe education price is 8% below list.");
  assert_eq!(
    findings[0].sources,
    vec![
      source("https://apple.com/kr-edu", Some("discount"), None),
      source("https://apple.com/kr/shop/mac", Some("list price"), None)
    ]
  );
  assert_eq!(findings[1].sources, vec![source("https://joonggo.example/m4", None, None)]);
  assert!(findings[2].text.contains("SUMMARY: buying new"));
}

#[test]
fn carries_on_where_a_result_cites_nothing_and_gives_that_finding_an_empty_source_list() {
  let findings = result_findings("TITLE: weather today\nCloudy with rain.\nSUMMARY: take an umbrella.");

  assert_eq!(findings.len(), 1);
  assert!(findings[0].sources.is_empty());
  assert!(findings[0].text.contains("Cloudy with rain."));
}

#[test]
fn keeps_the_answered_verdict_out_of_every_finding_since_it_is_the_runs_word_to_the_runner() {
  let findings = result_findings(
    "ANSWERED: yes :: one tool\nNAME: Faceit\nAutomates face rigging.\nSOURCE: https://faceit.example :: features\n===ITEM===\nTITLE: rigging tools\nSUMMARY: Faceit.",
  );

  assert_eq!(findings.len(), 2);
  assert!(!findings.iter().any(|finding| finding.text.contains("ANSWERED")));
  assert!(findings[0].text.starts_with("NAME: Faceit"));
  assert_eq!(findings[0].sources, vec![source("https://faceit.example", Some("features"), None)]);
}

#[test]
fn leaves_out_narration_written_above_the_verdict_line() {
  let findings = result_findings(
    "Putting the result together.\n\nANSWERED: yes :: price\nTITLE: silver\nSUMMARY:\n1. value",
  );

  assert_eq!(findings.len(), 1);
  assert_eq!(findings[0].text, "SUMMARY:\n1. value");
}

#[test]
fn says_nothing_of_an_empty_result_rather_than_inventing_a_finding_for_it() {
  assert!(result_findings("").is_empty());
  assert!(result_findings("\n\n===ITEM===\n   \n").is_empty());
}

#[test]
fn keeps_the_document_line_out_of_the_findings_too_the_way_the_rest_of_the_head_is_kept_out() {
  let findings =
    result_findings("ANSWERED: yes :: value\nTITLE: silver\nDOCUMENT: no\nSUMMARY:\n1. 12,410 won");

  assert_eq!(findings[0].text, "SUMMARY:\n1. 12,410 won");
}

// whether a search leaves a document

const BRIEF: &str = "ANSWERED: yes :: one gram of silver\nTITLE: silver 12,410 won\nDOCUMENT: no\nSUMMARY:\n1. 12,410 won\nUNCONFIRMED:\n1. none";

#[test]
fn a_one_value_result_that_says_it_leaves_none_leaves_none() {
  assert!(leaves_no_document(BRIEF));
  assert!(leaves_no_document(&BRIEF.replace("DOCUMENT: no", "DOCUMENT: none")));
}

#[test]
fn a_result_that_says_nothing_about_it_leaves_one() {
  assert!(!leaves_no_document(&BRIEF.replace("DOCUMENT: no\n", "")));
  assert!(!leaves_no_document(&BRIEF.replace("DOCUMENT: no", "DOCUMENT: yes")));
}

#[test]
fn a_result_carrying_a_body_of_any_kind_keeps_its_document_whatever_its_head_says() {
  for body in [
    "BODY:\nThe two prices differ.",
    "SECTION: the two prices\nThey differ.",
    "===ITEM===\nNAME: an exchange",
    "IMAGE: https://example.com/a.jpg :: chart",
    "REVIEW: 4.5 :: good :: https://example.com/r",
  ] {
    assert!(!leaves_no_document(&format!("{BRIEF}\n{body}")), "{body}");
  }
}

// a run's stream read for where it went

#[test]
fn collects_the_pages_opened_and_the_questions_asked_under_the_branch_that_made_the_call() {
  let trace = read_stream(&fanned_out(), None);
  let first = Some("Task(scout) 1".to_owned());
  let second = Some("Task(scout) 2".to_owned());

  assert_eq!(trace.branches, vec![first.clone(), second.clone(), None]);
  assert_eq!(
    trace.queries,
    vec![AskedQuery {
      query: "m4 mac mini education".to_owned(),
      branch: first.clone()
    }]
  );
  assert_eq!(
    trace.pages.iter().map(|page| page.url.as_str()).collect::<Vec<_>>(),
    [
      "https://apple.com/kr-edu",
      "https://joonggo.example/m4",
      "https://apple.com/kr/shop/mac"
    ]
  );
  assert_eq!(
    trace.pages.iter().map(|page| page.branch.clone()).collect::<Vec<_>>(),
    vec![first, second, None]
  );
}

#[test]
fn keeps_a_branch_whose_opening_call_it_never_saw_under_the_id_the_lines_carry() {
  let orphan = tool_use(
    "toolu_z",
    "mcp__web__web_fetch",
    serde_json::json!({ "url": "https://example.com/a" }),
    Some("toolu_gone"),
  );

  assert_eq!(
    read_stream(&orphan, None).pages,
    vec![OpenedPage {
      url: "https://example.com/a".to_owned(),
      branch: Some("toolu_gone".to_owned())
    }]
  );
}

#[test]
fn reads_the_web_by_the_mcp_prefix_and_leaves_every_other_tool_alone() {
  let mixed = [
    tool_use("toolu_1", "Read", serde_json::json!({ "file_path": "/etc/hosts" }), None),
    tool_use(
      "toolu_2",
      "mcp__web__web_open",
      serde_json::json!({ "url": "https://example.com/b" }),
      None,
    ),
    tool_use(
      "toolu_3",
      "mcp__chrome-devtools__navigate_page",
      serde_json::json!({ "url": "https://example.com/c" }),
      None,
    ),
  ]
  .join("\n");

  assert_eq!(
    read_stream(&mixed, None).pages,
    vec![OpenedPage {
      url: "https://example.com/b".to_owned(),
      branch: None
    }]
  );
}

// which branch a finding came out of

#[test]
fn is_the_one_branch_that_opened_the_pages_it_cites_and_nobody_where_two_of_them_did() {
  let attributed = attribute_branches(result_findings(TWO_ITEMS), &read_stream(&fanned_out(), None));

  assert_eq!(attributed[0].branch, None);
  assert_eq!(attributed[1].branch.as_deref(), Some("Task(scout) 2"));
  assert_eq!(attributed[2].branch, None);
}

// the document the search leaves behind

fn held(run_id: i64, branch: Option<&str>, text: &str, sources: Vec<HeldSource>) -> HeldFinding {
  HeldFinding {
    run_id: Some(run_id),
    branch: branch.map(str::to_owned),
    text: text.to_owned(),
    sources,
  }
}

fn record() -> Record {
  Record {
    query: "cheapest M4 mac mini".to_owned(),
    started_depth: "basic".to_owned(),
    ended_depth: Some("thorough".to_owned()),
    status: "done".to_owned(),
    started_at: 1_757_000_000,
    finished_at: Some(1_757_000_240),
    runs: vec![11, 12],
    findings: vec![
      held(
        11,
        Some("Task(scout) 2"),
        "NAME: used\nUsed ones go for about 70% of launch price.",
        vec![HeldSource {
          url: "https://joonggo.example/m4".to_owned(),
          supports: Some("price".to_owned()),
          mark: None,
        }],
      ),
      held(12, None, "SUMMARY: buying new with the discount is better.", vec![]),
    ],
    ..Record::default()
  }
}

fn none_kept() -> BTreeMap<String, String> {
  BTreeMap::new()
}

fn rendered(search: &Record, trace: &Trace, kept: &BTreeMap<String, String>) -> String {
  render(search, trace, kept, &WORDING, chrono_tz::UTC)
}

#[test]
fn folds_the_branches_trail_under_the_result_they_were_merged_into() {
  let search = &WORDING.search;
  let rendered = rendered(&record(), &read_stream(&fanned_out(), None), &none_kept());
  let result = rendered.find(&format!("## {}", search.result.heading)).unwrap();

  assert!(result < rendered.find(&format!("## {}", search.trail.heading)).unwrap());
  assert!(rendered.contains("<details>"));
  assert!(rendered.contains("### Task(scout) 1"));
  assert!(rendered.contains("### Task(scout) 2"));
  assert!(rendered.contains(&search.trail.found));
  assert!(rendered.contains("- https://joonggo.example/m4"));
  assert!(rendered.contains("- m4 mac mini education"));
  assert!(rendered.contains(&format!("### {}", search.trail.own)));
}

#[test]
fn gives_each_rungs_branch_a_section_of_its_own_named_for_the_run_that_opened_it() {
  let search = &WORDING.search;
  let older = Record {
    findings: vec![
      held(11, Some("Task(scout) 1"), "NAME: what the run before saw", vec![]),
      held(12, Some("Task(scout) 1"), "NAME: what this run saw", vec![]),
    ],
    ..record()
  };
  let rendered = rendered(&older, &read_stream(&fanned_out(), Some(12)), &none_kept());
  let branch_part = &rendered[rendered.find(&format!("## {}", search.trail.heading)).unwrap()..];
  let mark = |run: i64| fill_marks(&search.trail.run, &[("run", &run.to_string())]);

  assert!(branch_part.contains(&format!("### Task(scout) 1{}", mark(11))));
  assert!(branch_part.contains(&format!("### Task(scout) 1{}", mark(12))));

  let (eleven, twelve) = (branch_part.find(&mark(11)).unwrap(), branch_part.find(&mark(12)).unwrap());
  let below = &branch_part[eleven..twelve];

  assert!(below.contains("- what the run before saw"));
  assert!(!below.contains("- what this run saw"));
  assert!(branch_part[twelve..].contains("- what this run saw"));
  assert!(!below.contains(&search.trail.opened));
}

#[test]
fn heads_the_document_with_what_the_search_was_and_how_far_it_went() {
  let rendered = rendered(&record(), &read_stream(&fanned_out(), None), &none_kept());
  let head = &WORDING.search.head;

  assert!(
    rendered.contains(&format!("# {}", fill_marks(&head.untitled, &[("query", "cheapest M4 mac mini")])))
  );
  assert!(rendered.contains(&fill_marks(&head.depth, &[("value", "basic → thorough")])));
  assert!(rendered.contains(&fill_marks(&head.status, &[("value", "done")])));
  assert!(rendered.contains(&fill_marks(&head.runs, &[("value", "11, 12")])));
}

#[test]
fn reads_every_finding_in_the_result_section_with_the_pages_under_it() {
  let search = &WORDING.search;
  let rendered = rendered(&record(), &Trace::default(), &none_kept());
  let result = &rendered[rendered.find(&format!("## {}", search.result.heading)).unwrap()..];
  let branch = fill_marks(&search.result.branch, &[("branch", "Task(scout) 2")]);

  assert!(result.contains("### used"));
  assert!(result.contains(&format!("{branch}{}", fill_marks(&search.trail.run, &[("run", "11")]))));
  assert!(result.contains("- https://joonggo.example/m4 :: price"));
  assert!(result.contains("SUMMARY: buying new with the discount is better."));
}

#[test]
fn and_a_search_that_never_split_into_branches_has_no_branch_section_at_all() {
  let search = &WORDING.search;
  let plain = Record {
    runs: vec![11],
    findings: record()
      .findings
      .into_iter()
      .map(|finding| HeldFinding {
        branch: None,
        run_id: Some(11),
        ..finding
      })
      .collect(),
    ..record()
  };
  let rendered = rendered(
    &plain,
    &Trace {
      run_id: Some(11),
      ..Trace::default()
    },
    &none_kept(),
  );

  assert!(!rendered.contains(&format!("## {}", search.trail.heading)));
  assert!(!rendered.contains(&fill_marks(&search.trail.run, &[("run", "11")])));
}

// a result in the document shape

const SHAPED: &str = "ANSWERED: yes :: silver price\nTITLE: silver at 63.74 dollars an ounce, up 1.4%\nTOPIC: today's silver price\nKEYWORDS: silver price, spot ,exchange\nRULE: house rule\nSUMMARY:\n1. Spot silver is 63.74 dollars an ounce[1].\n2. The exchange sells a gram at 11,890 won[2†].\nUNCONFIRMED:\n1. today's won rate\nSOURCE [1]: https://www.kitco.com/charts/silver :: spot\nSOURCE [2†]: https://siseboda.com/silver :: local price\n===ITEM===\nNAME: Noodle House\nADDRESS: 1 Main St\nMAP: https://map.naver.com/p/search/noodle\nROADVIEW: https://map.kakao.com/?roadview=1\nA short note[3].\nMORE:\nA long note.\nIMAGE: https://img.example/a.jpg :: the front\nREVIEW: 4.5 :: Rich broth, kind owner. :: https://blog.example/1\nREVIEW: NO_RATING :: The noodles came soggy.\nSOURCE [3]: https://place.example/1 :: reviews";

fn shaped() -> String {
  SHAPED.replace("NO_RATING", WORDING.search.field("no_rating"))
}

#[test]
fn lifts_the_head_out_of_the_text_and_keeps_it_apart() {
  assert_eq!(
    result_head(&shaped()),
    ResultHead {
      title: Some("silver at 63.74 dollars an ounce, up 1.4%".to_owned()),
      topic: Some("today's silver price".to_owned()),
      keywords: vec!["silver price".to_owned(), "spot".to_owned(), "exchange".to_owned()],
      rule: Some("house rule".to_owned()),
      kind: None,
    }
  );
  assert_eq!(result_head("TITLE:\nbody"), ResultHead::default());

  let findings = result_findings(&shaped());
  let text = findings.iter().map(|finding| finding.text.as_str()).collect::<Vec<_>>().join("\n");

  for head in ["TITLE:", "TOPIC:", "KEYWORDS:", "RULE:"] {
    assert!(!text.contains(head), "{head}");
  }

  assert!(findings[0].text.starts_with("SUMMARY:"));
}

#[test]
fn reads_the_footnote_each_source_is_cited_by_a_snippet_only_one_with_its_dagger() {
  let findings = result_findings(&shaped());

  assert_eq!(
    findings[0].sources,
    vec![
      source("https://www.kitco.com/charts/silver", Some("spot"), Some("1")),
      source("https://siseboda.com/silver", Some("local price"), Some("2†"))
    ]
  );
  assert_eq!(findings[1].sources, vec![source("https://place.example/1", Some("reviews"), Some("3"))]);
}

fn document(result: &str, kept: &BTreeMap<String, String>) -> String {
  let findings = result_findings(result);

  rendered(
    &Record {
      query: "silver price".to_owned(),
      started_depth: "basic".to_owned(),
      status: "done".to_owned(),
      started_at: 1_757_000_000,
      options: vec![("shape".to_owned(), "brief".to_owned())],
      head: result_head(result),
      runs: vec![1],
      findings: findings
        .into_iter()
        .map(|finding| HeldFinding {
          run_id: Some(1),
          branch: None,
          text: finding.text,
          sources: finding
            .sources
            .into_iter()
            .map(|source| HeldSource {
              url: source.url,
              supports: source.supports,
              mark: source.mark,
            })
            .collect(),
        })
        .collect(),
      ..Record::default()
    },
    &Trace {
      run_id: Some(1),
      ..Trace::default()
    },
    kept,
  )
}

fn part(label: &str) -> String {
  format!("#### {}", WORDING.search.parts[label])
}

fn has_structure_line(rendered: &str, labels: &[&str]) -> bool {
  rendered
    .lines()
    .any(|line| labels.iter().any(|label| line.starts_with(&format!("{label}:"))))
}

#[test]
fn writes_the_document_with_the_head_on_top_and_every_structure_line_as_what_it_stands_for() {
  let search = &WORDING.search;
  let rendered = document(&shaped(), &none_kept());
  let head = &search.head;
  let said = |name: &str, value: &str| fill_marks(search.field(name), &[("value", value)]);

  assert!(rendered.starts_with("# silver at 63.74 dollars an ounce, up 1.4%\n"));
  assert!(rendered.contains(&fill_marks(&head.query, &[("value", "silver price")])));
  assert!(rendered.contains(&fill_marks(&head.topic, &[("value", "today's silver price")])));
  assert!(rendered.contains(&fill_marks(&head.keywords, &[("value", "silver price, spot, exchange")])));
  assert!(rendered.contains(&fill_marks(&head.rule, &[("value", "house rule")])));
  assert!(rendered.contains(&fill_marks(&head.option, &[("name", "shape"), ("value", "brief")])));
  assert!(rendered.contains(&part("SUMMARY")));
  assert!(rendered.contains(&part("UNCONFIRMED")));
  assert!(rendered.contains(&part("MORE")));
  assert!(rendered.contains("### Noodle House"));
  assert!(rendered.contains(&said("ADDRESS", "1 Main St")));
  assert!(rendered.contains(&said("ROADVIEW", "https://map.kakao.com/?roadview=1")));
  assert!(rendered.contains("![the front](https://img.example/a.jpg)"));
  assert!(rendered.contains("- [2†] https://siseboda.com/silver :: local price"));
  assert!(!has_structure_line(
    &rendered,
    &[
      "SUMMARY",
      "UNCONFIRMED",
      "NAME",
      "ADDRESS",
      "MAP",
      "ROADVIEW",
      "MORE",
      "IMAGE",
      "REVIEW"
    ]
  ));
}

#[test]
fn heads_each_section_with_its_own_name_and_invents_no_part_the_result_left_out() {
  let survey = "ANSWERED: yes :: refactoring techniques\nTITLE: refactoring falls into six groups\nVERDICT:\n1. Learn the moves first.\nSUMMARY:\n1. There are six groups.\nSECTION: each group's techniques\n| group | technique |\n| --- | --- |\n| moves | move function |\nUNCONFIRMED:\n1. none";
  let rendered = document(survey, &none_kept());
  let verdict = rendered.find(&part("VERDICT")).unwrap();

  assert!(verdict < rendered.find(&part("SUMMARY")).unwrap());
  assert!(!has_structure_line(&rendered, &["VERDICT", "SECTION"]));
  assert!(rendered.contains("#### each group's techniques"));
  assert!(rendered.contains(&part("UNCONFIRMED")));

  for absent in ["INTRO", "BODY", "CONCLUSION"] {
    assert!(!rendered.contains(&part(absent)), "{absent}");
  }

  assert!(rendered.contains("| moves | move function |"));
}

// A result written with an introduction, a body and a conclusion keeps them as the headings they are.
#[test]
fn draws_a_results_intro_body_and_conclusion_as_their_headings() {
  let parted =
    "TITLE: silver\nSUMMARY:\n1. up.\nINTRO:\nasked.\nBODY:\nup.\nCONCLUSION:\nbuy.\nUNCONFIRMED:\n1. none";
  let rendered = document(parted, &none_kept());

  for heading in ["INTRO", "BODY", "CONCLUSION"] {
    assert!(rendered.contains(&part(heading)), "{heading}");
  }

  assert!(!has_structure_line(&rendered, &["INTRO", "BODY", "CONCLUSION"]));
}

#[test]
fn writes_a_review_as_the_quotation_it_is() {
  let rendered = document(&shaped(), &none_kept());
  let link = fill_marks(WORDING.search.field("review_link"), &[("page", "https://blog.example/1")]);

  assert!(rendered.contains(&format!("> **4.5** Rich broth, kind owner.{link}")));
  assert!(rendered.contains("> The noodles came soggy."));
}

#[test]
fn writes_a_products_release_successor_selling_state_and_discount_as_what_they_stand_for() {
  let search = &WORDING.search;
  let product = "ANSWERED: yes :: food dryer\nTITLE: one dryer can be had now\nSUMMARY:\n1. Buy it.\n===ITEM===\nNAME: Dryer S320\nRELEASED: November 2025\nNEWER: none\nSTOCK: 3 left :: 2026-09-21 14:20 :: https://shop.example/p/1\nDEAL: 129,000 won :: 149,000 won :: lowest in three months\n===ITEM===\nUNCONFIRMED:\n1. none";
  let rendered = document(product, &none_kept());
  let said = |name: &str, value: &str| fill_marks(search.field(name), &[("value", value)]);
  let deal = fill_marks(
    search.field("deal"),
    &[
      ("now", "129,000 won"),
      ("usual", &fill_marks(search.field("deal_usual"), &[("usual", "149,000 won")])),
      ("real", &fill_marks(search.field("deal_real"), &[("real", "lowest in three months")])),
    ],
  );

  assert!(rendered.contains("### Dryer S320"));
  assert!(rendered.contains(&said("RELEASED", "November 2025")));
  assert!(rendered.contains(&said("NEWER", "none")));
  assert!(rendered.contains(&said("STOCK", "3 left · 2026-09-21 14:20 · https://shop.example/p/1")));
  assert!(rendered.contains(&deal));
  assert!(!has_structure_line(&rendered, &["RELEASED", "NEWER", "STOCK", "DEAL"]));

  for absent in ["INTRO", "BODY", "CONCLUSION"] {
    assert!(!rendered.contains(&part(absent)), "{absent}");
  }
}

#[test]
fn shows_the_copy_it_kept_of_a_picture_with_the_address_it_came_from_under_it() {
  let kept = BTreeMap::from([(
    "https://img.example/a.jpg".to_owned(),
    "/data/searches/35/images/ab.avif".to_owned(),
  )]);
  let rendered = document(&shaped(), &kept);
  let original = fill_marks(WORDING.search.field("original"), &[("address", "https://img.example/a.jpg")]);

  assert!(rendered.contains(&format!("![the front](/data/searches/35/images/ab.avif)\n{original}")));
}

// a result written as study material

const STUDY: &str = "ANSWERED: yes :: kubernetes primer\nTITLE: learning kubernetes\nTOPIC: kubernetes basics\nKEYWORDS: kubernetes, pod\nKIND: learning\n# Before you start\nWho it is for.\n# Part 1 Getting started\n## Chapter 1 At a glance\n### Goals\n1. Say what a pod is[1].\n```yaml\n# a comment is no heading\n```\n# What could not be confirmed\n1. the newest defaults\nSOURCE [1]: https://kubernetes.io/docs/concepts/workloads/pods/ :: what a pod is";

#[test]
fn takes_the_kind_off_its_head_and_only_a_kind_a_search_may_write() {
  assert_eq!(result_head(STUDY).kind.as_deref(), Some("learning"));
  assert_eq!(result_head("KIND: Learning\nTITLE: a").kind.as_deref(), Some("learning"));
  assert_eq!(result_head("KIND: page\nTITLE: a").kind, None);
  assert_eq!(result_head("KIND: import\nTITLE: a").kind, None);
  assert_eq!(result_head("KIND: diary\nTITLE: a").kind, None);
  assert!(!result_findings(STUDY)[0].text.contains("KIND:"));
}

// Study material's parts, chapters and sections sit under the document's result, the levels between them
// kept.
#[test]
fn sets_its_parts_chapters_and_sections_under_the_documents_result_the_levels_between_them_kept() {
  let rendered = document(STUDY, &none_kept());

  assert!(rendered.contains(&format!("## {}\n\n### Before you start", WORDING.search.result.heading)));
  assert!(rendered.contains("### Part 1 Getting started\n#### Chapter 1 At a glance\n##### Goals"));
  assert!(rendered.contains("### What could not be confirmed"));
  assert!(rendered.contains("```yaml\n# a comment is no heading\n```"));
  assert!(!rendered.contains(&part("SUMMARY")));
}
