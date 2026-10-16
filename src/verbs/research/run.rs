//! One research run's model, from its road to its row (`docs/research-runs.md`, "The launch and the runner";
//! `docs/api.md`, "Runs apart"): the research key's tier on the provider its line starts on, or the
//! registration it is kept on; the run's research settings handed to its web tools in its environment; the run
//! carried apart by bmo-agency, which walks the accounts its road is offered, feeds every line to the run's
//! log and records what it spent; every line it gains kept here, so a run stopped from outside still has what
//! it wrote; and the run's row closed once, with its pool places given back.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::Value;

use super::{LINE, Profile, Told, clock, finish_leases, owner as research_owner};
use crate::clients::agency::{
  Agency, ApartAsking, ApartLog, ApartReply, Instructions, Note, Road as Going, RunClose,
};
use crate::control::research::{checked_effort, launch_mapping, research_provider};
use crate::now_ms;

/// How often a run apart is asked how it stands.
pub const APART_POLL: Duration = Duration::from_millis(500);

/// Every text block the run's assistant turns wrote, in order: what a run stopped from outside had written.
pub fn assistant_texts(stream: &str) -> Vec<String> {
  stream
    .lines()
    .map(str::trim)
    .filter(|line| line.starts_with('{'))
    .filter_map(|line| serde_json::from_str::<Value>(line).ok())
    .filter(|event| event.get("type").and_then(Value::as_str) == Some("assistant"))
    .flat_map(|event| event.pointer("/message/content").and_then(Value::as_array).cloned().unwrap_or_default())
    .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
    .filter_map(|block| block.get("text").and_then(Value::as_str).map(str::to_owned))
    .collect()
}

/// What one run's web tools are told, laid over what every run of the line is: the close its profile's clock
/// gives it where it holds one, the owner its branches are held under, its source and role, its branch
/// allowance — none for a run with no owner — its room, and its fetch strategy and pacing.
pub fn run_told(
  standing: Told,
  profile: &Profile,
  owner: Option<String>,
  source: &str,
  chat_id: i64,
  now: i64,
) -> Told {
  Told {
    closes_at: clock(profile, now).map(|clock| clock.closes_at),
    max_children: Some(if owner.is_some() { profile.max_children() } else { 0 }),
    owner,
    source: Some(source.to_owned()),
    role: Some(profile.child_role().to_owned()),
    chat_id: Some(chat_id),
    strategy: Some(profile.strategy().to_owned()),
    host_interval_ms: Some(profile.host_interval_ms()),
    engine_interval_ms: Some(profile.engine_interval_ms()),
    ..standing
  }
}

/// The road a run goes out on: the research key's tier on the provider its line starts on, or the
/// registration it is kept on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Road {
  Key(String),
  Kept(String),
}

/// One run as its runner starts it: its room and row, the log its lines are fed to, and what it has written.
pub struct Research {
  agency: Arc<dyn Agency>,
  source: &'static str,
  chat_id: i64,
  run_id: Option<i64>,
  owner: Option<String>,
  log: Option<ApartLog>,
  streamed: Arc<Mutex<Vec<String>>>,
  stopping: Arc<AtomicBool>,
  context_tokens: Option<i64>,
  closed: bool,
}

impl Research {
  /// A run of `source` for the room, under its row and unit where its launcher opened them; a run with no
  /// unit keeps no log.
  pub fn new(agency: Arc<dyn Agency>, source: &'static str, chat_id: i64, run_id: Option<i64>, unit: &str) -> Self {
    Self {
      agency,
      source,
      chat_id,
      run_id,
      owner: run_id.map(|id| research_owner(source, id)),
      log: (!unit.is_empty()).then(|| ApartLog {
        run: unit.to_owned(),
        chat_id: Some(chat_id),
      }),
      streamed: Arc::default(),
      stopping: Arc::new(AtomicBool::new(false)),
      context_tokens: None,
      closed: false,
    }
  }

  /// The name the run's places are held under in the pool; none for a run with no row, which opens no branch.
  pub fn owner(&self) -> Option<&str> {
    self.owner.as_deref()
  }

  /// The run's log, by the name its lines are grouped under, where it keeps one.
  pub fn log_run(&self) -> Option<String> {
    self.log.as_ref().map(|log| log.run.clone())
  }

  /// A note in the run's log, where it keeps one.
  pub async fn note(&self, text: String) {
    let Some(log) = &self.log else { return };
    let note = Note {
      agent: self.source.to_owned(),
      run: log.run.clone(),
      chat_id: log.chat_id,
      text,
    };

    if let Err(why) = self.agency.note(&note).await {
      eprintln!("{} run note could not be written: {why}", self.source);
    }
  }

  /// What the run had written so far, a record line to a line.
  pub fn streamed(&self) -> String {
    self.streamed.lock().unwrap_or_else(PoisonError::into_inner).join("\n")
  }

  /// The run asked to stop: a model it carries is stopped, and one not launched yet never is.
  pub fn stop(&self) {
    self.stopping.store(true, Ordering::SeqCst);
  }

  /// Whether the run was asked to stop.
  pub fn stopping(&self) -> bool {
    self.stopping.load(Ordering::SeqCst)
  }

