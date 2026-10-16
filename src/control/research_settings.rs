//! What the agents screen reads and asks for about the common co-worker pool, the research profiles and
//! the search engines (`docs/api.md`, "Research settings (HTTP)"): every change is one of the actions
//! here, checked before it is written. The pool, the providers and the tiers are bmo-agency's; the
//! engines are bmo-explorer's, read through the tool host.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::{Map, Value, json};

use super::research::{self, FieldFault, Profile, Values, child_mapping, coworker_providers, field_faults, profiles};
use crate::Refusal;
use crate::clients::agency::{Agency, Ping, Place, Role, RouteAsked, Tier};
use crate::clients::tool_host::SearchEngines;
use crate::now_ms;
use crate::plumbing::http::{Call, Refused, Route, Standing, answer};
use crate::settings::Research;

// How long the engines' read is waited for before the screen is answered without them.
const ENGINES_WAIT: Duration = Duration::from_secs(30);

fn bad(code: &str, error: impl Into<String>) -> Refused {
  Refused::new(StatusCode::BAD_REQUEST, code, error)
}

fn agency_failed(error: String) -> Refused {
  Refused::new(StatusCode::BAD_GATEWAY, "agency_failed", error)
}

/// A ping as the screen reads one.
fn ping_view(ping: &Ping) -> Value {
  json!({ "place": ping.place, "at": ping.at, "outcome": ping.outcome, "answeredAt": ping.answered_at })
}

/// The latest of some pings, where there is one.
fn latest(pings: &[Ping]) -> Option<Ping> {
  pings.iter().max_by_key(|one| one.at).cloned()
}

/// Each pool role with the tier its work key is assigned.
struct RoleView {
  role: Role,
  tier: Option<String>,
  view: Value,
}

async fn role_views(agency: &dyn Agency, profiles: &[Profile]) -> Result<Vec<RoleView>, Refused> {
  let assignments = agency.assignments().await?;
  let mut views = Vec::new();

  for role in agency.roles().await? {
    let tier = assignments
      .iter()
      .find(|row| row.key == format!("role:{}", role.name))
      .map(|row| row.tier.clone());
    let used_by: Vec<&str> = profiles
      .iter()
      .filter(|profile| {
        super::whole(&profile.values.max_children).unwrap_or(0) > 0
          && profile.values.child_role.as_str() == Some(&role.name)
      })
      .map(|profile| profile.key.as_str())
      .collect();
    let view = json!({
      "name": role.name,
      "description": role.description,
      "enabled": role.enabled,
      "pooled": role.pooled,
      "control": role.control,
      "tier": tier,
      "usedBy": used_by,
    });

    views.push(RoleView { role, tier, view });
  }

  Ok(views)
}

