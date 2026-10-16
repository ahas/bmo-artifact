//! The settings: the committed file reads, the local file overrides it, and the environment overrides
//! both (`docs/layout.md`, "Settings").

mod support;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use bmo_artifact::settings::{LOCAL_SETTINGS_FILE, SETTINGS_FILE, Settings};
use support::ROOT;

/// A directory holding the committed settings file, and the local file where one is given.
fn dir_with(local: Option<&str>) -> tempfile::TempDir {
  let dir = tempfile::tempdir().expect("a temporary directory is made");

  fs::copy(Path::new(ROOT).join(SETTINGS_FILE), dir.path().join(SETTINGS_FILE)).expect("the settings copy");

  if let Some(local) = local {
    fs::write(dir.path().join(LOCAL_SETTINGS_FILE), local).expect("the local settings are written");
  }

  dir
}

#[test]
fn the_committed_settings_read() {
  let dir = dir_with(None);
  let settings = Settings::read(dir.path(), Some(HashMap::new())).expect("the settings read");

  assert_eq!(settings.server.listen.to_string(), "127.0.0.1:3230");
  assert_eq!(settings.database.url, "postgres://localhost/bmo_artifact");
  assert_eq!(settings.files.root, Path::new("data"));
  assert_eq!(settings.wording.file, None);
  assert_eq!(settings.time.zone, chrono_tz::UTC);
}

#[test]
fn the_wording_file_and_the_time_zone_are_set_like_any_key() {
  let dir = dir_with(Some("[time]\nzone = \"America/New_York\"\n"));
  let environment = HashMap::from([("BMO_ARTIFACT_WORDING__FILE".to_owned(), "documents.yaml".to_owned())]);
  let settings = Settings::read(dir.path(), Some(environment)).expect("the settings read");

  assert_eq!(settings.time.zone, chrono_tz::America::New_York);
  assert_eq!(settings.wording.file.as_deref(), Some(Path::new("documents.yaml")));
}

#[test]
fn a_time_zone_no_one_names_is_refused() {
  let dir = dir_with(Some("[time]\nzone = \"Nowhere/Town\"\n"));

  assert!(Settings::read(dir.path(), Some(HashMap::new())).is_err());
}

#[test]
fn the_local_example_reads_over_the_committed_settings() {
  let example =
    fs::read_to_string(Path::new(ROOT).join("settings.local.toml.example")).expect("the example reads");
  let dir = dir_with(Some(&example));

  Settings::read(dir.path(), Some(HashMap::new())).expect("the example reads as settings");
}

#[test]
fn the_local_file_overrides_the_committed_one() {
  let dir = dir_with(Some("[database]\nurl = \"postgres://archive.invalid/documents\"\n"));
  let settings = Settings::read(dir.path(), Some(HashMap::new())).expect("the settings read");

  assert_eq!(settings.database.url, "postgres://archive.invalid/documents");
  assert_eq!(settings.server.listen.to_string(), "127.0.0.1:3230");
}

#[test]
fn the_environment_overrides_both_files() {
  let dir = dir_with(Some("[database]\nurl = \"postgres://archive.invalid/documents\"\n"));
  let environment = HashMap::from([
    ("BMO_ARTIFACT_DATABASE__URL".to_owned(), "postgres://env.invalid/documents".to_owned()),
    ("BMO_ARTIFACT_SERVER__LISTEN".to_owned(), "0.0.0.0:4000".to_owned()),
    ("UNRELATED__URL".to_owned(), "ignored".to_owned()),
  ]);
  let settings = Settings::read(dir.path(), Some(environment)).expect("the settings read");

  assert_eq!(settings.database.url, "postgres://env.invalid/documents");
  assert_eq!(settings.server.listen.to_string(), "0.0.0.0:4000");
}

#[test]
fn a_missing_settings_file_is_refused() {
  let dir = tempfile::tempdir().expect("a temporary directory is made");

  assert!(Settings::read(dir.path(), Some(HashMap::new())).is_err());
}