  /// The flag a stop sets, for a wait that is called off with it.
  pub fn stop_flag(&self) -> Arc<AtomicBool> {
    Arc::clone(&self.stopping)
  }

  /// The run's model carried on its road, `prompt` on its input, under the instructions bmo-agency composes
  /// for the line: its answer; or why it gave none, in the words the room's failure is logged with.
  /// `standing` is what every run of the line's tools is told — its tools and the rulebooks it hands out —
  /// which the run's own settings are laid over. The provider must carry the line, and the effort be one the
  /// provider and model accept; neither is ever changed to fit.
  pub async fn ask(
    &mut self,
    standing: Told,
    road: &Road,
    profile: &Profile,
    instructions: Instructions,
    prompt: &str,
  ) -> Result<ApartReply, String> {
    if self.stopping() {
      return Err("stopped before the run was launched".to_owned());
    }

    let told = run_told(standing, profile, self.owner.clone(), self.source, self.chat_id, now_ms());
    let (going, work_key) = match road {
      Road::Key(key) => {
        let agency = &*self.agency;
        let provider = research_provider(agency, LINE).await?;
        let mapping = launch_mapping(agency, key, &provider).await?;
        let effort = checked_effort(agency, &mapping).await?;
        let going = Going::Key {
          provider: mapping.provider,
          model: mapping.model,
          effort: Some(effort),
          tier: mapping.tier,
        };

        (going, key.clone())
      }
      Road::Kept(registration) => (
        Going::Registration {
          name: registration.clone(),
        },
        LINE.to_owned(),
      ),
    };

    self.ask_composed(going, &work_key, &told, instructions, prompt).await
  }

  /// A run already resolved — a branch's, on its pool place's model — carried apart, `prompt` on its input:
  /// its answer; or why it gave none.
  pub async fn ask_composed(
    &mut self,
    road: Going,
    work_key: &str,
    told: &Told,
    instructions: Instructions,
    prompt: &str,
  ) -> Result<ApartReply, String> {
    if self.stopping() {
      return Err("stopped before the run was launched".to_owned());
    }

    let asking = ApartAsking {
      agent: LINE.to_owned(),
      line: LINE.to_owned(),
      title: format!("{} run", self.source),
      work_key: work_key.to_owned(),
      road,
      instructions,
      environment: told.environment(),
      prompt: prompt.to_owned(),
      log: self.log.clone(),
    };
    let reply = self.carry(&asking).await?;

    self.context_tokens = reply.context_tokens.or(self.context_tokens);

    if !reply.live_tasks.is_empty() {
      return Err("the run ended with its own work still running".to_owned());
    }

    Ok(reply)
  }

  // The run apart started, asked how it stands until it is no longer running, every line it gained kept, and
  // stopped the moment the run is.
  async fn carry(&mut self, asking: &ApartAsking) -> Result<ApartReply, String> {
    let agency = Arc::clone(&self.agency);
    let id = agency.start_apart(asking).await.map_err(|fault| fault.to_string())?;
    let keep = |lines: Vec<String>| *self.streamed.lock().unwrap_or_else(PoisonError::into_inner) = lines;

    loop {
      if self.stopping() {
        if let Err(why) = agency.stop_apart(&id).await {
          eprintln!("{} run apart could not be stopped: {why}", self.source);
        }

        if let Ok(apart) = agency.apart(&id).await {
          keep(apart.streamed);
        }

        return Err("the run was stopped".to_owned());
      }

      let apart = agency.apart(&id).await.map_err(|fault| fault.to_string())?;
      let running = apart.running();
      let (state, reply, failure) = (apart.state, apart.reply, apart.failure);

      keep(apart.streamed);

      if !running {
        return match (state.as_str(), reply) {
          ("answered", Some(reply)) => Ok(reply),
          ("stopped", _) => Err("the run was stopped".to_owned()),
          _ => Err(failure.unwrap_or_else(|| format!("the run apart ended {state} with no answer"))),
        };
      }

      tokio::time::sleep(APART_POLL).await;
    }
  }

  /// The run counted under its agent.
  pub async fn count_run(&self) {
    if let Err(why) = self.agency.count_run(self.source).await {
      eprintln!("{} run could not be counted: {why}", self.source);
    }
  }

  /// The run ended with how it ended: every place it holds in the pool given back, and its row closed with
  /// the context it ended carrying. Whether this was the ending that closed the row: the first reached is
  /// the real one.
  pub async fn close(&mut self, outcome: &str) -> bool {
    if let Some(owner) = &self.owner
      && let Err(why) = finish_leases(&*self.agency, owner, &format!("{} run {outcome}", self.source)).await
    {
      eprintln!("{} run pool places could not be given back: {why}", self.source);
    }

    let Some(run_id) = self.run_id else { return false };

    if self.closed {
      return false;
    }

    self.closed = true;

    let close = RunClose {
      finished_at: now_ms(),
      outcome: outcome.to_owned(),
      context_tokens: self.context_tokens,
    };

    match self.agency.close_run(run_id, &close).await {
      Ok(closed) => closed,
      Err(why) => {
        eprintln!("the run's row could not be closed: {why}");
        true
      }
    }
  }
}