async fn tier_views(
  agency: &dyn Agency,
  roles: &[RoleView],
  tiers: &[Tier],
  status: &[Place],
) -> Result<Vec<Value>, Refused> {
  let providers = agency.providers().await?;
  let lending = coworker_providers(agency).await.map_err(agency_failed)?;
  let held = agency.reservations(None).await?;
  let mut views = Vec::new();

  for provider in &lending {
    let label = providers
      .iter()
      .find(|one| one.name == *provider)
      .map(|one| one.label.clone())
      .unwrap_or_default();

    for tier in tiers.iter().filter(|one| one.coworker).map(|one| one.name.as_str()) {
      let Some(place) = status.iter().find(|one| one.provider == *provider && one.tier == tier) else {
        continue;
      };
      let asked = RouteAsked {
        work_key: None,
        provider: Some(provider.clone()),
        tier: tier.to_owned(),
      };
      let (mapping, error) = match agency.resolve(&asked).await? {
        Some(mapping) => (Some(mapping), None),
        None => (None, Some(format!("No route serves {tier} on {provider}"))),
      };
      let mut reservations = Vec::new();
      let mut last_pings = Vec::new();

      for reservation in held
        .iter()
        .filter(|one| one.asking.provider == *provider && one.asking.tier == tier)
      {
        let seats = agency.seats(&reservation.token).await?;
        let last = latest(&agency.pings(&reservation.token).await?);

        if let Some(ping) = &last {
          last_pings.push(ping.clone());
        }

        reservations.push(json!({
          "owner": reservation.asking.owner,
          "source": reservation.asking.source,
          "role": reservation.asking.role,
          "wanted": reservation.asking.wanted,
          "held": seats.iter().filter(|seat| seat.state == "reserved").count(),
          "waiting": seats.iter().filter(|seat| seat.state == "waiting").count(),
          "createdAt": reservation.created_at,
          "lastPing": last.as_ref().map(ping_view),
        }));
      }

      let serving: Vec<&str> = roles
        .iter()
        .filter(|one| one.role.enabled && !one.role.control && one.tier.as_deref() == Some(tier))
        .map(|one| one.role.name.as_str())
        .collect();
      let snapshot_models: Vec<Value> = place
        .snapshot_models
        .iter()
        .map(|one| json!({ "model": one.model, "effort": one.effort }))
        .collect();

      views.push(json!({
        "provider": provider,
        "label": label,
        "tier": tier,
        "model": mapping.as_ref().map(|one| one.model.clone()),
        "effort": mapping.as_ref().and_then(|one| one.effort.clone()),
        "error": error,
        "capacity": place.capacity,
        "defaultCapacity": place.default_capacity,
        "snapshotCapacities": place.snapshot_capacities,
        "snapshotModels": snapshot_models,
        "active": place.active,
        "reserved": place.reserved,
        "waiting": place.waiting,
        "yielding": { "active": place.yielding_active, "waiting": place.yielding_waiting },
        "roles": serving,
        "reservations": reservations,
        "lastPing": latest(&last_pings).as_ref().map(ping_view),
      }));
    }
  }

  Ok(views)
}

/// Where a new optional request for a role would stand right now: the same count the runner's
/// admission reads, so the screen and the launch never disagree.
async fn child_room(agency: &dyn Agency, roles: &[RoleView], status: &[Place], role: &str) -> Value {
  let room = |state: &str, provider: Value, tier: Value, model: Value, capacity: Value, counts: [i64; 3], error: Value| {
    json!({
      "state": state,
      "provider": provider,
      "tier": tier,
      "model": model,
      "capacity": capacity,
      "active": counts[0],
      "waiting": counts[1],
      "free": counts[2],
      "error": error,
    })
  };

  if !roles
    .iter()
    .any(|one| one.role.name == role && one.role.enabled && one.role.pooled)
  {
    return room("role-off", Value::Null, Value::Null, Value::Null, Value::Null, [0; 3], Value::Null);
  }

  let mapping = match child_mapping(agency, role).await {
    Ok(mapping) => mapping,
    Err(error) => {
      return room("unresolved", Value::Null, Value::Null, Value::Null, Value::Null, [0; 3], json!(error));
    }
  };
  let place = status
    .iter()
    .find(|one| one.provider == mapping.provider && one.tier == mapping.tier);
  let (provider, tier, model) = (json!(mapping.provider), json!(mapping.tier), json!(mapping.model));

  match place {
    Some(place) if place.capacity != 0 => {
      let free = (place.capacity - place.active - place.waiting).max(0);

      room(
        if free > 0 { "free" } else { "full" },
        provider,
        tier,
        model,
        json!(place.capacity),
        [place.active, place.waiting, free],
        Value::Null,
      )
    }
    place => room(
      "no-capacity",
      provider,
      tier,
      model,
      json!(place.map(|one| one.capacity)),
      [place.map_or(0, |one| one.active), place.map_or(0, |one| one.waiting), 0],
      Value::Null,
    ),
  }
}

