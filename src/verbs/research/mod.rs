//! What any research run — a search or a browse — stands on beside its own work (`docs/research-runs.md`):
//! what its web tools are told, the profile it launched on handed to its runner as it stood, the registration
//! it was set off under where that keeps its research, its clock, its branches' places in the common
//! co-worker pool, and the knowledge its words call for.
//!
//! - `admission` — whether a deeper run goes as asked or as the shallowest, and the wait an explicit depth
//!   makes for its first seat.
//! - `run` — the run carried apart by bmo-agency, followed as it goes, stopped from outside, and closed with
//!   its row and its pool places.
//! - `branch` — `bmo search branch`, one branch of a run on a place of the pool.

pub mod admission;
pub mod branch;
pub mod run;

use std::collections::BTreeMap;
use std::future::Future;

use chrono::TimeZone;
use chrono_tz::Tz;
use sea_orm::DatabaseConnection;
use serde_json::{Map, Value, json};

use crate::Refusal;
use crate::clients::agency::{Agency, Reservation, ReservationAsking};
use crate::clients::gate::Gate;
use crate::clients::job::Job;
use crate::control::research::{self as profiles, Values, child_mapping, research_provider};
use crate::control::whole;
use crate::settings::Research;

/// The line every research run of this module goes out on, and the agent it is counted under.
pub const LINE: &str = "search";

/// What a research run's web tools are told about the run (`docs/research-runs.md`, "What the settings
/// open"): each value set is one variable, and one left unset is no variable at all.
#[derive(Clone, Debug, Default)]
pub struct Told {
  /// Epoch milliseconds after which the web tools refuse new work.
  pub closes_at: Option<i64>,
  /// The pool owner the run's branches are held under.
  pub owner: Option<String>,
  pub source: Option<String>,
  pub role: Option<String>,
  pub max_children: Option<i64>,
  pub chat_id: Option<i64>,
  pub strategy: Option<String>,
  pub host_interval_ms: Option<i64>,
  pub engine_interval_ms: Option<i64>,
  /// Whether the tools serve the branch tool.
  pub branch_tool: bool,
  /// Whether the tools serve the picture viewer.
  pub image_tool: bool,
  /// The rulebooks the tools hand out whole by name.
  pub rulebooks: Vec<String>,
}

impl Told {
  /// The variables the tools read them from, in order.
  pub fn variables(&self) -> Vec<(&'static str, String)> {
    let mut told = Vec::new();
    let mut set = |name: &'static str, value: Option<String>| {
      if let Some(value) = value.filter(|value| !value.is_empty()) {
        told.push((name, value));
      }
    };

    set("BMO_RESEARCH_TOOLS", self.branch_tool.then(|| "1".to_owned()));
    set("BMO_RESEARCH_IMAGES", self.image_tool.then(|| "1".to_owned()));
    set("BMO_RESEARCH_RULEBOOKS", Some(self.rulebooks.join(",")));
    set("BMO_RESEARCH_CLOSES_AT", self.closes_at.map(|at| at.to_string()));
    set("BMO_RESEARCH_OWNER", self.owner.clone());
    set("BMO_RESEARCH_SOURCE", self.source.clone());
    set("BMO_RESEARCH_ROLE", self.role.clone());
    set("BMO_RESEARCH_BRANCHES", self.max_children.map(|count| count.to_string()));
    set("BMO_RESEARCH_CHAT", self.chat_id.map(|chat| chat.to_string()));
    set("BMO_WEB_STRATEGY", self.strategy.clone());
    set("BMO_WEB_HOST_INTERVAL_MS", self.host_interval_ms.map(|ms| ms.to_string()));
    set("BMO_WEB_ENGINE_INTERVAL_MS", self.engine_interval_ms.map(|ms| ms.to_string()));
    told
  }

  /// The same variables as a run apart's environment carries them.
  pub fn environment(&self) -> BTreeMap<String, String> {
    self.variables().into_iter().map(|(name, value)| (name.to_owned(), value)).collect()
  }
}

