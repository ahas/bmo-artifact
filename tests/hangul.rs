//! No Hangul in any text file of the repository: code, tests, settings and documents are English. The
//! console bundle's language pack, the folder its declaration names, holds the words a person reads in
//! each language, and is the one place another language stands.

use crate::support;

use std::fs;
use std::path::Path;

use support::{ROOT, files_under, shown};

/// The folder of the console bundle's words, as its declaration names it.
fn language_pack() -> std::path::PathBuf {
  let client = Path::new(ROOT).join("client");
  let declaration: serde_json::Value = serde_json::from_str(
    &fs::read_to_string(client.join("console-bundle.json")).expect("the declaration reads"),
  )
  .expect("the declaration is JSON");

  client.join(declaration["languagePack"].as_str().expect("the declaration names its language pack"))
}

/// Whether a character is Hangul: a syllable, a jamo or a compatibility jamo.
fn is_hangul(c: char) -> bool {
  matches!(c, '\u{AC00}'..='\u{D7A3}' | '\u{1100}'..='\u{11FF}' | '\u{3130}'..='\u{318F}' | '\u{A960}'..='\u{A97F}' | '\u{D7B0}'..='\u{D7FF}')
}

#[test]
fn no_text_file_holds_hangul() {
  let mut found = Vec::new();
  let pack = language_pack();

  for path in files_under(Path::new(ROOT)).into_iter().filter(|path| !path.starts_with(&pack)) {
    // A file that is not UTF-8 is not text.
    let Ok(text) = fs::read_to_string(&path) else { continue };

    for (at, line) in text.lines().enumerate() {
      if line.chars().any(is_hangul) {
        found.push(format!("{} line {}", shown(&path), at + 1));
      }
    }
  }

  assert!(found.is_empty(), "these lines hold Hangul: {found:?}");
}

#[test]
fn hangul_is_told_from_other_scripts() {
  assert!(is_hangul('\u{D55C}'));
  assert!(is_hangul('\u{3131}'));
  assert!(!is_hangul('a'));
  assert!(!is_hangul('\u{65E5}'));
}
