//! The research profiles (`docs/research-runs.md`, "Research profiles"): one row a key, the checks each
//! field must pass — the form's, which name the field, and the store's own — and what a key's run
//! launches with right now, as bmo-agency resolves it.

use sea_orm::{DatabaseConnection, QueryResult};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::clients::agency::{Agency, Mapping, RouteAsked};
use crate::settings::{Research, Seed};
use crate::{Refusal, all, run};

const MINUTE: i64 = 60_000;
const SECOND: i64 = 1_000;

/// One profile's values, each as the request or the row carries it: a value of the wrong kind is kept
/// as it came, for the checks to name.
#[derive(Clone, Debug, PartialEq)]
pub struct Values {
  pub cap_ms: Value,
  pub synthesis_reserve_ms: Value,
  pub child_role: Value,
  pub max_children: Value,
  pub wait_limit_ms: Value,
  pub strategy: Value,
  pub host_interval_ms: Value,
  pub engine_interval_ms: Value,
}

impl Values {
  /// The eight fields out of a request body, and nothing else. A field left out is no value at all,
  /// which every check refuses as it refuses a value of the wrong kind, and which a clockless key's
  /// clock fields do not read as empty.
  pub fn of(raw: &Value) -> Self {
    let field = |name: &str| raw.get(name).cloned().unwrap_or(Value::Bool(false));

    Self {
      cap_ms: field("capMs"),
      synthesis_reserve_ms: field("synthesisReserveMs"),
      child_role: field("childRole"),
      max_children: field("maxChildren"),
      wait_limit_ms: field("waitLimitMs"),
      strategy: field("strategy"),
      host_interval_ms: field("hostIntervalMs"),
      engine_interval_ms: field("engineIntervalMs"),
    }
  }

  /// What a key starts on.
  pub fn seeded(seed: &Seed) -> Self {
    Self {
      cap_ms: json!(seed.cap_ms),
      synthesis_reserve_ms: json!(seed.synthesis_reserve_ms),
      child_role: json!(seed.child_role),
      max_children: json!(seed.max_children),
      wait_limit_ms: json!(seed.wait_limit_ms),
      strategy: json!(seed.strategy),
      host_interval_ms: json!(seed.host_interval_ms),
      engine_interval_ms: json!(seed.engine_interval_ms),
    }
  }

  /// The values as a view carries them.
  pub fn fields(&self) -> Map<String, Value> {
    let mut fields = Map::new();

    fields.insert("capMs".into(), self.cap_ms.clone());
    fields.insert("synthesisReserveMs".into(), self.synthesis_reserve_ms.clone());
    fields.insert("childRole".into(), self.child_role.clone());
    fields.insert("maxChildren".into(), self.max_children.clone());
    fields.insert("waitLimitMs".into(), self.wait_limit_ms.clone());
    fields.insert("strategy".into(), self.strategy.clone());
    fields.insert("hostIntervalMs".into(), self.host_interval_ms.clone());
    fields.insert("engineIntervalMs".into(), self.engine_interval_ms.clone());

    fields
  }
}

/// One key's profile: its row, or its seed set at no time where it has none yet.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
  pub key: String,
  pub values: Values,
  pub set_at: i64,
}

/// One field that is wrong: the value's name, a snake_case code, and the sentence the form shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldFault {
  pub field: &'static str,
  pub code: &'static str,
  pub error: String,
}

impl FieldFault {
  fn new(field: &'static str, code: &'static str, error: impl Into<String>) -> Self {
    Self {
      field,
      code,
      error: error.into(),
    }
  }
}

/// A number that is whole and within its bounds, as the checks read one.
fn whole(value: &Value, min: i64, max: i64) -> Option<i64> {
  super::whole(value).filter(|number| value.is_number() && (min..=max).contains(number))
}

fn role_shape(name: &str) -> bool {
  let mut chars = name.chars();

  chars.next().is_some_and(|first| first.is_ascii_lowercase())
    && name.len() <= 64
    && chars.all(|one| one.is_ascii_lowercase() || one.is_ascii_digit() || one == '_' || one == '-')
}

fn counted(count: i64, unit: &str) -> String {
  if count == 1 { format!("1 {unit}") } else { format!("{count} {unit}s") }
}

