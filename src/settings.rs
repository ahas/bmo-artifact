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
