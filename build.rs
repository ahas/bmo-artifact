//! Embeds the repository's version into both binaries, and refuses a manifest that disagrees with it.

use std::fs;

fn main() {
  println!("cargo:rerun-if-changed=VERSION");

  let version = fs::read_to_string("VERSION").expect("VERSION stands at the repository's root");
  let version = version.trim();
  let manifest = env!("CARGO_PKG_VERSION");

  assert_eq!(
    version, manifest,
    "VERSION says {version} and Cargo.toml says {manifest}; they must agree"
  );

  println!("cargo:rustc-env=BMO_ARTIFACT_VERSION={version}");
}