/// A length of time as the form says one: whole minutes where it divides, seconds otherwise.
pub fn spoken(ms: i64) -> String {
  if ms % MINUTE == 0 {
    counted(ms / MINUTE, "minute")
  } else if ms % SECOND == 0 {
    counted(ms / SECOND, "second")
  } else {
    format!("{} seconds", ms as f64 / 1000.0)
  }
}

/// Every field that is wrong, in the order the form shows them; empty where the profile holds. A key the
/// settings do not hold is refused before it reaches here, and reads as one with no clock, no branch
/// limit of its own and no strategy.
pub fn field_faults(research: &Research, key: &str, values: &Values, pooled_roles: &[String]) -> Vec<FieldFault> {
  let limits = &research.limits;
  let declared = research.key(key);
  let mut faults = Vec::new();
  let (cap_min, cap_max) = (limits.cap_ms.least(), limits.cap_ms.most());

  if !declared.is_some_and(|one| one.clocked) {
    if !values.cap_ms.is_null() {
      faults.push(FieldFault::new("capMs", "no_clock", "This run holds no total time."));
    }

    if !values.synthesis_reserve_ms.is_null() {
      faults.push(FieldFault::new("synthesisReserveMs", "no_clock", "This run holds no wrap-up time."));
    }
  } else {
    let cap = whole(&values.cap_ms, cap_min, cap_max);
    let reserve_min = limits.synthesis_reserve_ms.least();
    let research_min = limits.research_ms.least();

    if cap.is_none() {
      faults.push(FieldFault::new(
        "capMs",
        "cap_out_of_range",
        format!("The total time runs from {} to {}.", spoken(cap_min), spoken(cap_max)),
      ));
    }

    match whole(&values.synthesis_reserve_ms, reserve_min, cap_max) {
      None => faults.push(FieldFault::new(
        "synthesisReserveMs",
        "reserve_too_short",
        format!("The wrap-up time must be at least {}.", spoken(reserve_min)),
      )),
      Some(reserve) if cap.is_some_and(|cap| cap - reserve < research_min) => faults.push(FieldFault::new(
        "synthesisReserveMs",
        "research_too_short",
        format!("The wrap-up time must leave at least {} of research.", spoken(research_min)),
      )),
      Some(_) => {}
    }
  }

  let children_max = limits.max_children.most();

  match whole(&values.max_children, 0, children_max) {
    None => faults.push(FieldFault::new(
      "maxChildren",
      "children_out_of_range",
      format!("The branch count is a whole number from 0 to {children_max}."),
    )),
    Some(children) if declared.is_some_and(|one| one.branchless) && children != 0 => {
      faults.push(FieldFault::new("maxChildren", "branchless", "This run opens no branches."));
    }
    Some(_) => {}
  }

  match values.child_role.as_str().filter(|role| role_shape(role)) {
    None => faults.push(FieldFault::new("childRole", "bad_role", "The branch role is not a role name.")),
    Some(role)
      if whole(&values.max_children, 1, children_max).is_some() && !pooled_roles.iter().any(|one| one == role) =>
    {
      faults.push(FieldFault::new(
        "childRole",
        "role_not_pooled",
        format!("{role} is not a role enabled in the pool."),
      ));
    }
    Some(_) => {}
  }

  let wait_max = limits.wait_limit_ms.most();

  if whole(&values.wait_limit_ms, 0, wait_max).is_none() {
    faults.push(FieldFault::new(
      "waitLimitMs",
      "wait_out_of_range",
      format!("The wait limit runs from 0 to {}.", spoken(wait_max)),
    ));
  }

  if !values
    .strategy
    .as_str()
    .is_some_and(|strategy| declared.is_some_and(|one| one.strategies.iter().any(|accepted| accepted == strategy)))
  {
    faults.push(FieldFault::new("strategy", "bad_strategy", "This run does not accept that fetch strategy."));
  }

  let interval_max = limits.interval_ms.most();

  if whole(&values.host_interval_ms, 0, interval_max).is_none() {
    faults.push(FieldFault::new(
      "hostIntervalMs",
      "interval_out_of_range",
      format!("The host interval runs from 0 to {}.", spoken(interval_max)),
    ));
  }

  if whole(&values.engine_interval_ms, 0, interval_max).is_none() {
    faults.push(FieldFault::new(
      "engineIntervalMs",
      "interval_out_of_range",
      format!("The engine interval runs from 0 to {}.", spoken(interval_max)),
    ));
  }

  faults
}

