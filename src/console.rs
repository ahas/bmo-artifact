//! The console bundle as the service answers it (`docs/api.md`, "The console"; bmo-ui's
//! `docs/console-bundles.md`): the manifest a console finds the document screens by, the module, stylesheet
//! and words the build of `client/` writes, and the floor every call the screens make stands on.

use std::path::Path;

use axum::http::{HeaderMap, StatusCode};
use sha2::{Digest, Sha256};

use crate::Refusal;
use crate::http::share_reads::SHARE_READS;
use crate::wire::ConsoleManifest;

/// The route of the service's own API the manifest is answered at.
pub const MANIFEST_ROUTE: &str = "/console-manifest";
/// The path the bundle's words are answered at.
pub const WORDS_PATH: &str = "/console/words";
/// What every file the bundle answers stands under.
pub const FILES_PATH: &str = "/console";

/// The header the console carries the reader's role in.
pub const OWNER_ROLE_HEADER: &str = "owner-role";
/// The header the console carries the signed-in login's owner key in.
pub const OWNER_KEY_HEADER: &str = "owner-key";

/// The least role a reader holds to open the screens.
pub const ROLE: Role = Role::Member;
/// Whether every call the screens make needs the reader's owner key.
pub const OWNER_KEY: bool = false;

// The files the build writes into the bundle's folder.
const MODULE_FILE: &str = "bundle.js";
const STYLESHEET_FILE: &str = "bundle.css";
const WORDS_FILE: &str = "words.json";

// How many hexadecimal digits of a file's digest its path carries.
const DIGEST_DIGITS: usize = 16;

/// A role a reader holds; an administrator holds every role a member holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
  Administrator,
  Member,
}

impl Role {
  /// The role a word names, none for any other word.
  pub fn named(word: &str) -> Option<Self> {
    match word {
      "administrator" => Some(Self::Administrator),
      "member" => Some(Self::Member),
      _ => None,
    }
  }

  /// The word a role is named by.
  pub fn word(self) -> &'static str {
    match self {
      Self::Administrator => "administrator",
      Self::Member => "member",
    }
  }

  /// Whether a reader holding this role may open what needs `needed`.
  pub fn allows(self, needed: Self) -> bool {
    self == Self::Administrator || self == needed
  }
}

/// Whether a call the console relays for a reader carries what the screens need: a role allowing the
/// manifest's, and the owner key where the manifest asks for it.
pub fn screen_admits(headers: &HeaderMap) -> Result<(), Refusal> {
  let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok()).map(str::trim);
  let held = header(OWNER_ROLE_HEADER).and_then(Role::named);

  if !held.is_some_and(|held| held.allows(ROLE)) {
    return Err(Refusal::new(
      StatusCode::FORBIDDEN,
      format!("the screens need a reader whose {OWNER_ROLE_HEADER} allows {}", ROLE.word()),
    ));
  }

  if OWNER_KEY && header(OWNER_KEY_HEADER).is_none_or(str::is_empty) {
    return Err(Refusal::new(
      StatusCode::FORBIDDEN,
      format!("the screens need the reader's {OWNER_KEY_HEADER}"),
    ));
  }

  Ok(())
}

/// One file the bundle answers: the name its path ends in, its media type and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleFile {
  pub name: String,
  pub media_type: &'static str,
  pub bytes: Vec<u8>,
}

impl BundleFile {
  // The file the build wrote, named by its stem, the first digits of its digest and its extension.
  fn read(folder: &Path, file: &str, media_type: &'static str) -> Option<Self> {
    let bytes = std::fs::read(folder.join(file)).ok()?;
    let digest: String = Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect();
    let (stem, extension) = file.split_once('.')?;

    Some(Self {
      name: format!("{stem}.{}.{extension}", &digest[..DIGEST_DIGITS]),
      media_type,
      bytes,
    })
  }

  /// The path of the service's API the file is answered at.
  pub fn path(&self) -> String {
    format!("{FILES_PATH}/{}", self.name)
  }
}

/// What the build of `client/` wrote, read when the service starts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsoleBundle {
  pub module: Option<BundleFile>,
  pub stylesheet: Option<BundleFile>,
  /// Each locale's words under its code.
  pub words: Option<Vec<u8>>,
}

impl ConsoleBundle {
  /// What the folder holds; a file it lacks is none.
  pub fn read(folder: &Path) -> Self {
    Self {
      module: BundleFile::read(folder, MODULE_FILE, "text/javascript; charset=utf-8"),
      stylesheet: BundleFile::read(folder, STYLESHEET_FILE, "text/css; charset=utf-8"),
      words: std::fs::read(folder.join(WORDS_FILE)).ok(),
    }
  }

  /// Whether the folder held a bundle: a module and its words.
  pub fn built(&self) -> bool {
    self.module.is_some() && self.words.is_some()
  }

  /// The manifest, none where no bundle was built.
  pub fn manifest(&self) -> Option<ConsoleManifest> {
    let module = self.module.as_ref().filter(|_| self.words.is_some())?;

    Some(ConsoleManifest {
      name: "artifact".into(),
      title: "artifact.title.screen".into(),
      icon: "i-material-symbols-archive-outline".into(),
      screen_path: "/archive".into(),
      service_name: "artifact".into(),
      bundle: module.path(),
      stylesheet: self.stylesheet.as_ref().map(BundleFile::path),
      words: WORDS_PATH.into(),
      role: ROLE.word().into(),
      owner_key: OWNER_KEY,
      events: false,
      websocket: false,
      push_kinds: Vec::new(),
      share_reads: SHARE_READS.iter().map(|path| (*path).to_owned()).collect(),
    })
  }

  /// The words, none where no bundle was built.
  pub fn words(&self) -> Option<&[u8]> {
    self.words.as_deref().filter(|_| self.module.is_some())
  }

  /// The module or the stylesheet the manifest names by `name`, none for any other name.
  pub fn file(&self, name: &str) -> Option<&BundleFile> {
    [self.module.as_ref(), self.stylesheet.as_ref()]
      .into_iter()
      .flatten()
      .filter(|_| self.built())
      .find(|file| file.name == name)
  }
}
