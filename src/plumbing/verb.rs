//! A verb, its answer and its errors, the shell it runs on, and how the CLI reads its line and ends with
//! them.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use clap::{ArgMatches, Args, Command, FromArgMatches};
use sea_orm::{DatabaseConnection, DbErr};
use tokio::sync::OnceCell;

use crate::plumbing::refusal::Refusal;
use crate::pool::{Archive, Fault};
use crate::settings::Settings;
use crate::store::documents::settle;
use crate::wording::{Wording, WordingError};

pub use inventory;

/// One verb the CLI runs, registered beside its answer with `bmo_artifact::verb!`.
pub trait Verb: Send + Sync + 'static {
  /// The words a caller runs it by: `archive doc`.
  const NAME: &'static str;
  /// The other spellings it keeps: `archive-doc`.
  const ALIASES: &'static [&'static str] = &[];
  /// What it does, in the line the CLI's help shows.
  const ABOUT: &'static str;

  /// What it is given on the line.
  type Args: Args + FromArgMatches + Send;

  fn run(&self, args: Self::Args, shell: &Shell) -> impl Future<Output = Result<Answer, VerbError>> + Send;
}

/// What running a verb comes to.
pub type Running<'a> = Pin<Box<dyn Future<Output = Result<Answer, VerbError>> + Send + 'a>>;

/// A verb as the CLI holds it among the others.
pub trait Runs: Send + Sync {
  fn name(&self) -> &'static str;
  fn aliases(&self) -> &'static [&'static str];
  /// Its command, which reads what it is given.
  fn command(&self) -> Command;
  /// It run on what its command read.
  fn run<'a>(&'a self, matches: &'a ArgMatches, shell: &'a Shell) -> Running<'a>;
}

impl<V: Verb> Runs for V {
  fn name(&self) -> &'static str {
    V::NAME
  }

  fn aliases(&self) -> &'static [&'static str] {
    V::ALIASES
  }

  // The about is set after the arguments, which would otherwise put their own type's comment in its place.
  fn command(&self) -> Command {
    V::Args::augment_args(Command::new(V::NAME).visible_aliases(V::ALIASES.iter().copied())).about(V::ABOUT)
  }

  fn run<'a>(&'a self, matches: &'a ArgMatches, shell: &'a Shell) -> Running<'a> {
    Box::pin(async move {
      let args = V::Args::from_arg_matches(matches).map_err(|why| VerbError::Refused(why.to_string()))?;

      Verb::run(self, args, shell).await
    })
  }
}

/// One registered verb.
pub struct RegisteredVerb(pub &'static dyn Runs);

inventory::collect!(RegisteredVerb);

/// Registers a verb with the CLI: `bmo_artifact::verb!(Migrate);`
#[macro_export]
macro_rules! verb {
  ($verb:expr) => {
    $crate::plumbing::verb::inventory::submit! { $crate::plumbing::verb::RegisteredVerb(&$verb) }
  };
}

/// Every verb linked into this binary.
pub fn verbs() -> impl Iterator<Item = &'static dyn Runs> {
  inventory::iter::<RegisteredVerb>.into_iter().map(|entry| entry.0)
}

/// Each name or spelling two verbs claim, as a sentence; none where every verb has its own.
pub fn verb_faults() -> Vec<String> {
  let mut claimed: BTreeMap<&str, usize> = BTreeMap::new();

  for verb in verbs() {
    for name in std::iter::once(verb.name()).chain(verb.aliases().iter().copied()) {
      *claimed.entry(name).or_default() += 1;
    }
  }

  claimed
    .into_iter()
    .filter(|(_, claims)| *claims > 1)
    .map(|(name, claims)| format!("{name}: claimed by {claims} verbs"))
    .collect()
}

/// A command line with a verb's two words, given as two arguments, taken as the one name the verb is
/// registered under, so `archive` and `doc` are read as the verb `archive doc`. Any other line is left as
/// it is.
pub fn joined(mut line: Vec<OsString>, names: &[&str]) -> Vec<OsString> {
  let two = match (line.get(1).and_then(|word| word.to_str()), line.get(2).and_then(|word| word.to_str())) {
    (Some(first), Some(second)) => format!("{first} {second}"),
    _ => return line,
  };

  if names.contains(&two.as_str()) {
    line.splice(1..3, [OsString::from(two)]);
  }

  line
}

