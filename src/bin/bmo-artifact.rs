//! The one binary: the service, its command and its harness plugin (`docs/api.md`, "The command"). Its line
//! is built from every command registered beside its own run.

use std::process::ExitCode;

use bmo_artifact::plumbing::verb::{
  Answer, FAILURE, Shell, VerbError, finished, joined, verb_faults, verbs,
};
use bmo_artifact::settings::Settings;
use clap::{ArgMatches, Command};

fn command() -> Command {
  Command::new("bmo-artifact")
    .version(bmo_artifact::VERSION)
    .about("Documents kept, searched and shared: the service, its command and its MCP server.")
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

// The command the line names, run on what it read.
async fn run(matches: &ArgMatches, shell: &Shell) -> Result<Answer, VerbError> {
  let Some((name, read)) = matches.subcommand() else {
    return Err(VerbError::Refused("no command is named".to_owned()));
  };
  let verb = verbs()
    .find(|verb| verb.name() == name)
    .ok_or_else(|| VerbError::Refused(format!("no command is named {name}")))?;

  verb.run(read, shell).await
}