fn whole_fault(value: &Value, what: &str, min: i64, max: i64) -> Option<String> {
  whole(value, min, max)
    .is_none()
    .then(|| format!("{what} must be a whole number from {min} to {max}"))
}

/// Everything a profile must satisfy on its own and against the pool's roles, as the store refuses it;
/// nothing where it holds.
pub fn profile_fault(research: &Research, key: &str, values: &Values, pooled_roles: &[String]) -> Option<String> {
  let Some(declared) = research.key(key) else {
    return Some(format!("{key} is not a research key"));
  };
  let limits = &research.limits;
  let (cap_min, cap_max) = (limits.cap_ms.least(), limits.cap_ms.most());
  let clock = if declared.clocked {
    whole_fault(&values.cap_ms, "capMs", cap_min, cap_max).or_else(|| {
      whole_fault(
        &values.synthesis_reserve_ms,
        "synthesisReserveMs",
        limits.synthesis_reserve_ms.least(),
        cap_max,
      )
    })
  } else if !values.cap_ms.is_null() || !values.synthesis_reserve_ms.is_null() {
    Some(format!("{key} has no clock, so capMs and synthesisReserveMs must be null"))
  } else {
    None
  };
  let fault = clock
    .or_else(|| whole_fault(&values.max_children, "maxChildren", 0, limits.max_children.most()))
    .or_else(|| whole_fault(&values.wait_limit_ms, "waitLimitMs", 0, limits.wait_limit_ms.most()))
    .or_else(|| whole_fault(&values.host_interval_ms, "hostIntervalMs", 0, limits.interval_ms.most()))
    .or_else(|| whole_fault(&values.engine_interval_ms, "engineIntervalMs", 0, limits.interval_ms.most()));

  if fault.is_some() {
    return fault;
  }

  let research_min = limits.research_ms.least();

  if declared.clocked
    && super::whole(&values.cap_ms).unwrap_or(0) - super::whole(&values.synthesis_reserve_ms).unwrap_or(0)
      < research_min
  {
    return Some(format!(
      "synthesisReserveMs must leave at least {} seconds of research inside capMs",
      research_min / SECOND
    ));
  }

  if !values
    .strategy
    .as_str()
    .is_some_and(|strategy| declared.strategies.iter().any(|one| one == strategy))
  {
    return Some(format!("strategy must be one of {}", declared.strategies.join(", ")));
  }

  let children = super::whole(&values.max_children).unwrap_or(0);

  if declared.branchless && children != 0 {
    return Some(format!("{key} opens no branches"));
  }

  let Some(role) = values.child_role.as_str().filter(|role| role_shape(role)) else {
    return Some("childRole is not a role name".into());
  };

  (children > 0 && !pooled_roles.iter().any(|one| one == role))
    .then(|| format!("childRole {role} is not an enabled pooled role"))
}

fn profile_of(research: &Research, row: &QueryResult) -> Result<Profile, Refusal> {
  let key: String = row.try_get("", "key")?;
  let clocked = research.key(&key).is_some_and(|one| one.clocked);
  // Whether a kind of run is held to a clock is the key's own declaration, never the row's.
  let clock = |column: &str| -> Result<Value, Refusal> {
    Ok(if clocked { json!(row.try_get::<Option<i64>>("", column)?) } else { Value::Null })
  };

  Ok(Profile {
    values: Values {
      cap_ms: clock("cap_ms")?,
      synthesis_reserve_ms: clock("synthesis_reserve_ms")?,
      child_role: json!(row.try_get::<String>("", "child_role")?),
      max_children: json!(row.try_get::<i64>("", "max_children")?),
      wait_limit_ms: json!(row.try_get::<i64>("", "wait_limit_ms")?),
      strategy: json!(row.try_get::<String>("", "strategy")?),
      host_interval_ms: json!(row.try_get::<i64>("", "host_interval_ms")?),
      engine_interval_ms: json!(row.try_get::<i64>("", "engine_interval_ms")?),
    },
    set_at: row.try_get("", "set_at")?,
    key,
  })
}

