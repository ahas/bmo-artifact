//! The CLI verbs: each reads as the old verb of the same name does (`docs/api.md`).

use std::process::ExitCode;

use bmo_artifact::plumbing::verb::{Answer, FAILURE, Shell, VerbError, finished};
use bmo_artifact::settings::Settings;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bmo-artifact-cli", version = bmo_artifact::VERSION, about = "The document archive's verbs.")]
struct Cli {
  #[command(subcommand)]
  verb: Verb,
}

#[derive(Subcommand)]
enum Verb {
  /// Prints the repository's version.
  Version,
}

fn main() -> ExitCode {
  let cli = Cli::parse();
  let shell = match Settings::load() {
    Ok(settings) => Shell::new(settings),
    Err(why) => {
      eprintln!("{why}");
      return ExitCode::from(FAILURE);
    }
  };
  let ended = finished(run(cli.verb, &shell));

  for line in ended.out {
    println!("{line}");
  }

  if let Some(said) = ended.err {
    eprintln!("{said}");
  }

  ExitCode::from(ended.code)
}

fn run(verb: Verb, _shell: &Shell) -> Result<Answer, VerbError> {
  match verb {
    Verb::Version => Ok(vec![bmo_artifact::VERSION.to_owned()].into()),
  }
}