/// What a verb answers: the lines it prints, and the code it ends with where that is not success.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Answer {
  pub lines: Vec<String>,
  pub code: u8,
}

impl From<Vec<String>> for Answer {
  fn from(lines: Vec<String>) -> Self {
    Self { lines, code: 0 }
  }
}

/// Why a verb did not do what it was asked, in the line it prints before it ends with a failure.
#[derive(Debug, thiserror::Error)]
pub enum VerbError {
  /// The caller asked for something the verb refuses, said in the verb's own sentence.
  #[error("{0}")]
  Refused(String),
  /// The verb ended with a code of its own, said in its own sentence.
  #[error("{said}")]
  Ended { code: u8, said: String },
  /// The verb did part of what it was asked: the lines it prints of that, and the refusal of the rest.
  #[error("{refusal}")]
  Partly { lines: Vec<String>, refusal: String },
  /// The carried code refused, or the archive's database failed under it.
  #[error(transparent)]
  Archive(#[from] Refusal),
}

impl From<DbErr> for VerbError {
  fn from(error: DbErr) -> Self {
    Refusal::from(error).into()
  }
}

impl From<Fault> for VerbError {
  fn from(fault: Fault) -> Self {
    Refusal::from(fault).into()
  }
}

impl From<sqlx::Error> for VerbError {
  fn from(error: sqlx::Error) -> Self {
    Fault::from(error).into()
  }
}

/// What a verb run from the CLI stands on.
#[derive(Clone, Debug)]
pub struct Shell {
  pub settings: Settings,
  /// The database the settings name, opened on the first statement a verb asks of it.
  pub archive: Archive,
  /// The document wording the settings name, read when the shell is made.
  pub wording: Wording,
  // Whether the document tables have been settled in this process.
  settled: Arc<OnceCell<()>>,
}

impl Shell {
  pub fn new(settings: Settings) -> Result<Self, WordingError> {
    let archive = Archive::new(settings.database.url.clone());
    let wording = Wording::read(settings.wording.file.as_deref())?;

    Ok(Self {
      settings,
      archive,
      wording,
      settled: Arc::default(),
    })
  }

  /// The database, its document tables settled before the first statement asked of them
  /// (`docs/layout.md`, "Storage").
  pub async fn documents(&self) -> Result<&DatabaseConnection, VerbError> {
    let db = self.archive.db()?;
    let prefix = self.settings.documents.kindless_import_prefix.as_deref();

    self.settled.get_or_try_init(|| settle(db, prefix)).await?;

    Ok(db)
  }
}

/// How the CLI ends a verb: what it prints on standard output, what on standard error, and its code.
#[derive(Debug, PartialEq, Eq)]
pub struct Finished {
  pub out: Vec<String>,
  pub err: Option<String>,
  pub code: u8,
}

/// The code a failure ends with, where the verb names none of its own.
pub const FAILURE: u8 = 1;

/// How a verb's answer or error ends the CLI (`docs/api.md`, "The command").
pub fn finished(answered: Result<Answer, VerbError>) -> Finished {
  match answered {
    Ok(answer) => Finished {
      out: answer.lines,
      err: None,
      code: answer.code,
    },
    Err(VerbError::Partly { lines, refusal }) => Finished {
      out: lines,
      err: Some(refusal).filter(|refusal| !refusal.is_empty()),
      code: FAILURE,
    },
    Err(VerbError::Ended { code, said }) => Finished {
      out: Vec::new(),
      err: Some(said),
      code,
    },
    Err(VerbError::Archive(refusal)) => Finished {
      out: Vec::new(),
      err: Some(refusal.detail),
      code: FAILURE,
    },
    Err(VerbError::Refused(said)) => Finished {
      out: Vec::new(),
      err: Some(said),
      code: FAILURE,
    },
  }
}
