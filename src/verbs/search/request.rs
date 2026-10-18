//! A search as it is asked (`docs/archive.md`, "What the search is given"): the three depths of the ladder,
//! the options a launch may put on any search, the request file the launcher writes and the runner reads, and
//! the run's prompt made of it.

use chrono::TimeZone;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::helpers::lines::{Trees, read_required, render};

/// One rung of the ladder: its name, and whether a run on it is a deep dive, whose result may carry a
/// narration to be spoken and kept with its document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Depth {
  pub name: &'static str,
  pub deep_dive: bool,
}

/// The ladder, shallowest first: escalation moves one place along it.
pub const DEPTHS: [Depth; 3] = [
  Depth {
    name: "basic",
    deep_dive: false,
  },
  Depth {
    name: "thorough",
    deep_dive: true,
  },
  Depth {
    name: "exhaustive",
    deep_dive: true,
  },
];

/// The rung a search runs on when nobody names one.
pub const DEFAULT_DEPTH: &str = "basic";

/// The rungs' names as a usage line writes them.
pub fn depth_names() -> String {
  DEPTHS.iter().map(|depth| depth.name).collect::<Vec<_>>().join("|")
}

/// The research key a search of a depth reads its settings and its tier under.
pub fn search_key(depth: &str) -> String {
  format!("search:{depth}")
}

/// Whether a word names a rung.
pub fn is_depth(value: &str) -> bool {
  DEPTHS.iter().any(|depth| depth.name == value)
}

/// A rung's place on the ladder, `0` for the shallowest, as the run ledger records a run's depth; a name off
/// the ladder is the shallowest, as `depth` reads it.
pub fn ladder_place(value: &str) -> i64 {
  DEPTHS.iter().position(|depth| depth.name == value).map_or(0, |place| place as i64)
}

/// The rung a runner was handed; an unknown name is the shallowest, since a stale caller's guess must only
/// ever cost a thinner answer and never a wider search nobody asked for.
pub fn depth(value: Option<&str>) -> Depth {
  DEPTHS.iter().find(|depth| Some(depth.name) == value).copied().unwrap_or(DEPTHS[0])
}

/// The regions a launch may name.
pub const REGIONS: [&str; 2] = ["domestic", "global"];

/// The shapes a launch may name; `learning` asks for study material, a document kind of its own.
pub const SHAPES: [&str; 4] = ["brief", "table", "full", "learning"];

/// Who a search answers: the sender of the message it was launched for, as the chat's record names them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asker {
  pub id: i64,
  pub name: String,
}

/// The options a launch put on the search; one left out is the run's own judgement from the question. They
/// are written, the ones set alone, in the order a launch reads them, which is the order a row keeps them in.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Options {
  /// The one page to bring in whole: a search carrying it researches nothing.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub region: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub shape: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub period: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub exclude: Option<String>,
}

impl Options {
  /// The options a row keeps, by name, the ones set alone, in the order a launch reads them.
  pub fn named(&self) -> Vec<(&'static str, &str)> {
    [
      ("source", &self.source),
      ("region", &self.region),
      ("shape", &self.shape),
      ("period", &self.period),
      ("exclude", &self.exclude),
    ]
    .into_iter()
    .filter_map(|(name, value)| value.as_deref().map(|value| (name, value)))
    .collect()
  }
}

/// The options as a launcher was given them, or why they are refused: a region or a shape outside the known
/// words is what somebody asked for, and running without it would answer a different question.
pub fn options(
  period: Option<&str>,
  region: Option<&str>,
  exclude: Option<&str>,
  shape: Option<&str>,
  source: Option<&str>,
) -> Result<Options, String> {
  let mut options = Options::default();

  if let Some(source) = source {
    let source = source.trim();

    if !is_web_address(source) {
      return Err("--import must be a web address (http or https)".to_owned());
    }

    options.source = Some(source.to_owned());
  }

  if let Some(region) = region {
    if !REGIONS.contains(&region) {
      return Err(format!("--region must be one of {}", REGIONS.join("|")));
    }

    options.region = Some(region.to_owned());
  }

  if let Some(shape) = shape {
    if !SHAPES.contains(&shape) {
      return Err(format!("--shape must be one of {}", SHAPES.join("|")));
    }

    options.shape = Some(shape.to_owned());
  }

  options.period = period.map(str::trim).filter(|period| !period.is_empty()).map(str::to_owned);
  options.exclude = exclude.map(str::trim).filter(|exclude| !exclude.is_empty()).map(str::to_owned);

  Ok(options)
}