/// The web settings a research key's profile holds: the strategy and the two intervals its web tools pace
/// themselves by. A key the settings do not hold is refused, as its run would be.
pub async fn web_settings(
  db: &DatabaseConnection,
  research: &Research,
  key: &str,
) -> Result<(String, i64, i64), String> {
  let profile = profile(db, research, key).await.map_err(|refused| refused.detail)?;

  Ok((profile.strategy().to_owned(), profile.host_interval_ms(), profile.engine_interval_ms()))
}

/// The moment a run is asked at, as its prompt names it, in the settings' time zone: `2026-09-17 23:40 KST,
/// Thursday`.
pub fn asked_at_text(epoch_seconds: i64, zone: Tz) -> String {
  zone
    .timestamp_opt(epoch_seconds, 0)
    .single()
    .map_or_else(|| epoch_seconds.to_string(), |at| at.format("%Y-%m-%d %H:%M %Z, %A").to_string())
}

// ---------------------------------------------------------------- the profile

/// One research key's profile as a run launches on it: the key, its values, and when they were set.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
  pub key: String,
  pub values: Values,
  pub set_at: i64,
}

impl Profile {
  /// The whole time a clocked run has, none on a key with no clock.
  pub fn cap_ms(&self) -> Option<i64> {
    whole(&self.values.cap_ms)
  }

  /// The end of that time kept for writing the result, none on a key with no clock.
  pub fn synthesis_reserve_ms(&self) -> Option<i64> {
    whole(&self.values.synthesis_reserve_ms)
  }

  pub fn child_role(&self) -> &str {
    self.values.child_role.as_str().unwrap_or_default()
  }

  pub fn max_children(&self) -> i64 {
    whole(&self.values.max_children).unwrap_or_default()
  }

  pub fn wait_limit_ms(&self) -> i64 {
    whole(&self.values.wait_limit_ms).unwrap_or_default()
  }

  pub fn strategy(&self) -> &str {
    self.values.strategy.as_str().unwrap_or_default()
  }

  pub fn host_interval_ms(&self) -> i64 {
    whole(&self.values.host_interval_ms).unwrap_or_default()
  }

  pub fn engine_interval_ms(&self) -> i64 {
    whole(&self.values.engine_interval_ms).unwrap_or_default()
  }

  /// The profile a run kept on its caller's registration launches on: no branch at any depth, so it reserves
  /// and waits for nothing in the pool and its tools serve no branch.
  pub fn kept_on(mut self, registration: Option<&str>) -> Self {
    if registration.is_some() {
      self.values.max_children = json!(0);
    }

    self
  }

  /// The launch-time copy a launcher hands its runner, so a run keeps the settings it started on.
  pub fn snapshot(&self) -> String {
    let mut fields = Map::new();

    fields.insert("key".into(), json!(self.key));
    fields.extend(self.values.fields());
    fields.insert("setAt".into(), json!(self.set_at));

    Value::Object(fields).to_string()
  }

  /// A launch-time copy read back for the key it must be for, and refused where it is not that key's or
  /// breaks a check the profile's own store holds it to.
  pub fn parse(text: &str, research: &Research, key: &str) -> Result<Self, String> {
    let parsed: Value =
      serde_json::from_str(text).map_err(|_| "The research profile snapshot is not JSON".to_owned())?;

    if !parsed.is_object() {
      return Err("The research profile snapshot is not an object".to_owned());
    }

    let held = parsed.get("key").cloned().unwrap_or(Value::Null);

    if held.as_str() != Some(key) {
      let named = held.as_str().map_or_else(|| held.to_string(), str::to_owned);

      return Err(format!("The research profile snapshot is for {named}, not {key}"));
    }

    let values = Values::of(&parsed);
    // The pool's roles are not read here: the role the snapshot names stands as enabled, since the
    // launcher checked it against the pool when it took the copy.
    let pooled: Vec<String> = match (whole(&values.max_children), values.child_role.as_str()) {
      (Some(children), Some(role)) if children > 0 => vec![role.to_owned()],
      _ => vec![],
    };

    if let Some(fault) = profiles::profile_fault(research, key, &values, &pooled) {
      return Err(format!("The research profile snapshot is invalid: {fault}"));
    }

    Ok(Self {
      key: key.to_owned(),
      values,
      set_at: parsed.get("setAt").and_then(whole).unwrap_or_default(),
    })
  }
}

