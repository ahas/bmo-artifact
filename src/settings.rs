//! The settings: `settings.toml`, overridden by `settings.local.toml` and then by the environment
//! (`docs/layout.md`, "Settings").

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use config::{Config, Environment, File, FileFormat};
use serde::Deserialize;

/// The settings file every deployment reads.
pub const SETTINGS_FILE: &str = "settings.toml";
/// The file a deployment overrides it with, which git ignores.
pub const LOCAL_SETTINGS_FILE: &str = "settings.local.toml";
/// What every environment variable that sets a key begins with.
pub const ENVIRONMENT_PREFIX: &str = "BMO_ARTIFACT";

/// Everything the service and the verbs are started with.
#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
  pub server: Server,
  pub database: Database,
  pub files: Files,
  /// Nothing set here reads the English default.
  #[serde(default)]
  pub wording: WordingFile,
  pub time: Time,
  pub documents: Documents,
  /// Nothing set here leaves the console's address unknown.
  #[serde(default)]
  pub console: Console,
  /// Nothing set here leaves every unit's address unknown.
  #[serde(default)]
  pub units: Units,
  pub research: Research,
  /// Nothing set here leaves the prompt tree unknown.
  #[serde(default)]
  pub prompts: Prompts,
}

/// The HTTP server.
#[derive(Clone, Debug, Deserialize)]
pub struct Server {
  /// The address the server listens on.
  pub listen: SocketAddr,
}

/// The database holding the document tables and the research profiles.
#[derive(Clone, Debug, Deserialize)]
pub struct Database {
  /// The PostgreSQL address.
  pub url: String,
}

/// The file root holding each document's folder.
#[derive(Clone, Debug, Deserialize)]
pub struct Files {
  /// The folder holding `searches/<document id>/`; a relative one is read from where the process starts.
  pub root: PathBuf,
}

/// The document wording file (`docs/api.md`, "Wording").
#[derive(Clone, Debug, Default, Deserialize)]
pub struct WordingFile {
  /// The file a document's lines and a search's notices are read from; a relative one is read from where
  /// the process starts, and none reads the English default in `lang/`.
  pub file: Option<PathBuf>,
}

/// How a sentence or a document states a time.
#[derive(Clone, Debug, Deserialize)]
pub struct Time {
  /// The time zone, by its IANA name.
  pub zone: chrono_tz::Tz,
}

/// The documents the agent files (`docs/archive.md`, "A document the agent writes").
#[derive(Clone, Debug, Deserialize)]
pub struct Documents {
  /// The owner's private room: handed the console address and never a share link, and the room a
  /// document goes to where none is named.
  pub room: Option<i64>,
  /// Who asked, where nobody is named.
  pub asker: Option<i64>,
  /// The writer name a document the agent writes is recorded under.
  pub writer: String,
  /// The name a share link the verbs make is recorded as made by.
  pub sender: String,
  /// The prefix that marks a row naming no kind as an import; none marks no row so.
  pub kindless_import_prefix: Option<String>,
}

/// The console the owner reads documents on.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Console {
  /// Its public address, which a document's console address and its share links stand under.
  pub address: Option<String>,
}

/// The address of each unit this repository calls (`docs/api.md`, "What this repository calls").
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Units {
  /// bmo-voice.
  pub voice: Option<String>,
  /// The tool host, which bmo-explorer is reached through.
  pub tool_host: Option<String>,
  /// bmo-agency.
  pub agency: Option<String>,
  /// bmo-job.
  pub job: Option<String>,
  /// bmo-receiver.
  pub receiver: Option<String>,
  /// bmo-gate.
  pub gate: Option<String>,
}

/// The tree an agent's documents are read from.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Prompts {
  /// The tree whose `prompts/agents/<agent>/` holds each agent's documents.
  pub code: Option<PathBuf>,
}

impl Prompts {
  /// The folder of one agent's own documents, refused where the tree is not set.
  pub fn agent_dir(&self, agent: &str) -> Result<PathBuf, String> {
    let code = self
      .code
      .as_ref()
      .ok_or_else(|| "the prompt tree is not set: set code under [prompts] in the settings".to_owned())?;

    Ok(code.join("prompts").join("agents").join(agent))
  }
}

/// What every research run is held to (`docs/research-runs.md`, "Research profiles").
#[derive(Clone, Debug, Deserialize)]
pub struct Research {
  /// The most places one pool limit may hold.
  pub pool_capacity_most: i64,
  pub limits: Limits,
  /// The capabilities a run's research settings open, in capability order.
  pub opens: Vec<Opens>,
  pub search: SearchRulebooks,
  /// Every research key, in the order the console lists them.
  pub keys: Vec<ResearchKey>,
}

