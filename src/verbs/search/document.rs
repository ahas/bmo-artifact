//! What a search leaves behind, made out of what the run said and what it did (`docs/archive.md`). A search
//! run is given the web and its branches and nothing else, so it cannot write its own document: the runner
//! writes it from two materials. The result says what was found and, on its SOURCE lines, which page each
//! claim rests on; the stream says which pages were opened and which branch opened them, the ones the run
//! cited nothing from included. Everything here reads and writes text alone: the record puts what it makes
//! where it lives.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

use chrono_tz::Tz;
use regex::Regex;

use super::result::is_answered_line;
use crate::helpers::marks::fill_marks;
use crate::helpers::run_log::json::Json;
use crate::helpers::run_log::lines::{BRANCH_TOOLS, stream_lines};
use crate::helpers::toolbook::mcp_call_of;
use crate::kinds::known;
use crate::verbs::archive_docs::document_time;
use crate::wording::Wording;
use crate::wording::search::Search;

// The line that separates one entry of a set from the next, and the line a claim's page arrives on.
static ITEM_BREAK: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[ \t]*===ITEM===[ \t]*$").expect("a pattern"));
static SOURCE_LINE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^[ \t]*SOURCE(?:[ \t]*\[[ \t]*(\d+)[ \t]*(†)?[ \t]*\])?[ \t]*:[ \t]*(\S+)(?:[ \t]*::[ \t]*(.*?))?[ \t]*$")
    .expect("a pattern")
});
static NAMED_LINE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^[ \t]*(?:NAME|TITLE|VERDICT|SUMMARY|SECTION):[ \t]*(.+?)[ \t]*$").expect("a pattern")
});

// The head of a result, kept apart from its body.
static HEAD_LINE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^[ \t]*(TITLE|TOPIC|KEYWORDS|RULE|KIND|DOCUMENT):[ \t]*(.*?)[ \t]*$").expect("a pattern")
});

// The labels that open a fixed place of the body alone on their line, an entry's lines, and a section.
static PART_LINE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^[ \t]*(VERDICT|SUMMARY|INTRO|BODY|CONCLUSION|UNCONFIRMED|MORE):[ \t]*$").expect("a pattern")
});
static FIELD_LINE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r"^[ \t]*(NAME|ADDRESS|MAP|ROADVIEW|MENU|RELEASED|NEWER|STOCK|DEAL|IMAGE|REVIEW):[ \t]*(.*?)[ \t]*$",
  )
  .expect("a pattern")
});
static SECTION_LINE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[ \t]*SECTION:[ \t]*(.+?)[ \t]*$").expect("a pattern"));

// The run's word that what it found is an answer rather than a reading document, and what a result with a
// body carries, over which that word does not hold.
static DOCUMENT_LINE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"(?mi)^[ \t]*DOCUMENT:[ \t]*(\S+)").expect("a pattern"));
static BODY_LINE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r"(?mi)^(?:[ \t]*(?:SECTION|INTRO|BODY|CONCLUSION|MORE|NAME|MENU|IMAGE|REVIEW|===ITEM===)|#{1,6}[ \t])",
  )
  .expect("a pattern")
});

