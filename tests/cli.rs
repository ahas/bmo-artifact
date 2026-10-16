//! The CLI binary as a caller runs it: a verb's answer on standard output and success, a call it does
//! not know refused with a failure (`docs/api.md`, "CLI answers").

mod support;

use std::process::Command;

use support::ROOT;

fn cli(args: &[&str]) -> std::process::Output {
  Command::new(env!("CARGO_BIN_EXE_bmo-artifact-cli"))
    .args(args)
    .current_dir(ROOT)
    .output()
    .expect("the CLI runs")
}

#[test]
fn the_version_verb_prints_the_version() {
  let output = cli(&["version"]);

  assert!(output.status.success());
  assert_eq!(String::from_utf8_lossy(&output.stdout), format!("{}\n", bmo_artifact::VERSION));
  assert_eq!(bmo_artifact::VERSION, include_str!("../VERSION").trim());
}

#[test]
fn a_wording_file_that_cannot_be_read_stops_every_verb() {
  let output = Command::new(env!("CARGO_BIN_EXE_bmo-artifact-cli"))
    .arg("version")
    .env("BMO_ARTIFACT_WORDING__FILE", "no-such-wording.yaml")
    .current_dir(ROOT)
    .output()
    .expect("the CLI runs");

  assert_eq!(output.status.code(), Some(1));
  assert!(output.stdout.is_empty());
  assert!(
    String::from_utf8_lossy(&output.stderr).starts_with("the document wording file no-such-wording.yaml: "),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );
}

#[test]
fn an_unknown_verb_fails() {
  let output = cli(&["no-such-verb"]);

  assert!(!output.status.success());
  assert!(!output.stderr.is_empty());
}