/// Every key's profile, in the settings' order; a key with no row yet reads as its seed, set at no time.
pub async fn profiles(db: &DatabaseConnection, research: &Research) -> Result<Vec<Profile>, Refusal> {
  let rows = all(db, "SELECT * FROM research_profiles", vec![]).await?;
  let mut read = Vec::new();

  for declared in &research.keys {
    let mut found = None;

    for row in &rows {
      if row.try_get::<String>("", "key")? == declared.key {
        found = Some(profile_of(research, row)?);
      }
    }

    read.push(found.unwrap_or_else(|| Profile {
      key: declared.key.clone(),
      values: Values::seeded(&declared.seed),
      set_at: 0,
    }));
  }

  Ok(read)
}

/// One key's profile, as `profiles` reads it; none for a key the settings do not hold.
pub async fn profile(db: &DatabaseConnection, research: &Research, key: &str) -> Result<Option<Profile>, Refusal> {
  Ok(profiles(db, research).await?.into_iter().find(|profile| profile.key == key))
}

/// A key's profile written, where it holds; otherwise the store's refusal.
pub async fn set_profile(
  db: &DatabaseConnection,
  research: &Research,
  key: &str,
  values: &Values,
  pooled_roles: &[String],
  now: i64,
) -> Result<Result<(), String>, Refusal> {
  if let Some(fault) = profile_fault(research, key, values, pooled_roles) {
    return Ok(Err(fault));
  }

  let number = |value: &Value| -> sea_orm::Value { super::whole(value).into() };

  run(
    db,
    r#"INSERT INTO research_profiles
         (key, cap_ms, synthesis_reserve_ms, effort, child_role, max_children, wait_limit_ms, strategy, host_interval_ms, engine_interval_ms, set_at)
       VALUES ($1, $2, $3, NULL, $4, $5, $6, $7, $8, $9, $10)
       ON CONFLICT (key) DO UPDATE SET cap_ms = excluded.cap_ms, synthesis_reserve_ms = excluded.synthesis_reserve_ms,
         effort = excluded.effort, child_role = excluded.child_role, max_children = excluded.max_children,
         wait_limit_ms = excluded.wait_limit_ms, strategy = excluded.strategy, host_interval_ms = excluded.host_interval_ms,
         engine_interval_ms = excluded.engine_interval_ms, set_at = excluded.set_at"#,
    vec![
      key.into(),
      number(&values.cap_ms),
      number(&values.synthesis_reserve_ms),
      values.child_role.as_str().unwrap_or_default().into(),
      number(&values.max_children),
      number(&values.wait_limit_ms),
      values.strategy.as_str().unwrap_or_default().into(),
      number(&values.host_interval_ms),
      number(&values.engine_interval_ms),
      now.into(),
    ],
  )
  .await?;

  Ok(Ok(()))
}

/// What a key starts on and goes back to when reset.
pub fn seed(research: &Research, key: &str) -> Option<Values> {
  research.key(key).map(|declared| Values::seeded(&declared.seed))
}

/// The provider a research run of a line starts on: the first enabled one, in bmo-agency's order, whose
/// program carries it.
pub async fn research_provider(agency: &dyn Agency, line: &str) -> Result<String, String> {
  agency
    .providers()
    .await
    .map_err(|fault| fault.to_string())?
    .into_iter()
    .find(|provider| provider.enabled && provider.carries(line))
    .map(|provider| provider.name)
    .ok_or_else(|| format!("No enabled provider carries the {line} line"))
}

/// The providers that lend co-workers and are enabled, in bmo-agency's order.
pub async fn coworker_providers(agency: &dyn Agency) -> Result<Vec<String>, String> {
  Ok(
    agency
      .providers()
      .await
      .map_err(|fault| fault.to_string())?
      .into_iter()
      .filter(|provider| provider.enabled && provider.coworkers)
      .map(|provider| provider.name)
      .collect(),
  )
}