async fn profile_views(
  agency: &dyn Agency,
  research: &Research,
  profiles: &[Profile],
  roles: &[RoleView],
  status: &[Place],
) -> Result<Vec<Value>, Refused> {
  let assignments = agency.assignments().await?;
  let mut views = Vec::new();

  for profile in profiles {
    let resolved = research::resolved(agency, research, &profile.key).await;
    let children = super::whole(&profile.values.max_children).unwrap_or(0);
    let mut view = Map::new();

    view.insert("key".into(), json!(profile.key));
    view.extend(profile.values.fields());
    view.insert("setAt".into(), json!(profile.set_at));
    view.insert(
      "seed".into(),
      research::seed(research, &profile.key).map_or(Value::Null, |seed| Value::Object(seed.fields())),
    );
    view.insert(
      "strategies".into(),
      json!(research.key(&profile.key).map(|one| one.strategies.clone()).unwrap_or_default()),
    );
    view.insert(
      "tier".into(),
      json!(assignments.iter().find(|row| row.key == profile.key).map(|row| row.tier.clone())),
    );
    view.insert("provider".into(), json!(resolved.provider));
    view.insert("model".into(), json!(resolved.mapping.as_ref().map(|one| one.model.clone())));
    view.insert("effort".into(), json!(resolved.mapping.as_ref().and_then(|one| one.effort.clone())));
    view.insert("error".into(), json!(resolved.error));
    view.insert(
      "room".into(),
      if children > 0 {
        child_room(agency, roles, status, profile.values.child_role.as_str().unwrap_or_default()).await
      } else {
        Value::Null
      },
    );
    views.push(Value::Object(view));
  }

  Ok(views)
}

/// The engines and their rests as the screen reads them, or why bmo-explorer could not be read.
async fn engine_views(standing: &Standing, now: i64) -> (Vec<Value>, Vec<Value>, Option<String>) {
  let read: Result<SearchEngines, String> = tokio::time::timeout(ENGINES_WAIT, standing.tool_host.search_engines())
    .await
    .unwrap_or_else(|_| Err(format!("bmo-explorer did not answer within {} seconds", ENGINES_WAIT.as_secs())));

  match read {
    Ok(read) => {
      let engines = read
        .engines
        .iter()
        .map(|engine| {
          json!({
            "engine": engine.name,
            "state": engine.state,
            "reason": engine.reason,
            "observedAt": engine.observed_at,
            "suspendedUntil": engine.suspended_until,
            "elapsedMs": engine.elapsed_ms,
            "resultCount": engine.result_count,
            "category": engine.category,
            "korean": engine.korean_native,
            // A suspension whose time has passed is not a rest any more, whatever the engine still says.
            "resting": engine.state == "suspended" && engine.suspended_until.is_some_and(|until| until > now),
          })
        })
        .collect();
      let cooldowns = read
        .cooldowns
        .iter()
        .map(|one| json!({ "reason": one.reason, "ms": one.ms }))
        .collect();

      (engines, cooldowns, None)
    }
    Err(error) => (Vec::new(), Vec::new(), Some(error)),
  }
}

/// The limits each field of a profile is held to, as the form reads them.
fn limits(research: &Research) -> Value {
  let limits = &research.limits;

  json!({
    "capMs": { "min": limits.cap_ms.min, "max": limits.cap_ms.max },
    "synthesisReserveMs": { "min": limits.synthesis_reserve_ms.min },
    "researchMs": { "min": limits.research_ms.min },
    "maxChildren": { "max": limits.max_children.max },
    "waitLimitMs": { "max": limits.wait_limit_ms.max },
    "intervalMs": { "max": limits.interval_ms.max },
  })
}

/// `GET /research-settings`: the common co-worker pool, the research profiles and the engines.
pub struct ResearchSettings;

impl Route for ResearchSettings {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/research-settings";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let standing = call.standing();
    let research = &standing.settings.research;
    let agency = standing.agency.as_ref();
    let db = standing.archive.db().map_err(Refusal::from)?;
    let now = now_ms();
    let profiles = profiles(db, research).await?;
    let roles = role_views(agency, &profiles).await?;
    let tiers = agency.tiers().await?;
    let status = agency.status().await?;
    let coworker_tiers: Vec<&str> = tiers.iter().filter(|one| one.coworker).map(|one| one.name.as_str()).collect();
    let defaults: Map<String, Value> = tiers
      .iter()
      .filter(|one| one.coworker)
      .map(|one| (one.name.clone(), json!(one.default_capacity)))
      .collect();
    let (engines, cooldowns, engines_error) = engine_views(standing, now).await;
    let mut body = json!({
      "readAt": now,
      "pool": {
        "roles": roles.iter().map(|one| one.view.clone()).collect::<Vec<_>>(),
        "tiers": tier_views(agency, &roles, &tiers, &status).await?,
        "capacityLimit": research.pool_capacity_most,
        "defaults": defaults,
        "coworkerTiers": coworker_tiers,
      },
      "profiles": profile_views(agency, research, &profiles, &roles, &status).await?,
      "limits": limits(research),
      "engines": engines,
      "cooldowns": cooldowns,
    });

