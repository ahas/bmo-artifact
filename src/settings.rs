//! The settings: `settings.toml`, overridden by `settings.local.toml` and then by the environment
//! (`docs/api.md`, "Settings").

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

/// Everything the service and its command are started with.
#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
  pub server: Server,
  /// Nothing set here lets no console in and reads the bundle from `client/dist`.
  #[serde(default)]
  pub console: Console,
  /// Nothing set here reads the player from `client/player`.
  #[serde(default)]
  pub player: Player,
  pub database: Database,
  pub files: Files,
  pub explorer: Explorer,
  /// Nothing set here names no account service: no reader stands in any unit.
  #[serde(default)]
  pub accounts: Accounts,
  #[serde(default)]
  pub share: Share,
  /// Nothing set here reads the English default.
  #[serde(default)]
  pub wording: WordingFile,
  /// Nothing set here marks no row naming no kind as an import.
  #[serde(default)]
  pub documents: Documents,
  /// Nothing set here reaches Graph at its own address and names no page token road.
  #[serde(default)]
  pub sharepoint: SharePoint,
}

/// The HTTP server.
#[derive(Clone, Debug, Deserialize)]
pub struct Server {
  /// The address the server listens on.
  pub listen: SocketAddr,
  /// The service's token, which reaches every route; an empty one lets no caller in by it.
  #[serde(default)]
  pub token: String,
}

/// The console the document screens are relayed through (`docs/api.md`, "The console").
#[derive(Clone, Debug, Deserialize)]
pub struct Console {
  /// The console's token, which reaches what the screens use; an empty one lets no console in.
  #[serde(default)]
  pub token: String,
  /// The folder the build of `client/` writes the bundle into; a relative one is read from the directory the
  /// settings stand in.
  #[serde(default = "Console::default_bundle")]
  pub bundle: PathBuf,
  /// The other services the screens call through the console, each by the name the console relays it under.
  #[serde(default)]
  pub calls: Vec<String>,
}

impl Console {
  fn default_bundle() -> PathBuf {
    PathBuf::from("client/dist")
  }
}

impl Default for Console {
  fn default() -> Self {
    Self {
      token: String::new(),
      bundle: Self::default_bundle(),
      calls: Vec::new(),
    }
  }
}

/// bmo-ui's audio player a hosted page loads (`docs/api.md`, "Readings").
#[derive(Clone, Debug, Deserialize)]
pub struct Player {
  /// The folder the build of `client/` writes the player's script and its words into; a relative one is read
  /// from the directory the settings stand in.
  #[serde(default = "Player::default_folder")]
  pub folder: PathBuf,
}

impl Player {
  fn default_folder() -> PathBuf {
    PathBuf::from("client/player")
  }
}

impl Default for Player {
  fn default() -> Self {
    Self {
      folder: Self::default_folder(),
    }
  }
}

/// The database holding the document tables.
#[derive(Clone, Debug, Deserialize)]
pub struct Database {
  /// The PostgreSQL address.
  pub url: String,
}

/// The file root holding each document's folder.
#[derive(Clone, Debug, Deserialize)]
pub struct Files {
  /// The folder holding `searches/<document id>/`; a relative one is read from the directory the settings
  /// stand in.
  pub root: PathBuf,
}

/// The page reader, bmo-explorer's client.
#[derive(Clone, Debug, Deserialize)]
pub struct Explorer {
  /// The data root the client is opened over, where bmo-explorer keeps its own folder.
  pub data_root: PathBuf,
}

/// The account service a reader's unit and a share's company are asked of (`docs/api.md`, "What this service
/// calls").
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Accounts {
  /// Its base address; an empty one names none.
  #[serde(default)]
  pub address: String,
  /// Its server's token, presented as the bearer.
  #[serde(default)]
  pub token: String,
}

/// The share links.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Share {
  /// The address a link's `url` stands under: the address, a `/` and the token.
  #[serde(default)]
  pub address: String,
}