/// A key's profile as the archive holds it now: its row, or its seed where it has none yet.
pub async fn profile(db: &DatabaseConnection, research: &Research, key: &str) -> Result<Profile, Refusal> {
  profiles::profile(db, research, key)
    .await?
    .map(|held| Profile {
      key: held.key,
      values: held.values,
      set_at: held.set_at,
    })
    .ok_or_else(|| Refusal::bad(format!("Missing research profile: {key}")))
}

// ---------------------------------------------------------------- the clock

/// A clocked run's clock (`docs/research-runs.md`, "The browse clock"): when it started, when its research
/// closes, and its ceiling, each in epoch milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
  pub started_at: i64,
  pub closes_at: i64,
  pub cap_at: i64,
}

/// The clock of a run on this profile started now, none on a key with no clock.
pub fn clock(profile: &Profile, started_at: i64) -> Option<Clock> {
  let (cap, reserve) = (profile.cap_ms()?, profile.synthesis_reserve_ms()?);

  Some(Clock {
    started_at,
    closes_at: started_at + cap - reserve,
    cap_at: started_at + cap,
  })
}

// ---------------------------------------------------------------- a registration that keeps its research

/// The registration a launch is made under, where it keeps its research (`docs/research-runs.md`, "A research
/// run set off on a registration"): the one bmo-agency says owns the home the launching process was handed.
/// None for a launch with no home, and for one whose registration does not keep its research.
pub async fn kept_research_on(agency: &dyn Agency, home: Option<&str>) -> Result<Option<String>, String> {
  let Some(home) = home.filter(|home| !home.is_empty()) else { return Ok(None) };
  let held = agency.registration_by_home(home).await.map_err(|fault| fault.to_string())?;

  Ok(held.filter(|held| held.keeps_research).map(|held| held.name))
}

// ---------------------------------------------------------------- the pool

/// The owner a research run's places are held under in the pool.
pub fn owner(source: &str, run_id: i64) -> String {
  format!("{source}-run:{run_id}")
}

/// The seats a research run's branches will use, held from before its clock starts until it ends, on the
/// provider the search line starts on, where every branch runs.
pub async fn reserve_places(
  agency: &dyn Agency,
  owner: &str,
  source: &str,
  role: &str,
  count: i64,
) -> Result<Reservation, String> {
  research_provider(agency, LINE).await?;

  let mapping = child_mapping(agency, role).await?;
  let asking = ReservationAsking {
    reservation_key: format!("{owner}:branches"),
    owner: owner.to_owned(),
    source: source.to_owned(),
    role: role.to_owned(),
    provider: mapping.provider,
    model: mapping.model,
    effort: mapping.effort,
    tier: mapping.tier,
    wanted: count,
  };

  agency.reserve(&asking).await.map_err(|fault| fault.to_string())
}

/// Everything one run still holds, given back when that run ends however it ended: its branches'
/// reservation, and every request of its own; how many were.
pub async fn finish_leases(agency: &dyn Agency, owner: &str, outcome: &str) -> Result<i64, String> {
  let said = |fault: crate::clients::agency::AgencyFault| fault.to_string();
  let mut finished = 0;

  for held in agency.reservations(Some(owner)).await.map_err(said)? {
    if agency.release(&held.token, outcome).await.map_err(said)? {
      finished += 1;
    }
  }

  for request in agency.requests(Some(owner)).await.map_err(said)? {
    if request.asking.owner != owner || request.asking.reservation.is_some() {
      continue;
    }

    if agency.finish(&request.token, outcome, request.state == "waiting").await.map_err(said)? {
      finished += 1;
    }
  }

  Ok(finished)
}

