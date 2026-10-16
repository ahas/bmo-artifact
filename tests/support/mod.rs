//! What the repository checks share: the root, and the walk over the files they read.

// Each test file is its own crate and uses only part of this module.
#![allow(dead_code)]

pub mod scratch;

use std::fs;
use std::path::{Path, PathBuf};

pub const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// Directories no check reads: build output and installed packages.
const SKIPPED: [&str; 2] = ["target", "node_modules"];

/// Whether a directory is one no check reads: those above, and every hidden one, git's own and the
/// tools' local state among them.
pub fn is_skipped(name: &str) -> bool {
  SKIPPED.contains(&name) || name.starts_with('.')
}

/// Every file under `dir`, at any depth, skipping what no check reads. A directory that does not exist
/// holds no files.
pub fn files_under(dir: &Path) -> Vec<PathBuf> {
  let mut files = Vec::new();
  walk(dir, &mut files, &mut Vec::new());
  files.sort();
  files
}

/// Every directory under `dir`, at any depth, skipping what no check reads.
pub fn dirs_under(dir: &Path) -> Vec<PathBuf> {
  let mut dirs = Vec::new();
  walk(dir, &mut Vec::new(), &mut dirs);
  dirs.sort();
  dirs
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>, dirs: &mut Vec<PathBuf>) {
  let Ok(entries) = fs::read_dir(dir) else { return };

  for entry in entries {
    let entry = entry.expect("an entry is readable");
    let kind = entry.file_type().expect("an entry's type is readable");

    if kind.is_dir() {
      if !is_skipped(&entry.file_name().to_string_lossy()) {
        dirs.push(entry.path());
        walk(&entry.path(), files, dirs);
      }
    } else if kind.is_file() {
      files.push(entry.path());
    }
  }
}

/// A path as it reads relative to the root, for a failure's message.
pub fn shown(path: &Path) -> String {
  path.strip_prefix(ROOT).unwrap_or(path).display().to_string()
}

/// Every Rust file of the repository.
pub fn rust_files(dir: &Path) -> Vec<PathBuf> {
  files_under(dir)
    .into_iter()
    .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
    .collect()
}
