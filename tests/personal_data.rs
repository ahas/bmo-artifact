//! No tracked file holds personal data: a mail address outside the reserved example domains, a person's home
//! path, a telephone number, a token or a private network address (`PERSONAL-DATA.md`). A finding names the
//! file, the line and the kind, never the value.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use regex::Regex;

/// The home folders a neutral example writes, so a person's own is not among them.
const NEUTRAL_HOME: [&str; 5] = ["user", "tester", "qa-admin", "placeholder", "example"];

fn root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every file git tracks or would track: the committed ones and the new ones it does not ignore.
fn tracked() -> Vec<String> {
  let output = Command::new("git")
    .args(["ls-files", "--cached", "--others", "--exclude-standard", "-z"])
    .current_dir(root())
    .stdin(Stdio::null())
    .output()
    .expect("git lists the files");

  assert!(
    output.status.success(),
    "git ls-files failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );

  String::from_utf8(output.stdout)
    .expect("the paths are text")
    .split('\0')
    .filter(|path| !path.is_empty() && root().join(path).is_file())
    .map(str::to_owned)
    .collect()
}

/// The tracked files that read as text, with their text.
fn tracked_texts() -> Vec<(String, String)> {
  tracked()
    .into_iter()
    .filter_map(|path| std::fs::read_to_string(root().join(&path)).ok().map(|text| (path, text)))
    .collect()
}

/// The line a byte offset stands on, counted from one.
fn line_of(text: &str, at: usize) -> usize {
  text[..at].bytes().filter(|byte| *byte == b'\n').count() + 1
}

/// Notes a finding by the file, the line and the kind, never the value.
fn note(found: &mut Vec<String>, path: &str, text: &str, kind: &str, at: usize) {
  found.push(format!("{path}:{}: {kind}", line_of(text, at)));
}

/// Whether a home path's own folder is a neutral example.
fn is_neutral_home(folder: &str) -> bool {
  NEUTRAL_HOME.contains(&folder) || folder.starts_with("agent-")
}

/// Whether a mail address's domain is the reserved example space, where a neutral example address stands.
fn is_example_address(address: &str) -> bool {
  address.rsplit_once('@').is_some_and(|(_, domain)| is_example_domain(domain))
}

/// Whether a domain is the reserved example space.
fn is_example_domain(domain: &str) -> bool {
  let domain = domain.to_lowercase();

  ["example.com", "example.net", "example.org"]
    .iter()
    .any(|example| domain == *example || domain.ends_with(&format!(".{example}")))
    || [".example", ".test", ".invalid", ".localhost"]
      .iter()
      .any(|suffix| domain.ends_with(suffix))
}

#[test]
fn no_personal_data_in_a_tracked_file() {
  let mail = Regex::new(concat!(r"\b[A-Za-z0-9._%+-]+@", r"[A-Za-z][A-Za-z0-9.-]*\.[A-Za-z]{2,}\b"))
    .expect("the mail pattern reads");
  let home = Regex::new(r"(?:/home/|/Users/|[A-Za-z]:\\Users\\)([A-Za-z0-9._-]+)")
    .expect("the home path pattern reads");
  let telephone = Regex::new(concat!(
    r"\b01[016789][-. ]?\d{3,4}[-. ]?\d{4}\b|",
    r"\+\d{1,3}[-. ]?\d{2,4}[-. ]?\d{3,4}[-. ]?\d{4}\b|",
    r"\+\d{9,15}\b"
  ))
  .expect("the telephone pattern reads");
  let token = Regex::new(concat!(
    r"gh[pousr]_[A-Za-z0-9]{20,}|",
    r"github_pat_[A-Za-z0-9_]{20,}|",
    r"sk-[A-Za-z0-9_-]{20,}|",
    r"xox[baprs]-[A-Za-z0-9-]{10,}|",
    r"AKIA[0-9A-Z]{16}|",
    r"AIza[0-9A-Za-z_-]{35}"
  ))
  .expect("the token pattern reads");
  let private = Regex::new(concat!(
    r"(?i)\b(?:10\.\d{1,3}\.\d{1,3}\.\d{1,3}|",
    r"192\.168\.\d{1,3}\.\d{1,3}|",
    r"172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3})\b|",
    r"\b(?:f[cd][0-9a-f]{2}|fe80):"
  ))
  .expect("the private address pattern reads");
  let mut found = Vec::new();

  for (path, text) in tracked_texts() {
    for address in mail.find_iter(&text).filter(|address| !is_example_address(address.as_str())) {
      note(&mut found, &path, &text, "mail address", address.start());
    }

    for capture in home.captures_iter(&text) {
      let neutral = capture.get(1).is_some_and(|folder| is_neutral_home(folder.as_str()));

      if !neutral {
        note(&mut found, &path, &text, "home path", capture.get(0).expect("a match").start());
      }
    }

    for number in telephone.find_iter(&text) {
      note(&mut found, &path, &text, "telephone number", number.start());
    }

    for token in token.find_iter(&text) {
      note(&mut found, &path, &text, "token", token.start());
    }

    for address in private.find_iter(&text) {
      note(&mut found, &path, &text, "private network address", address.start());
    }
  }

  found.sort();
  found.dedup();
  assert!(found.is_empty(), "personal data in tracked files: {found:#?}");
}

#[test]
fn the_neutral_forms_and_the_lines_are_read() {
  assert!(is_example_address("tester@example.com"));
  assert!(is_example_address("qa-admin@mail.example.org"));
  assert!(is_example_domain("host.test"));
  assert!(is_example_domain("mail.example.invalid"));
  assert!(is_neutral_home("tester"));
  assert!(is_neutral_home("agent-a"));
  assert!(!is_example_domain("corp.internal"));
  assert!(!is_neutral_home("someone"));

  let elsewhere = format!("tester@{}", "corp.internal");

  assert!(!is_example_address(&elsewhere));
  assert_eq!(line_of("one\ntwo\nthree", 4), 2);
}