// Whether the research run an owner names has ended: its run closed and its unit no longer running, or no run
// at all. Time alone proves nothing, since a branch inside a standing unit may still be running.
async fn ended<F, Fut>(job: &Job, owner: &str, source: &str, running: &F) -> Result<bool, String>
where
  F: Fn(String) -> Fut,
  Fut: Future<Output = bool>,
{
  let Some((kind, id)) = owner.split_once("-run:") else { return Ok(false) };

  if !matches!(kind, "search" | "browse")
    || !matches!(source, "search" | "browse")
    || id.is_empty()
    || !id.bytes().all(|byte| byte.is_ascii_digit())
  {
    return Ok(false);
  }

  let Ok(id) = id.parse::<i64>() else { return Ok(false) };
  let Some(run) = job.run(id).await? else { return Ok(true) };

  Ok(run.finished_at.is_some() && !running(run.unit).await)
}

/// What research runs left behind when their process died before they could give it back: only a run that
/// is closed in bmo-job's ledger and whose unit no longer runs is reclaimed; how many places came back.
pub async fn reclaim_leases<F, Fut>(agency: &dyn Agency, job: &Job, running: F) -> Result<i64, String>
where
  F: Fn(String) -> Fut,
  Fut: Future<Output = bool>,
{
  let said = |fault: crate::clients::agency::AgencyFault| fault.to_string();
  let mut reclaimed = 0;

  for held in agency.reservations(None).await.map_err(said)? {
    if ended(job, &held.asking.owner, &held.asking.source, &running).await?
      && agency
        .release(&held.token, "research run ended without giving its seats back")
        .await
        .map_err(said)?
    {
      reclaimed += 1;
    }
  }

  for request in agency.requests(None).await.map_err(said)? {
    if request.asking.reservation.is_some()
      || !ended(job, &request.asking.owner, &request.asking.source, &running).await?
    {
      continue;
    }

    if agency
      .finish(
        &request.token,
        "research run ended without giving its place back",
        request.state == "waiting",
      )
      .await
      .map_err(said)?
    {
      reclaimed += 1;
    }
  }

  Ok(reclaimed)
}

// ---------------------------------------------------------------- the knowledge it is handed

/// One piece of knowledge a piece of work is handed: where it is kept, and what it says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Piece {
  pub path: String,
  pub text: String,
}

/// The block a piece of work is handed, and the paths in it; no block where nothing was picked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Knowledge {
  pub block: Option<String>,
  pub paths: Vec<String>,
}

impl Knowledge {
  /// The pieces picked for a piece of work called `noun`, as its prompt carries them.
  pub fn of(noun: &str, pieces: &[Piece]) -> Self {
    Self {
      block: knowledge_block(noun, pieces),
      paths: pieces.iter().map(|piece| piece.path.clone()).collect(),
    }
  }
}

/// A quiet instruction as one of the lines a piece of work's knowledge is picked for.
pub fn instruction_line(text: &str) -> String {
  format!("quiet instruction from the bot: {text}")
}

/// The knowledge bmo-gate picks for a piece of work called `noun` in a room, for its own lines; none where
/// the gate picks nothing or cannot be asked, which costs the work its reference and nothing else.
pub async fn work_knowledge(gate: &Gate, chat_id: i64, lines: &[String], noun: &str) -> Knowledge {
  match gate.work_knowledge(chat_id, lines).await {
    Ok(pieces) => Knowledge::of(noun, &pieces),
    Err(why) => {
      eprintln!("chat {chat_id}: the knowledge for a piece of work could not be picked: {why}");
      Knowledge::default()
    }
  }
}

/// The picked pieces in the shape a turn's reference stands in, so nothing in them can pass for the work's own
/// instructions, under one line saying whose they are and which way they cut.
pub fn knowledge_block(noun: &str, pieces: &[Piece]) -> Option<String> {
  if pieces.is_empty() {
    return None;
  }

  let mut lines = vec![format!(
    "Reference for this {noun} ({}): what the agent's memory holds on its matter — facts as they stand now and the rules every room keeps. Where a document or a comment says otherwise, this is what is true.",
    pieces.len()
  )];

  for piece in pieces {
    lines.push(format!("[{}]", piece.path));
    lines.push(piece.text.trim().to_owned());
  }

  lines.push("END OF REFERENCE".to_owned());

  Some(lines.join("\n"))
}
