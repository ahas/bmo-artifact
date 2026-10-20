//! The document wording file: the English default's lines fill in with their marks, a file the settings name
//! is read in its place and held to the same keys (`docs/api.md`, "The document wording").

mod support;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use bmo_artifact::helpers::marks::fill_marks;
use bmo_artifact::plumbing::verb::Shell;
use bmo_artifact::settings::Settings;
use bmo_artifact::wording::{ENGLISH, Wording, keys};
use serde_json::Value;
use support::ROOT;

#[test]
fn the_english_default_holds_its_lines_each_filled_in_by_its_marks() {
  let english: Value = serde_saphyr::from_str(ENGLISH).expect("the English default is YAML");
  let wording = Wording::english().expect("the English default reads");

  assert_eq!(keys(&english).into_iter().collect::<Vec<_>>(), ["import.missing"]);
  assert_eq!(
    fill_marks(&wording.import.missing, &[("n", "2"), ("caption", "the chart")]),
    "Figure 2 could not be kept: the chart"
  );
}

fn written(text: &str) -> (tempfile::TempDir, std::path::PathBuf) {
  let dir = tempfile::tempdir().expect("a folder");
  let path = dir.path().join("documents.yaml");

  fs::write(&path, text).expect("written");
  (dir, path)
}

#[test]
fn a_file_lacking_a_key_or_holding_one_no_line_reads_is_refused_with_the_key() {
  let (_dir, path) = written("import: {}\n");
  let refused = Wording::read(Some(&path)).expect_err("refused").to_string();

  assert!(refused.ends_with(": it lacks import.missing"), "{refused}");
  assert!(refused.contains(&path.display().to_string()), "{refused}");

  let (_dir, path) = written(&format!("{ENGLISH}  later: \"x\"\n"));
  let refused = Wording::read(Some(&path)).expect_err("refused").to_string();

  assert!(refused.ends_with(": no line is read under import.later"), "{refused}");
}

#[test]
fn a_file_that_is_not_yaml_or_not_there_is_refused() {
  let (dir, path) = written("import: [");

  assert!(Wording::read(Some(&path)).is_err());
  assert!(Wording::read(Some(&dir.path().join("absent.yaml"))).is_err());
}

#[test]
fn a_shell_reads_the_wording_its_settings_name_in_place_of_the_default() {
  let (_dir, path) = written(&ENGLISH.replace("could not be kept", "was lost"));
  let environment = HashMap::from([("BMO_ARTIFACT_WORDING__FILE".to_owned(), path.display().to_string())]);
  let settings = Settings::read(Path::new(ROOT), Some(environment)).expect("the settings read");

  assert_eq!(
    Shell::new(settings).expect("the shell stands").wording.import.missing,
    "Figure {n} was lost: {caption}"
  );

  let environment = HashMap::from([("BMO_ARTIFACT_WORDING__FILE".to_owned(), "absent.yaml".to_owned())]);
  let settings = Settings::read(Path::new(ROOT), Some(environment)).expect("the settings read");

  assert!(Shell::new(settings).is_err());
}
