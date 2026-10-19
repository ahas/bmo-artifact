//! The loops the server repeats on its own clock (`docs/layout.md`, "Plumbing"): each registered beside its
//! own code, as a route is, and started by the server once it listens. A loop's passes run one after another,
//! never beside each other; a pass that fails is said on the server's error stream and the loop goes on.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

use crate::plumbing::http::Standing;

pub use inventory;

/// One loop the server runs, registered beside its pass with `bmo_artifact::background!`.
pub trait Loop: Send + Sync + 'static {
  /// What it is called by in what the server says of it.
  const NAME: &'static str;
  /// How long from one pass's start to the next.
  const PERIOD: Duration;

  /// One pass at `now_ms`, on what the server stands on.
  fn pass(&self, standing: &Standing, now_ms: i64) -> impl Future<Output = Result<(), String>> + Send;
}

/// What one pass comes to.
pub type Passing<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// A loop as the server holds it among the others.
pub trait Repeats: Send + Sync {
  fn name(&self) -> &'static str;
  fn period(&self) -> Duration;
  fn pass<'a>(&'a self, standing: &'a Standing, now_ms: i64) -> Passing<'a>;
}

impl<L: Loop> Repeats for L {
  fn name(&self) -> &'static str {
    L::NAME
  }

  fn period(&self) -> Duration {
    L::PERIOD
  }

  fn pass<'a>(&'a self, standing: &'a Standing, now_ms: i64) -> Passing<'a> {
    Box::pin(Loop::pass(self, standing, now_ms))
  }
}

/// One registered loop.
pub struct RegisteredLoop(pub &'static dyn Repeats);

inventory::collect!(RegisteredLoop);

/// Registers a loop, a `static` of its own, with the server: `bmo_artifact::background!(NEWS_BRIEF_LOOK);`
#[macro_export]
macro_rules! background {
  ($loop:expr) => {
    $crate::plumbing::background::inventory::submit! { $crate::plumbing::background::RegisteredLoop(&$loop) }
  };
}

/// Every loop linked into this binary.
pub fn loops() -> impl Iterator<Item = &'static dyn Repeats> {
  inventory::iter::<RegisteredLoop>.into_iter().map(|entry| entry.0)
}

/// Each name two loops claim, as a sentence; none where every loop has its own.
pub fn loop_faults() -> Vec<String> {
  let mut claimed: BTreeMap<&str, usize> = BTreeMap::new();

  for one in loops() {
    *claimed.entry(one.name()).or_default() += 1;
  }

  claimed
    .into_iter()
    .filter(|(_, claims)| *claims > 1)
    .map(|(name, claims)| format!("{name}: claimed by {claims} loops"))
    .collect()
}

/// The loops running, each in a task of its own, until they are stopped.
pub struct Running(Vec<JoinHandle<()>>);

impl Running {
  /// Every loop stopped where it stands.
  pub fn stop(self) {
    for task in self.0 {
      task.abort();
    }
  }
}

/// Every registered loop started on what the server stands on, its first pass at once.
pub fn start(standing: Arc<Standing>) -> Running {
  Running(
    loops()
      .map(|one| {
        let standing = Arc::clone(&standing);

        tokio::spawn(async move {
          let mut ticks = tokio::time::interval(one.period());

          ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);

          loop {
            ticks.tick().await;

            if let Err(why) = one.pass(&standing, crate::now_ms()).await {
              eprintln!("{}: the pass failed: {why}", one.name());
            }
          }
        })
      })
      .collect(),
  )
}