/// The bounds each profile field is checked against.
#[derive(Clone, Debug, Deserialize)]
pub struct Limits {
  pub cap_ms: Range,
  pub synthesis_reserve_ms: Range,
  pub research_ms: Range,
  pub max_children: Range,
  pub wait_limit_ms: Range,
  pub interval_ms: Range,
}

/// A bound's least and most, either of which may be left open.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct Range {
  pub min: Option<i64>,
  pub max: Option<i64>,
}

impl Range {
  /// The least, 0 where none is set.
  pub fn least(&self) -> i64 {
    self.min.unwrap_or(0)
  }

  /// The most, unbounded where none is set.
  pub fn most(&self) -> i64 {
    self.max.unwrap_or(i64::MAX)
  }
}

/// One capability a run's settings open, and the run variable that opens it.
#[derive(Clone, Debug, Deserialize)]
pub struct Opens {
  pub capability: String,
  pub variable: String,
}

/// What a search run's standing instructions carry after its task.
#[derive(Clone, Debug, Deserialize)]
pub struct SearchRulebooks {
  /// The rulebooks a search is bound by, in order.
  pub rulebooks: Vec<String>,
  /// Those given by their deciding section alone, and read whole only once it says they apply.
  pub on_request: Vec<String>,
  /// That section's heading.
  pub applies_section: String,
}

/// One research key and what is true of it.
#[derive(Clone, Debug, Deserialize)]
pub struct ResearchKey {
  pub key: String,
  /// The line its provider must carry.
  pub line: String,
  /// Whether it is held to a ceiling and a research close.
  pub clocked: bool,
  /// Whether it opens no branches at all.
  pub branchless: bool,
  /// The ways it accepts a page being fetched.
  pub strategies: Vec<String>,
  pub seed: Seed,
}

/// What one research key starts on; a key with no clock names no ceiling and no reserve.
#[derive(Clone, Debug, Deserialize)]
pub struct Seed {
  pub cap_ms: Option<i64>,
  pub synthesis_reserve_ms: Option<i64>,
  pub child_role: String,
  pub max_children: i64,
  pub wait_limit_ms: i64,
  pub strategy: String,
  pub host_interval_ms: i64,
  pub engine_interval_ms: i64,
}

impl Research {
  /// A research key's declaration, none for a key the settings do not hold.
  pub fn key(&self, key: &str) -> Option<&ResearchKey> {
    self.keys.iter().find(|one| one.key == key)
  }

  /// The capabilities a run's settings leave closed, read through `variable`, in capability order:
  /// every one for a run carrying no research setting at all, and otherwise those whose variable opens
  /// nothing.
  pub fn withheld<'v>(
    &self,
    carries_settings: bool,
    variable: impl Fn(&str) -> Option<&'v str>,
  ) -> Vec<&str> {
    self
      .opens
      .iter()
      .filter(|one| !carries_settings || !variable(&one.variable).is_some_and(opens))
      .map(|one| one.capability.as_str())
      .collect()
  }
}

/// Whether a run variable's value opens the capability it is tied to: it names at least one
/// comma-separated item that is neither empty nor `0`.
pub fn opens(value: &str) -> bool {
  value.split(',').map(str::trim).any(|item| !item.is_empty() && item != "0")
}

/// Why the settings could not be read.
#[derive(Debug, thiserror::Error)]
#[error("the settings could not be read: {0}")]
pub struct SettingsError(#[from] config::ConfigError);

impl Settings {
  /// The settings of the directory the process starts in, with this process's environment over them.
  pub fn load() -> Result<Self, SettingsError> {
    Self::read(Path::new("."), None)
  }

  /// The settings files in `dir`, with `environment` over them, or this process's environment where
  /// none is given.
  pub fn read(dir: &Path, environment: Option<HashMap<String, String>>) -> Result<Self, SettingsError> {
    let environment = Environment::with_prefix(ENVIRONMENT_PREFIX)
      .prefix_separator("_")
      .separator("__")
      .source(environment);

    Ok(
      Config::builder()
        .add_source(File::from(dir.join(SETTINGS_FILE)).format(FileFormat::Toml))
        .add_source(File::from(dir.join(LOCAL_SETTINGS_FILE)).format(FileFormat::Toml).required(false))
        .add_source(environment)
        .build()?
        .try_deserialize()?,
    )
  }
}