fn is_web_address(value: &str) -> bool {
  url::Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

/// What the launcher hands the runner: the question, who asked it, when in epoch seconds, and the options.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
  pub query: String,
  pub asker: Option<Asker>,
  pub asked_at: Option<i64>,
  pub options: Options,
}

/// The request file the runner reads; a file that is not a request object is the question alone, which is
/// what a caller that knows nothing of askers and options writes.
pub fn read_request(text: &str) -> Request {
  let trimmed = text.trim();
  let alone = || Request {
    query: trimmed.to_owned(),
    ..Request::default()
  };
  let Ok(serde_json::Value::Object(parsed)) = serde_json::from_str::<serde_json::Value>(trimmed) else {
    return alone();
  };
  let Some(query) = parsed.get("query").and_then(serde_json::Value::as_str) else {
    return alone();
  };
  let asker = parsed.get("asker").and_then(|asker| {
    Some(Asker {
      id: asker.get("id")?.as_i64()?,
      name: asker.get("name")?.as_str()?.to_owned(),
    })
  });
  let asked_at = parsed.get("askedAt").and_then(serde_json::Value::as_i64).filter(|at| *at > 0);
  let options = parsed
    .get("options")
    .filter(|options| options.is_object())
    .and_then(|options| serde_json::from_value(options.clone()).ok())
    .unwrap_or_default();

  Request {
    query: query.trim().to_owned(),
    asker,
    asked_at,
    options,
  }
}

/// When a question was asked, as the run's prompt says it: the moment in the settings' time zone, with the
/// zone's abbreviation and the weekday.
pub fn asked_at_text(epoch_seconds: i64, zone: Tz) -> String {
  zone
    .timestamp_opt(epoch_seconds, 0)
    .single()
    .map_or_else(|| epoch_seconds.to_string(), |at| at.format("%Y-%m-%d %H:%M %Z, %A").to_string())
}

/// Who asked, as the run's prompt says it: who asked decides nothing by itself, and is read only by a
/// rulebook that has already found itself to apply.
pub fn asker_text(asker: Option<&Asker>) -> String {
  match asker {
    None => "not known from the launch. Where a rulebook below applies to this search and decides by who asked, apply what it says for an asker who is not known.".to_owned(),
    Some(asker) => format!(
      "{} (Telegram id {}). Where a rulebook below applies to this search and decides by who asked, apply what it says for this person. Who asked never brings a rulebook into play by itself: whether a rulebook applies is settled by what was asked, and by that rulebook alone.",
      asker.name, asker.id
    ),
  }
}

/// The options, as the run's prompt says them.
pub fn options_text(options: &Options) -> String {
  let mut parts = vec![];

  if let Some(period) = &options.period {
    parts.push(format!("period: {period}"));
  }

  if let Some(region) = &options.region {
    parts.push(format!(
      "region: {}",
      if region == "domestic" { "domestic material first" } else { "overseas material too" }
    ));
  }

  if let Some(exclude) = &options.exclude {
    parts.push(format!("excluded sources: {exclude}"));
  }

  if let Some(shape) = &options.shape {
    parts.push(format!("result shape: {shape}"));
  }

  if parts.is_empty() {
    "none given; decide them from the question.".to_owned()
  } else {
    format!("{}. Any option not named here is decided from the question.", parts.join("; "))
  }
}

/// How many branches a run may open, in the prompt's words.
pub fn branch_allowance(max_children: i64) -> String {
  match max_children {
    0 => "no branch at all".to_owned(),
    1 => "one branch".to_owned(),
    many => format!("up to {many} branches"),
  }
}

/// One search run's request as its prompt, written into the search agent's `request.md`: the question, who
/// asked and when in `zone`, the options, the depth and the branch allowance.
pub fn task(
  trees: &Trees,
  request: &Request,
  depth: &str,
  max_children: i64,
  now_seconds: i64,
  zone: Tz,
) -> Result<String, crate::Refusal> {
  let template = read_required(&trees.agent_dir("search").join("request.md"))?;

  Ok(render(
    &template,
    &[
      ("query", &request.query),
      ("depth", depth),
      ("branches", &branch_allowance(max_children)),
      ("asker", &asker_text(request.asker.as_ref())),
      ("askedAt", &asked_at_text(request.asked_at.unwrap_or(now_seconds), zone)),
      ("options", &options_text(&request.options)),
    ],
  ))
}
