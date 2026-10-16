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
}

/// The HTTP server.
#[derive(Clone, Debug, Deserialize)]
pub struct Server {
  /// The address the server listens on.
  pub listen: SocketAddr,
}

/// The database holding the six document tables.
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
  /// The tool host, which bmo-explorer's page reading is reached through.
  pub tool_host: Option<String>,
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
