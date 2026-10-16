//! A deeper run against the common co-worker pool (`docs/research-runs.md`, "A named depth, and a full
//! pool"): whether it goes as asked or as the shallowest, and the wait an explicit depth makes for its first
//! seat before its model starts. What a run holds and gives back is the research module's own.

use std::future::Future;
use std::time::Duration;

use super::reserve_places;
use crate::clients::agency::Agency;
use crate::control::research::child_mapping;
use crate::now_ms;

/// Whether the person insisted on the depth they asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Demand {
  Explicit,
  Optional,
}

impl Demand {
  /// The word the launcher hands the runner.
  pub fn word(self) -> &'static str {
    match self {
      Self::Explicit => "explicit",
      Self::Optional => "optional",
    }
  }
}

/// A demand as a caller wrote it: absent is optional, since only a caller that says the person insisted
/// gets to wait; anything that is neither word is none.
pub fn parse_demand(value: Option<&str>) -> Option<Demand> {
  match value.unwrap_or_default() {
    "" | "optional" => Some(Demand::Optional),
    "explicit" => Some(Demand::Explicit),
    _ => None,
  }
}

/// How many places a role's tier has on the branch provider right now, those already waiting counted, and
/// why there are none where there are none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolRoom {
  pub free: i64,
  pub reason: Option<String>,
}

impl PoolRoom {
  fn none(reason: String) -> Self {
    Self {
      free: 0,
      reason: Some(reason),
    }
  }
}

/// The room a role's branches would find in the pool now.
pub async fn pool_room(agency: &dyn Agency, role: &str) -> PoolRoom {
  let mapping = match child_mapping(agency, role).await {
    Ok(mapping) => mapping,
    Err(why) => return PoolRoom::none(format!("the {role} role cannot be resolved: {why}")),
  };
  let roles = agency.roles().await.unwrap_or_default();

  if !roles.iter().any(|held| held.name == role && held.enabled && held.pooled) {
    return PoolRoom::none(format!("the {role} role is not an enabled pooled role"));
  }

  let status = agency.status().await.unwrap_or_default();
  let Some(place) = status
    .iter()
    .find(|one| one.provider == mapping.provider && one.tier == mapping.tier)
    .filter(|place| place.capacity > 0)
  else {
    return PoolRoom::none(format!(
      "no pool capacity is configured for {}/{}",
      mapping.provider, mapping.tier
    ));
  };
  let free = place.capacity - place.active - place.reserved - place.waiting;

  if free > 0 {
    PoolRoom { free, reason: None }
  } else {
    PoolRoom::none(format!(
      "the {} pool on {} is full ({} of {} in use, {} reserved, {} waiting)",
      mapping.tier, mapping.provider, place.active, place.capacity, place.reserved, place.waiting
    ))
  }
}

/// The depth a run goes at: the one asked, or the shallowest with why it was lowered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepthAdmission {
  pub depth: String,
  pub fallback: Option<String>,
}

/// An explicit request or one with no branches goes as asked. An optional one goes as asked only where the
/// pool has a place, and otherwise as `shallowest`: a depth nobody insisted on is not worth waiting for, and
/// the shallow answer is still an answer.
pub async fn admit_depth<F: Future<Output = PoolRoom>>(
  depth: &str,
  shallowest: &str,
  demand: Demand,
  max_children: i64,
  room: impl FnOnce() -> F,
) -> DepthAdmission {
  let asked = DepthAdmission {
    depth: depth.to_owned(),
    fallback: None,
  };

  if max_children == 0 || demand == Demand::Explicit {
    return asked;
  }

  let room = room().await;

  if room.free > 0 {
    return asked;
  }

  let why = room.reason.unwrap_or_else(|| "the pool has no place".to_owned());

  DepthAdmission {
    depth: shallowest.to_owned(),
    fallback: Some(format!("an optional {depth} search ran as {shallowest}: {why}")),
  }
}

/// What a run's branches are held as in the pool: its owner, its source, the role and the room it works for,
/// and how many.
pub struct Branches<'a> {
  pub owner: &'a str,
  pub source: &'a str,
  pub role: &'a str,
  pub count: i64,
}

/// How an explicit request's wait for its first seat came out: the reservation where a seat was let in, how
/// long it waited, and why it holds none.
#[derive(Debug, PartialEq, Eq)]
pub struct Place {
  pub reservation: Option<String>,
  pub waited_ms: i64,
  pub reason: Option<String>,
}

/// How often a waiting run asks the pool again.
pub const PLACE_POLL: Duration = Duration::from_millis(500);

/// An explicit request's wait for the first seat of its branches' reservation, before its model starts. A
/// wait that runs out, or that `cancelled` calls off, gives the reservation back, so it never blocks the line.
pub async fn await_place(
  agency: &dyn Agency,
  branches: &Branches<'_>,
  wait_limit_ms: i64,
  poll: Duration,
  cancelled: &(dyn Fn() -> bool + Send + Sync),
) -> Place {
  let started = now_ms();
  let token = match reserve_places(agency, branches.owner, branches.source, branches.role, branches.count).await {
    Ok(held) => held.token,
    Err(reason) => {
      return Place {
        reservation: None,
        waited_ms: 0,
        reason: Some(reason),
      };
    }
  };

  loop {
    if let Err(why) = agency.promote().await {
      eprintln!("the pool's reservations could not be promoted: {why}");
    }

    let seats = agency.seats(&token).await.unwrap_or_default();

    if seats.iter().any(|seat| seat.state == "reserved") {
      return Place {
        reservation: Some(token),
        waited_ms: now_ms() - started,
        reason: None,
      };
    }

    let waited = now_ms() - started;
    let stop = cancelled();

    if stop || waited >= wait_limit_ms {
      let reason = if stop {
        "cancelled while waiting for a pool place".to_owned()
      } else {
        format!("the pool stayed full for {} seconds", (wait_limit_ms as f64 / 1000.0).round())
      };

      if let Err(why) = agency.release(&token, &reason).await {
        eprintln!("the reservation could not be given back: {why}");
      }

      return Place {
        reservation: None,
        waited_ms: waited,
        reason: Some(reason),
      };
    }

    tokio::time::sleep(poll).await;
  }
}

/// Whether the machine's systemd holds a unit active, as the reclaim asks of a research run's unit.
pub async fn unit_running(unit: String) -> bool {
  tokio::process::Command::new("systemctl")
    .args(["--user", "is-active", "--quiet", &format!("{unit}.service")])
    .stdin(std::process::Stdio::null())
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .status()
    .await
    .is_ok_and(|status| status.success())
}