static KEYWORD_BREAK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[,，、]").expect("a pattern"));
static FIELD_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t]*::[ \t]*").expect("a pattern"));
static BLANK_RUN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n{3,}").expect("a pattern"));
static AUTHORED_HEADING: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^(#{1,6})([ \t].*)$").expect("a pattern"));
static FENCE_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[ \t]*(```|~~~)").expect("a pattern"));
static CALL_KIND: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(.*\)$").expect("a pattern"));

// The one MCP server the web reaches every agent through.
const WEB_SERVER: &str = "web";

// How far a heading may run when a block carries no line naming it.
const HEADING_CHARS: usize = 60;

/// The head a result gives its document; the first line of each kind wins, and an empty one says nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResultHead {
  pub title: Option<String>,
  pub topic: Option<String>,
  pub keywords: Vec<String>,
  pub rule: Option<String>,
  pub kind: Option<String>,
}

pub fn result_head(result: &str) -> ResultHead {
  let mut head = ResultHead::default();

  for line in result.split('\n') {
    let Some(found) = HEAD_LINE.captures(line) else { continue };
    let value = found.get(2).map_or("", |value| value.as_str());

    if value.is_empty() {
      continue;
    }

    match &found[1] {
      "TITLE" if head.title.is_none() => head.title = Some(value.to_owned()),
      "TOPIC" if head.topic.is_none() => head.topic = Some(value.to_owned()),
      "RULE" if head.rule.is_none() => head.rule = Some(value.to_owned()),
      "KIND" if head.kind.is_none() => {
        let lower = value.to_lowercase();

        // Only a kind a search may write itself is taken; any other word leaves the row's own kind
        // standing, since a run cannot make its result a page or an import by saying so.
        head.kind = known(Some(&lower)).filter(|kind| kind.searched()).map(|kind| kind.id().to_owned());
      }
      "KEYWORDS" if head.keywords.is_empty() => {
        head.keywords = KEYWORD_BREAK
          .split(value)
          .map(str::trim)
          .filter(|word| !word.is_empty())
          .map(str::to_owned)
          .collect();
      }
      _ => {}
    }
  }

  head
}

/// Whether a result says it is an answer that leaves no document: a question after one value is met in the
/// room, unless the result carries a body, which would be lost with nothing left to read it in.
pub fn leaves_no_document(result: &str) -> bool {
  let said = DOCUMENT_LINE
    .captures(result)
    .and_then(|found| found.get(1))
    .map(|found| found.as_str().to_lowercase())
    .unwrap_or_default();

  (said == "no" || said == "none") && !BODY_LINE.is_match(result)
}

/// A page the result named under a claim, what it said it stood on there, and the footnote mark it is cited by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CitedSource {
  pub url: String,
  pub supports: Option<String>,
  pub mark: Option<String>,
}

/// One claim-bearing block of a result, the pages cited under it, and the branch it came out of where the pages
/// say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
  pub text: String,
  pub sources: Vec<CitedSource>,
  pub branch: Option<String>,
}

/// The result as findings: a set of comparable things as many as it has entries, a single answer one. A
/// block with no source line is a finding with no pages; the line saying whether it answered is no claim.
pub fn result_findings(result: &str) -> Vec<Finding> {
  let mut findings = vec![];

  for block in blocks_of(&from_verdict(result)) {
    let mut sources = vec![];
    let mut kept = vec![];

    for line in block {
      if is_answered_line(line) || HEAD_LINE.is_match(line) {
        continue;
      }

      match SOURCE_LINE.captures(line) {
        None => kept.push(line),
        Some(cited) => sources.push(CitedSource {
          url: cited.get(3).map_or("", |url| url.as_str()).to_owned(),
          supports: cited
            .get(4)
            .map(|said| said.as_str())
            .filter(|said| !said.is_empty())
            .map(str::to_owned),
          mark: cited
            .get(1)
            .map(|mark| format!("{}{}", mark.as_str(), cited.get(2).map_or("", |dagger| dagger.as_str()))),
        }),
      }
    }

    let text = kept.join("\n").trim().to_owned();

    if text.is_empty() && sources.is_empty() {
      continue;
    }

    findings.push(Finding {
      text,
      sources,
      branch: None,
    });
  }

  findings
}

// The result from its verdict line on: anything a run wrote above it is narration on the way.
fn from_verdict(result: &str) -> String {
  let lines: Vec<&str> = result.split('\n').collect();

  match lines.iter().position(|line| is_answered_line(line)) {
    Some(at) if at > 0 => lines[at..].join("\n"),
    _ => result.to_owned(),
  }
}

fn blocks_of(result: &str) -> Vec<Vec<&str>> {
  let mut blocks: Vec<Vec<&str>> = vec![vec![]];

  for line in result.split('\n') {
    if ITEM_BREAK.is_match(line) {
      blocks.push(vec![]);
    } else if let Some(last) = blocks.last_mut() {
      last.push(line);
    }
  }

  blocks
}

/// A page a run opened, and the branch whose call opened it; none is the run itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedPage {
  pub url: String,
  pub branch: Option<String>,
}

/// A question a run put to the web, and the branch that put it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskedQuery {
  pub query: String,
  pub branch: Option<String>,
}

/// What the stream says the run did, sorted by whose doing it was, the branches in the order they first appear;
/// with the run it was read from, since a branch's label is its run's own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trace {
  pub run_id: Option<i64>,
  pub branches: Vec<Option<String>>,
  pub pages: Vec<OpenedPage>,
  pub queries: Vec<AskedQuery>,
}

/// The stream as what the run opened and asked, branch by branch, read as the run log reads it: a call this
/// finds is a call the log has a line for, and one page opened twice by one branch is one page.
pub fn read_stream(stream: &str, run_id: Option<i64>) -> Trace {
  let mut labels: HashMap<String, String> = HashMap::new();
  let mut counted: HashMap<String, usize> = HashMap::new();
  let mut trace = Trace {
    run_id,
    ..Trace::default()
  };
  let mut seen: HashSet<String> = HashSet::new();

  for line in stream.split('\n').map(str::trim).filter(|line| line.starts_with('{')) {
    let Some(event) = Json::parse(line) else { continue };

    for logged in stream_lines(&event) {
      let Some(name) = logged.name.as_deref().filter(|_| logged.kind == "tool") else {
        continue;
      };

      // A call opening a branch is remembered under the id it was made with, which every line inside the
      // branch carries back as its parent; numbered, since two branches of one kind are two branches.
      if BRANCH_TOOLS.contains(&CALL_KIND.replace(name, "").as_ref())
        && let Some(reference) = &logged.reference
      {
        let nth = counted.entry(name.to_owned()).or_insert(0);

        *nth += 1;
        labels.insert(reference.clone(), format!("{name} {nth}"));
        continue;
      }

      if mcp_call_of(name).map(|(server, _)| server).as_deref() != Some(WEB_SERVER) {
        continue;
      }

      let branch = logged
        .parent
        .as_ref()
        .map(|parent| labels.get(parent).cloned().unwrap_or_else(|| parent.clone()));
      let input: serde_json::Value = serde_json::from_str(&logged.text).unwrap_or_default();
      let url = input.get("url").and_then(serde_json::Value::as_str);
      let query = input.get("query").and_then(serde_json::Value::as_str);

      if url.is_none() && query.is_none() {
        continue;
      }

      if !trace.branches.contains(&branch) {
        trace.branches.push(branch.clone());
      }

      if let Some(url) = url
        && seen.insert(format!("{} {}", branch.as_deref().unwrap_or_default(), normal_url(url)))
      {
        trace.pages.push(OpenedPage {
          url: url.to_owned(),
          branch: branch.clone(),
        });
      }

      if let Some(query) = query {
        trace.queries.push(AskedQuery {
          query: query.to_owned(),
          branch,
        });
      }
    }
  }

  trace
}

// Two writings of one page: a trailing slash and a fragment are the same page under another name.
fn normal_url(url: &str) -> String {
  url.split('#').next().unwrap_or(url).trim_end_matches('/').to_owned()
}

/// Which branch each finding came out of, read off the pages it cites: one branch alone's pages make it that
/// branch's; pages from two, or pages no call can be found for, leave it the result's.
pub fn attribute_branches(findings: Vec<Finding>, trace: &Trace) -> Vec<Finding> {
  let mut openers: HashMap<String, Vec<Option<String>>> = HashMap::new();

  for page in &trace.pages {
    let opened = openers.entry(normal_url(&page.url)).or_default();

    if !opened.contains(&page.branch) {
      opened.push(page.branch.clone());
    }
  }

  findings
    .into_iter()
    .map(|finding| {
      let mut from: Vec<Option<String>> = vec![];

      for source in &finding.sources {
        for branch in openers.get(&normal_url(&source.url)).into_iter().flatten() {
          if !from.contains(branch) {
            from.push(branch.clone());
          }
        }
      }

      Finding {
        branch: if from.len() == 1 { from.remove(0) } else { None },
        ..finding
      }
    })
    .collect()
}

/// The pages kept under the search itself rather than under a claim: opened and cited nothing from, a page
/// already under a claim or already held not written again, and one page however many calls reached it.
pub fn uncited_pages(trace: &Trace, findings: &[Finding], already: &[String]) -> Vec<OpenedPage> {
  let cited: HashSet<String> = findings
    .iter()
    .flat_map(|finding| finding.sources.iter().map(|source| normal_url(&source.url)))
    .chain(already.iter().map(|url| normal_url(url)))
    .collect();
  let mut kept: Vec<OpenedPage> = vec![];
  let mut held: HashSet<String> = HashSet::new();

  for page in &trace.pages {
    let at = normal_url(&page.url);

    if !cited.contains(&at) && held.insert(at) {
      kept.push(page.clone());
    }
  }

  kept
}

/// A page cited under a finding or kept under the search, as the record holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldSource {
  pub url: String,
  pub supports: Option<String>,
  pub mark: Option<String>,
}

/// A finding as the record holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldFinding {
  pub run_id: Option<i64>,
  pub branch: Option<String>,
  pub text: String,
  pub sources: Vec<HeldSource>,
}

/// The whole of one search as its document is written from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Record {
  pub query: String,
  pub started_depth: String,
  pub ended_depth: Option<String>,
  pub status: String,
  pub started_at: i64,
  pub finished_at: Option<i64>,
  pub options: Vec<(String, String)>,
  pub head: ResultHead,
  /// The runs that have carried the search, as bmo-job keeps them, in the order they were queued.
  pub runs: Vec<i64>,
  pub findings: Vec<HeldFinding>,
  pub pages: Vec<HeldSource>,
}

/// The document a person reads: what the search was, the result the room was given, and under it, folded,
/// the trail that led there, in `wording`'s words with its times told in `zone`. `kept` is where each
/// picture's own copy was kept, by the address the result named it at.
pub fn render(
  search: &Record,
  trace: &Trace,
  kept: &BTreeMap<String, String>,
  wording: &Wording,
  zone: Tz,
) -> String {
  let words = &wording.search;
  let trail = branch_sections(search, trace, words);
  let mut lines = heading(search, wording, zone);

  lines.extend(result_section(search, kept, words));

  if !trail.is_empty() {
    lines.extend([
      format!("## {}", words.trail.heading),
      String::new(),
      "<details>".to_owned(),
      format!("<summary>{}</summary>", words.trail.summary),
      String::new(),
    ]);
    lines.extend(trail);
    lines.extend(["</details>".to_owned(), String::new()]);
  }

  format!("{}\n", lines.join("\n").trim())
}

fn heading(search: &Record, wording: &Wording, zone: Tz) -> Vec<String> {
  let words = &wording.search.head;
  let time = |seconds: i64| document_time(seconds, zone, &wording.archive_docs.time);
  let line = |template: &str, value: &str| fill_marks(template, &[("value", value)]);
  let depth = match &search.ended_depth {
    Some(ended) if *ended != search.started_depth => format!("{} → {ended}", search.started_depth),
    _ => search.started_depth.clone(),
  };
  let head = &search.head;
  let mut lines = vec![
    format!(
      "# {}",
      head
        .title
        .clone()
        .unwrap_or_else(|| fill_marks(&words.untitled, &[("query", &search.query)]))
    ),
    String::new(),
  ];

  if head.title.is_some() {
    lines.push(line(&words.query, &search.query));
  }

  if let Some(topic) = &head.topic {
    lines.push(line(&words.topic, topic));
  }

  if !head.keywords.is_empty() {
    lines.push(line(&words.keywords, &head.keywords.join(", ")));
  }

  if let Some(rule) = &head.rule {
    lines.push(line(&words.rule, rule));
  }

  for (name, value) in &search.options {
    lines.push(fill_marks(&words.option, &[("name", name), ("value", value)]));
  }

  lines.push(line(&words.depth, &depth));
  lines.push(line(&words.status, &search.status));
  lines.push(line(&words.started, &time(search.started_at)));

  if let Some(finished) = search.finished_at {
    lines.push(line(&words.finished, &time(finished)));
  }

  if !search.runs.is_empty() {
    lines.push(line(
      &words.runs,
      &search.runs.iter().map(i64::to_string).collect::<Vec<_>>().join(", "),
    ));
  }

  lines.push(String::new());
  lines
}

// Which run a branch was opened in, said only where the search took more than one.
fn run_mark(search: &Record, run_id: Option<i64>, words: &Search) -> String {
  match run_id {
    Some(run) if search.runs.len() > 1 => fill_marks(&words.trail.run, &[("run", &run.to_string())]),
    _ => String::new(),
  }
}

fn branch_sections(search: &Record, trace: &Trace, words: &Search) -> Vec<String> {
  let trail = &words.trail;
  let mut keys: Vec<(Option<i64>, Option<String>)> = vec![];
  let mut note = |run_id: Option<i64>, branch: Option<String>| {
    if !keys.iter().any(|key| key.0 == run_id && key.1 == branch) {
      keys.push((run_id, branch));
    }
  };

  // The rungs already spent come first, in the order their findings were written, and this run last.
  for finding in &search.findings {
    if finding.branch.is_some() {
      note(finding.run_id, finding.branch.clone());
    }
  }

  for branch in &trace.branches {
    note(trace.run_id, branch.clone());
  }

  let mut lines = vec![];

  for (run_id, branch) in &keys {
    let mine = trace.run_id.is_none() || *run_id == trace.run_id;
    let asked: Vec<&AskedQuery> = if mine {
      trace.queries.iter().filter(|query| query.branch == *branch).collect()
    } else {
      vec![]
    };
    let opened: Vec<&OpenedPage> = if mine {
      trace.pages.iter().filter(|page| page.branch == *branch).collect()
    } else {
      vec![]
    };
    let found: Vec<&HeldFinding> = search
      .findings
      .iter()
      .filter(|finding| finding.branch == *branch && finding.run_id == *run_id)
      .collect();

    lines.push(format!(
      "### {}{}",
      branch.as_deref().unwrap_or(&trail.own),
      run_mark(search, *run_id, words)
    ));
    lines.push(String::new());

    if !asked.is_empty() {
      lines.push(trail.asked.clone());
      lines.extend(asked.iter().map(|query| format!("- {}", query.query)));
      lines.push(String::new());
    }

    if !opened.is_empty() {
      lines.push(trail.opened.clone());
      lines.extend(opened.iter().map(|page| format!("- {}", page.url)));
      lines.push(String::new());
    }

    if !found.is_empty() {
      lines.push(trail.found.clone());
      lines.extend(found.iter().map(|finding| format!("- {}", first_line(&finding.text))));
      lines.push(String::new());
    }
  }

  lines
}

// The result the room was given, every finding in the order it was written, its structure lines written as
// what they stand for.
fn result_section(search: &Record, kept: &BTreeMap<String, String>, words: &Search) -> Vec<String> {
  let result = &words.result;
  let mut lines = vec![format!("## {}", result.heading), String::new()];

  if search.findings.is_empty() {
    lines.extend([result.empty.clone(), String::new()]);
    return lines;
  }

  for (index, finding) in search.findings.iter().enumerate() {
    if index > 0 {
      lines.extend(["---".to_owned(), String::new()]);
    }

    if let Some(branch) = &finding.branch {
      lines.push(format!(
        "{}{}",
        fill_marks(&result.branch, &[("branch", branch)]),
        run_mark(search, finding.run_id, words)
      ));
      lines.push(String::new());
    }

    lines.push(readable(&finding.text, kept, words));
    lines.push(String::new());

    if !finding.sources.is_empty() {
      lines.push(result.sources.clone());

      for source in &finding.sources {
        let mark = source.mark.as_ref().map_or_else(String::new, |mark| format!("[{mark}] "));

        lines.push(match &source.supports {
          None => format!("- {mark}{}", source.url),
          Some(supports) => format!("- {mark}{} :: {supports}", source.url),
        });
      }

      lines.push(String::new());
    }
  }

  lines
}

// One line of an entry written as what it stands for.
fn field(label: &str, value: &str, kept: &BTreeMap<String, String>, words: &Search) -> String {
  let said = |name: &str| fill_marks(words.field(name), &[("value", value)]);
  let pieces: Vec<&str> = FIELD_SPLIT.split(value).collect();
  let piece = |at: usize| pieces.get(at).copied().unwrap_or_default();

  match label {
    "NAME" => format!("\n### {value}\n"),
    "MENU" if value.is_empty() => words.field("menu_alone").to_owned(),
    "ADDRESS" | "MAP" | "ROADVIEW" | "MENU" | "RELEASED" | "NEWER" => said(label),
    // What the product page said of itself, when it was read and where: three facts that only mean anything
    // together.
    "STOCK" => fill_marks(
      words.field("STOCK"),
      &[(
        "value",
        &[piece(0), piece(1), piece(2)]
          .into_iter()
          .filter(|part| !part.is_empty())
          .collect::<Vec<_>>()
          .join(" · "),
      )],
    ),
    "DEAL" => {
      let usual = if piece(1).is_empty() {
        String::new()
      } else {
        fill_marks(words.field("deal_usual"), &[("usual", piece(1))])
      };
      let real = if piece(2).is_empty() {
        String::new()
      } else {
        fill_marks(words.field("deal_real"), &[("real", piece(2))])
      };

      fill_marks(words.field("deal"), &[("now", piece(0)), ("usual", &usual), ("real", &real)])
    }
    // One review as it was written: the rating the source gave it, the reviewer's words and their page.
    "REVIEW" => {
      let rating = piece(0);
      let scored = if rating.is_empty() || rating == words.field("no_rating") {
        String::new()
      } else {
        format!("**{rating}** ")
      };
      let page = piece(2);
      let link = if page.is_empty() {
        String::new()
      } else {
        fill_marks(words.field("review_link"), &[("page", page)])
      };

      format!("> {scored}{}{link}", piece(1))
    }
    // A picture: the copy kept where one was, with the address it came from under it, captioned with what it
    // shows, its shot and whose it is; its entry is the heading it already stands under.
    _ => {
      let href = piece(0);
      let caption = [piece(1), piece(3), piece(4)]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");

      match kept.get(href) {
        None => format!("![{caption}]({href})"),
        Some(copy) => {
          format!("![{caption}]({copy})\n{}", fill_marks(words.field("original"), &[("address", href)]))
        }
      }
    }
  }
}

// A finding's text with its structure lines written as the headings, labels, links and pictures they stand
// for; any other line stays as it was written.
fn readable(text: &str, kept: &BTreeMap<String, String>, words: &Search) -> String {
  let lines: Vec<String> = under_result(text)
    .split('\n')
    .map(|line| {
      if let Some(part) = PART_LINE.captures(line) {
        let label = &part[1];

        return format!("\n#### {}\n", words.parts.get(label).map_or(label, String::as_str));
      }

      if let Some(section) = SECTION_LINE.captures(line) {
        return format!("\n#### {}\n", &section[1]);
      }

      match FIELD_LINE.captures(line) {
        None => line.to_owned(),
        Some(found) => field(&found[1], found.get(2).map_or("", |value| value.as_str()), kept, words),
      }
    })
    .collect();

  BLANK_RUN.replace_all(&lines.join("\n"), "\n\n").trim().to_owned()
}

// The headings a result wrote itself moved down so the shallowest stands under the document's title and its
// result, the levels between them kept; a result that wrote none returned as it was.
fn under_result(text: &str) -> String {
  let lines: Vec<&str> = text.split('\n').collect();
  let mut fenced = false;
  let mut shallowest = 7;

  for line in &lines {
    if FENCE_LINE.is_match(line) {
      fenced = !fenced;
    } else if !fenced && let Some(heading) = AUTHORED_HEADING.captures(line) {
      shallowest = shallowest.min(heading[1].len());
    }
  }

  let shift = if shallowest == 7 { 0 } else { 3usize.saturating_sub(shallowest) };

  if shift == 0 {
    return text.to_owned();
  }

  fenced = false;

  lines
    .iter()
    .map(|line| {
      if FENCE_LINE.is_match(line) {
        fenced = !fenced;
        return (*line).to_owned();
      }

      match AUTHORED_HEADING.captures(line).filter(|_| !fenced) {
        None => (*line).to_owned(),
        Some(heading) => format!("{}{}", "#".repeat((heading[1].len() + shift).min(6)), &heading[2]),
      }
    })
    .collect::<Vec<_>>()
    .join("\n")
}

// What a block is called where it is listed: the line naming it, or the first thing it says, a part label
// alone on its line passed over.
fn first_line(text: &str) -> String {
  let first = text
    .split('\n')
    .map(str::trim)
    .find(|line| !line.is_empty() && !PART_LINE.is_match(line))
    .unwrap_or_default();
  let head = NAMED_LINE
    .captures(first)
    .map_or(first, |named| named.get(1).map_or("", |name| name.as_str()));
  let units: Vec<u16> = head.encode_utf16().collect();

  if units.len() > HEADING_CHARS {
    format!("{}…", String::from_utf16_lossy(&units[..HEADING_CHARS]))
  } else {
    head.to_owned()
  }
}