/// The document wording file (`docs/api.md`, "The document wording").
#[derive(Clone, Debug, Default, Deserialize)]
pub struct WordingFile {
  /// The file the lines are read from; a relative one is read from the directory the settings stand in, and
  /// none reads the English default in `lang/`.
  pub file: Option<PathBuf>,
}

/// How the document tables are settled (`docs/artifact.md`, "`searches`").
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Documents {
  /// The prefix that marks a row naming no kind as an import; none marks no row so.
  pub kindless_import_prefix: Option<String>,
}

/// Microsoft Graph's own address.
pub const GRAPH: &str = "https://graph.microsoft.com/v1.0";

/// What the SharePoint place reaches (`docs/artifact.md`, "The SharePoint place"): Microsoft Graph, and
/// The optional page-token service's page token road a person's Graph token is borrowed through.
#[derive(Clone, Debug, Deserialize)]
pub struct SharePoint {
  /// Graph's base address.
  #[serde(default = "graph")]
  pub graph: String,
  /// The page token road's base address; an empty one names none.
  #[serde(default)]
  pub tokens: String,
  /// This service's own caller token on the page token road.
  #[serde(default)]
  pub token: String,
}

fn graph() -> String {
  GRAPH.to_owned()
}

impl Default for SharePoint {
  fn default() -> Self {
    Self {
      graph: graph(),
      tokens: String::new(),
      token: String::new(),
    }
  }
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

  /// The settings a command stands on (`docs/api.md`, "The command"): the starting directory's where its
  /// settings file stands there, else the deployed release's own, beside the binary's `bin/`.
  pub fn for_command() -> Result<Self, SettingsError> {
    if Path::new(SETTINGS_FILE).exists() {
      return Self::load();
    }

    let release = std::env::current_exe()
      .ok()
      .and_then(|binary| binary.canonicalize().ok())
      .and_then(|binary| binary.parent()?.parent().map(Path::to_path_buf));

    match release {
      Some(release) => Self::read(&release, None),
      None => Self::load(),
    }
  }

  /// The settings files in `dir`, with `environment` over them, or this process's environment where
  /// none is given. Every path a setting names is made to stand on `dir`, so a relative one reads from
  /// there and not from wherever the process was started.
  pub fn read(dir: &Path, environment: Option<HashMap<String, String>>) -> Result<Self, SettingsError> {
    Self::layered(dir, &dir.join(LOCAL_SETTINGS_FILE), false, environment)
  }

  /// The settings file in `dir`, the local file `local` over it, and `environment` over both, or this process's
  /// environment where none is given; a local file that does not stand is refused where it is `required`. Every
  /// path a setting names is made to stand on `dir`.
  pub fn layered(
    dir: &Path,
    local: &Path,
    required: bool,
    environment: Option<HashMap<String, String>>,
  ) -> Result<Self, SettingsError> {
    let environment = Environment::with_prefix(ENVIRONMENT_PREFIX)
      .prefix_separator("_")
      .separator("__")
      .source(environment);

    let mut settings: Self = Config::builder()
      .add_source(File::from(dir.join(SETTINGS_FILE)).format(FileFormat::Toml))
      .add_source(File::from(local).format(FileFormat::Toml).required(required))
      .add_source(environment)
      .build()?
      .try_deserialize()?;

    settings.anchor(dir);

    Ok(settings)
  }

  /// Every path the settings name, made to stand on the directory they were read from.
  fn anchor(&mut self, dir: &Path) {
    anchor(dir, &mut self.console.bundle);
    anchor(dir, &mut self.player.folder);
    anchor(dir, &mut self.files.root);
    anchor(dir, &mut self.explorer.data_root);

    if let Some(file) = self.wording.file.as_mut() {
      anchor(dir, file);
    }
  }
}

/// A path made to stand on `dir` where it is relative.
fn anchor(dir: &Path, path: &mut PathBuf) {
  if path.is_relative() {
    *path = dir.join(&*path);
  }
}
