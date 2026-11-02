//! The layout: no folder named after its parent, Cargo's manifest the only build manifest at the root,
//! and no test standing in `src/` (`docs/layout.md`).

use crate::support;

use std::fs;
use std::path::Path;

use support::{ROOT, dirs_under, rust_files, shown};
use syn::visit::{self, Visit};

/// The manifests and lockfiles of other build tools, none of which may stand at the root.
const OTHER_MANIFESTS: [&str; 11] = [
  "package.json",
  "bun.lock",
  "bun.lockb",
  "bunfig.toml",
  "package-lock.json",
  "yarn.lock",
  "pnpm-lock.yaml",
  "pnpm-workspace.yaml",
  "deno.json",
  "pyproject.toml",
  "go.mod",
];

#[test]
fn no_folder_is_named_after_its_parent() {
  let found: Vec<String> = dirs_under(Path::new(ROOT))
    .into_iter()
    .filter(|dir| dir.file_name() == dir.parent().and_then(Path::file_name))
    .map(|dir| shown(&dir))
    .collect();

  assert!(found.is_empty(), "these folders repeat their parent's name: {found:?}");
}

#[test]
fn cargo_is_the_only_manifest_at_the_root() {
  let root = Path::new(ROOT);
  let found: Vec<&str> = OTHER_MANIFESTS.into_iter().filter(|name| root.join(name).exists()).collect();

  assert!(root.join("Cargo.toml").is_file(), "Cargo.toml stands at the root");
  assert!(found.is_empty(), "these manifests stand at the root: {found:?}");
}

/// The test attributes a Rust file carries: `#[test]` and `#[cfg(test)]`.
#[derive(Default)]
struct Tests(usize);

impl<'ast> Visit<'ast> for Tests {
  fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
    let path = attribute.path();
    let cfg_test =
      path.is_ident("cfg") && attribute.parse_args::<syn::Path>().is_ok_and(|arg| arg.is_ident("test"));

    if path.is_ident("test") || cfg_test {
      self.0 += 1;
    }

    visit::visit_attribute(self, attribute);
  }
}

fn tests_in(source: &str) -> usize {
  let mut tests = Tests::default();

  tests.visit_file(&syn::parse_file(source).expect("the source reads as Rust"));
  tests.0
}

#[test]
fn no_test_stands_in_src() {
  let found: Vec<String> = rust_files(&Path::new(ROOT).join("src"))
    .into_iter()
    .filter(|path| tests_in(&fs::read_to_string(path).expect("a Rust file is readable")) > 0)
    .map(|path| shown(&path))
    .collect();

  assert!(found.is_empty(), "these files under src/ hold tests: {found:?}");
}

#[test]
fn a_test_module_is_seen() {
  let source = "#[cfg(test)] mod tests { #[test] fn one() {} }\n#[cfg(unix)] fn two() {}";

  assert_eq!(tests_in(source), 2);
}
