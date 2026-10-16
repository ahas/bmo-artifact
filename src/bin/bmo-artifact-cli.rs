//! The CLI verbs: each reads as the old verb of the same name does (`docs/api.md`). The command line is
//! built from every verb registered beside its own answer; the two below are the binary's own.

use std::process::ExitCode;

use bmo_artifact::plumbing::verb::{
  Answer, FAILURE, Shell, Verb, VerbError, finished, joined, verb_faults, verbs,
};
use bmo_artifact::settings::Settings;
use bmo_artifact::store::migration;
use clap::{ArgMatches, Command};

/// `version`.
struct Version;

#[derive(clap::Args)]
struct Nothing {}

impl Verb for Version {
  const NAME: &'static str = "version";
  const ABOUT: &'static str = "Prints the repository's version.";

  type Args = Nothing;

  async fn run(&self, _: Nothing, _: &Shell) -> Result<Answer, VerbError> {
    Ok(vec![bmo_artifact::VERSION.to_owned()].into())
  }
}

bmo_artifact::verb!(Version);

/// `migrate`.
struct Migrate;

impl Verb for Migrate {
  const NAME: &'static str = "migrate";
  const ABOUT: &'static str = "Brings the database the settings name up to this repository's schema.";

  type Args = Nothing;

  async fn run(&self, _: Nothing, shell: &Shell) -> Result<Answer, VerbError> {
    let ran = migration::migrate(shell.archive.db()?).await?;

    Ok(vec![format!("migrations run: {ran}")].into())
  }
}

bmo_artifact::verb!(Migrate);

fn command() -> Command {
  Command::new("bmo-artifact-cli")
    .version(bmo_artifact::VERSION)
    .about("The document archive's verbs.")
    .subcommand_required(true)
    .subcommands(verbs().map(|verb| verb.command()))
}

#[tokio::main]
async fn main() -> ExitCode {
  let faults = verb_faults();

  if !faults.is_empty() {
    eprintln!("{}", faults.join("\n"));
    return ExitCode::from(FAILURE);
  }

  let names: Vec<&str> = verbs().map(|verb| verb.name()).collect();
  let line = joined(std::env::args_os().collect(), &names);
  let matches = command().get_matches_from(line);
  let shell = match shell() {
    Ok(shell) => shell,
    Err(why) => {
      eprintln!("{why}");
      return ExitCode::from(FAILURE);
    }
  };
  let ended = finished(run(&matches, &shell).await);

  for line in ended.out {
    println!("{line}");
  }

  if let Some(said) = ended.err {
    eprintln!("{said}");
  }

  ExitCode::from(ended.code)
}

// The shell the settings of the starting directory make, or the sentence saying why there is none.
fn shell() -> Result<Shell, String> {
  let settings = Settings::load().map_err(|why| why.to_string())?;

  Shell::new(settings).map_err(|why| why.to_string())
}

// The verb the line names, run on what its command read.
async fn run(matches: &ArgMatches, shell: &Shell) -> Result<Answer, VerbError> {
  let Some((name, read)) = matches.subcommand() else {
    return Err(VerbError::Refused("no verb is named".to_owned()));
  };
  let verb = verbs()
    .find(|verb| verb.name() == name)
    .ok_or_else(|| VerbError::Refused(format!("no verb is named {name}")))?;

  verb.run(read, shell).await
}