    if let Some(error) = engines_error {
      body["enginesError"] = json!(error);
    }

    Ok(answer(StatusCode::OK, body))
  }
}

crate::route!(ResearchSettings);

/// The refusal of a profile whose fields fail their checks: 422, each fault named.
fn invalid_profile(faults: &[FieldFault]) -> Refused {
  let error = faults.iter().map(|fault| fault.error.as_str()).collect::<Vec<_>>().join(" ");

  Refused::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_profile", error).with("faults", faults)
}

async fn save_profile(standing: &Standing, key: &str, values: &Values, now: i64) -> Result<(), Refused> {
  let research = &standing.settings.research;
  let db = standing.archive.db().map_err(Refusal::from)?;
  let pooled: Vec<String> = standing
    .agency
    .roles()
    .await?
    .into_iter()
    .filter(|role| role.enabled && role.pooled)
    .map(|role| role.name)
    .collect();
  let faults = field_faults(research, key, values, &pooled);

  if !faults.is_empty() {
    return Err(invalid_profile(&faults));
  }

  research::set_profile(db, research, key, values, &pooled, now)
    .await?
    .map_err(|fault| Refused::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_profile", fault).with("faults", json!([])))
}

fn research_key<'b>(research: &Research, body: &'b Value) -> Result<&'b str, Refused> {
  body
    .get("key")
    .and_then(Value::as_str)
    .filter(|key| research.key(key).is_some())
    .ok_or_else(|| bad("bad_key", "choose a research key the settings hold"))
}

/// `POST /research-settings`: one change to the pool's limits or a research profile.
pub struct ResearchChange;

impl Route for ResearchChange {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/research-settings";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let body = call.body().await?;
    let standing = call.standing();
    let research = &standing.settings.research;
    let agency = standing.agency.as_ref();
    let now = now_ms();
    let no_action = || {
      bad(
        "bad_action",
        "name an action: set-pool-limit, reset-pool-limits, set-profile or reset-profile",
      )
    };

    if !body.is_object() {
      return Err(no_action());
    }

    match body.get("action").and_then(Value::as_str) {
      Some("set-pool-limit") => {
        let lending = coworker_providers(agency).await.map_err(agency_failed)?;
        let coworker_tiers: Vec<String> = agency
          .tiers()
          .await?
          .into_iter()
          .filter(|one| one.coworker)
          .map(|one| one.name)
          .collect();
        let provider = body.get("provider").and_then(Value::as_str);
        let tier = body.get("tier").and_then(Value::as_str);
        let (Some(provider), Some(tier)) = (
          provider.filter(|one| lending.iter().any(|name| name == one)),
          tier.filter(|one| coworker_tiers.iter().any(|name| name == one)),
        ) else {
          return Err(bad(
            "bad_pool_limit",
            format!(
              "choose an enabled provider that lends co-workers and a co-worker tier ({})",
              coworker_tiers.join(", ")
            ),
          ));
        };
        let Some(capacity) = body
          .get("capacity")
          .and_then(super::whole)
          .filter(|count| (0..=research.pool_capacity_most).contains(count))
        else {
          return Err(bad(
            "bad_capacity",
            format!("a limit is a whole number from 0 to {}", research.pool_capacity_most),
          ));
        };

        agency.set_limit(provider, tier, capacity).await?;
      }
      Some("reset-pool-limits") => agency.reset_limits().await?,
      Some("set-profile") => {
        let key = research_key(research, &body)?;
        let Some(raw) = body.get("values").filter(|values| values.is_object() || values.is_array()) else {
          return Err(bad("no_values", "set-profile carries no values"));
        };

        save_profile(standing, key, &Values::of(raw), now).await?;
      }
      Some("reset-profile") => {
        let key = research_key(research, &body)?;
        let seed = research::seed(research, key).ok_or_else(|| bad("bad_key", format!("{key} has no seed")))?;

        save_profile(standing, key, &seed, now).await?;
      }
      _ => return Err(no_action()),
    }

    Ok(answer(StatusCode::OK, json!({ "ok": true })))
  }
}

crate::route!(ResearchChange);
