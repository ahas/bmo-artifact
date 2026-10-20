//! `bmo-artifact nouns` as `bmo` asks it (`docs/api.md`, "The command"): the nouns the command carries, one
//! a line, answered from any directory and each naming a command the binary runs.

use std::path::Path;
use std::process::{Command, Output, Stdio};

use bmo_artifact::commands::NOUNS;
use bmo_artifact::plumbing::verb::verbs;

fn nouns_from(directory: &Path) -> Output {
  let mut command = Command::new(env!("CARGO_BIN_EXE_bmo-artifact"));

  for (name, _) in std::env::vars().filter(|(name, _)| name.starts_with("BMO_ARTIFACT_")) {
    command.env_remove(name);
  }

  command
    .arg("nouns")
    .current_dir(directory)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .expect("the CLI starts")
}

#[test]
fn the_nouns_are_printed_one_a_line_from_any_directory() {
  let elsewhere = tempfile::tempdir().expect("a folder");
  let done = nouns_from(elsewhere.path());

  assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
  assert_eq!(String::from_utf8_lossy(&done.stdout), "html-read\n");
  assert_eq!(NOUNS, ["html-read"]);
}

#[test]
fn every_noun_names_a_command_the_binary_runs() {
  let names: Vec<&str> = verbs().map(|verb| verb.name()).collect();

  for noun in NOUNS {
    assert!(names.contains(noun), "{noun} is no command of this binary");
  }
}