/// The tier a work key is assigned.
pub async fn assigned_tier(agency: &dyn Agency, key: &str) -> Result<String, String> {
  agency
    .assignments()
    .await
    .map_err(|fault| fault.to_string())?
    .into_iter()
    .find(|assignment| assignment.key == key)
    .map(|assignment| assignment.tier)
    .ok_or_else(|| format!("Missing tier assignment: {key}"))
}

/// The route a work key's tier takes on a provider.
async fn resolve_key(agency: &dyn Agency, key: &str, provider: &str, tier: &str) -> Result<Mapping, String> {
  let asked = RouteAsked {
    work_key: Some(key.to_owned()),
    provider: Some(provider.to_owned()),
    tier: tier.to_owned(),
  };

  agency
    .resolve(&asked)
    .await
    .map_err(|fault| fault.to_string())?
    .ok_or_else(|| format!("No route serves {key} at {tier} on {provider}"))
}

/// What a branch of a role resolves to: the role's tier on the provider the search line starts on,
/// which must lend co-workers.
pub async fn child_mapping(agency: &dyn Agency, role: &str) -> Result<Mapping, String> {
  let provider = research_provider(agency, "search").await?;
  let providers = agency.providers().await.map_err(|fault| fault.to_string())?;
  let Some(lending) = providers.iter().find(|one| one.name == provider) else {
    return Err(format!("{provider}: is not a provider"));
  };

  if !lending.coworkers {
    return Err(format!("{provider}: lends no co-workers"));
  }

  if !lending.enabled {
    return Err(format!("{provider}: is not enabled"));
  }

  let key = format!("role:{role}");
  let tier = assigned_tier(agency, &key).await?;

  if !agency
    .tiers()
    .await
    .map_err(|fault| fault.to_string())?
    .iter()
    .any(|one| one.name == tier)
  {
    return Err(format!("Unknown model tier: {tier}"));
  }

  resolve_key(agency, &key, &provider, &tier).await
}

/// The effort a research run launches with, which is its mapping's: one that provider and that model
/// accept, as bmo-agency checks it, refused rather than lowered.
pub async fn checked_effort(agency: &dyn Agency, mapping: &Mapping) -> Result<String, String> {
  let Some(effort) = &mapping.effort else {
    return Err(format!("{}/{} has no effort for this research run", mapping.provider, mapping.model));
  };

  agency
    .check_effort(&mapping.provider, &mapping.model, effort)
    .await
    .map_err(|fault| fault.to_string())?;

  Ok(effort.clone())
}

/// What a key launches with right now, as the runner checks it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Resolved {
  /// The provider its line starts on.
  pub provider: Option<String>,
  /// That provider's mapping of the key's tier, where it holds.
  pub mapping: Option<Mapping>,
  /// Why it cannot launch.
  pub error: Option<String>,
}

/// What a research key launches with right now: the provider its line starts on, and that provider's
/// mapping of the key's tier, checked the way the runner checks it.
pub async fn resolved(agency: &dyn Agency, research: &Research, key: &str) -> Resolved {
  let Some(declared) = research.key(key) else {
    return Resolved {
      error: Some(format!("{key} is not a research key")),
      ..Resolved::default()
    };
  };
  let provider = match research_provider(agency, &declared.line).await {
    Ok(provider) => provider,
    Err(error) => {
      return Resolved {
        error: Some(error),
        ..Resolved::default()
      };
    }
  };
  let mapping = match launch_mapping(agency, key, &provider).await {
    Ok(mapping) => mapping,
    Err(error) => {
      return Resolved {
        provider: Some(provider),
        mapping: None,
        error: Some(error),
      };
    }
  };

  match checked_effort(agency, &mapping).await {
    Ok(_) => Resolved {
      provider: Some(provider),
      mapping: Some(mapping),
      error: None,
    },
    Err(error) => Resolved {
      provider: Some(provider),
      mapping: None,
      error: Some(error),
    },
  }
}

/// The mapping a key's own run launches with on a provider: its work key's tier there.
pub async fn launch_mapping(agency: &dyn Agency, key: &str, provider: &str) -> Result<Mapping, String> {
  let tier = assigned_tier(agency, key).await?;

  resolve_key(agency, key, provider, &tier).await
}
