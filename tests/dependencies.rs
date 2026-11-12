//! Dependencies: no path out of the repository, and no other unit's package but the base libraries and the
//! clients of the units this service calls, in any manifest or as the first segment of a path in Rust
//! (`docs/layout.md`, "Dependency check").

use crate::support;

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_json::Value as Json;
use support::{ROOT, files_under, rust_files, shown};
use syn::visit::{self, Visit};
use toml::{Table, Value};

/// The only packages of other units this repository may depend on.
const BASE_LIBRARIES: [&str; 4] = ["bmo-chat", "bmo-chat-client", "bmo-console-face", "bmo-ui"];

/// The units this service calls, each reached through its published client crate taken from its checkout
/// beside this one (`docs/api.md`, "What this service calls").
const CALLED_UNITS: [&str; 1] = ["bmo-explorer"];

/// The packages taken from their checkouts beside this repository's, each by the path it is taken from: the
/// called units' clients and bmo-chat's console face.
const FROM_CHECKOUTS: [(&str, &str); 2] =
  [("bmo-explorer", "../explorer"), ("bmo-console-face", "../../base/chat/console-face")];

/// Where a package is taken from where it is taken from its checkout.
fn checkout_path(name: &str) -> Option<&'static str> {
  FROM_CHECKOUTS.iter().find(|(package, _)| *package == name).map(|(_, path)| *path)
}

/// This repository's own package, which its tests and binaries name.
const OWN_PACKAGE: &str = "bmo-artifact";

/// The dependency tables a Cargo manifest or a target section may carry.
const CARGO_TABLES: [&str; 5] = [
  "dependencies",
  "dev-dependencies",
  "build-dependencies",
  "dev_dependencies",
  "build_dependencies",
];

/// The dependency tables of a package.json.
const NPM_TABLES: [&str; 4] =
  ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"];

/// Whether a package name is another unit's: `bmo-` or the `@bmo/` scope, and not a base library.
fn is_other_unit(name: &str) -> bool {
  let unscoped = name.strip_prefix("@bmo/").map(|rest| format!("bmo-{rest}"));
  let name = unscoped.as_deref().unwrap_or(name);

  (name.starts_with("bmo-") || name == "bmo")
    && !BASE_LIBRARIES.contains(&name)
    && !CALLED_UNITS.contains(&name)
    && name != OWN_PACKAGE
}

/// `path` with `.` and `..` resolved by reading alone, so a path that does not exist still compares.
fn lexical(path: &Path) -> PathBuf {
  let mut out = PathBuf::new();

  for component in path.components() {
    match component {
      Component::CurDir => {}
      Component::ParentDir => {
        out.pop();
      }
      other => out.push(other),
    }
  }

  out
}

fn reaches_outside(root: &Path, manifest_dir: &Path, path: &str) -> bool {
  !lexical(&manifest_dir.join(path)).starts_with(root)
}

/// The dependency tables standing directly in one section of a Cargo manifest.
fn own(section: &Table) -> impl Iterator<Item = &Table> {
  CARGO_TABLES.iter().filter_map(|name| section.get(*name).and_then(Value::as_table))
}

/// Every dependency table in a Cargo manifest: the package's own, each target's, the workspace's shared
/// one, and each patch source's.
fn cargo_tables(manifest: &Table) -> Vec<&Table> {
  let mut tables: Vec<&Table> = own(manifest).collect();

  if let Some(targets) = manifest.get("target").and_then(Value::as_table) {
    for target in targets.values().filter_map(Value::as_table) {
      tables.extend(own(target));
    }
  }

  if let Some(shared) = manifest
    .get("workspace")
    .and_then(|workspace| workspace.get("dependencies"))
    .and_then(Value::as_table)
  {
    tables.push(shared);
  }

  if let Some(patches) = manifest.get("patch").and_then(Value::as_table) {
    tables.extend(patches.values().filter_map(Value::as_table));
  }

  tables
}

/// What breaks the rules among one Cargo manifest's dependencies.
fn cargo_violations(root: &Path, manifest_dir: &Path, manifest: &Table, shown_as: &str) -> Vec<String> {
  let mut found = Vec::new();

  for table in cargo_tables(manifest) {
    for (key, spec) in table {
      let detail = spec.as_table();
      let name = detail.and_then(|d| d.get("package")).and_then(Value::as_str).unwrap_or(key);
      let path = detail.and_then(|d| d.get("path")).and_then(Value::as_str);

      match path {
        Some(path) if checkout_path(name) == Some(path) => {}
        Some(path) if reaches_outside(root, manifest_dir, path) => {
          found.push(format!("{shown_as}: {key} reaches outside the repository through {path}"));
        }
        Some(_) => {}
        None if is_other_unit(name) => {
          found.push(format!("{shown_as}: {key} is another unit's package ({name})"))
        }
        None => {}
      }
    }
  }

  found
}

/// What breaks the rules among one package.json's dependencies.
fn npm_violations(root: &Path, manifest_dir: &Path, manifest: &Json, shown_as: &str) -> Vec<String> {
  let mut found = Vec::new();

  for table in NPM_TABLES.iter().filter_map(|name| manifest.get(*name).and_then(Json::as_object)) {
    for (name, spec) in table {
      let spec = spec.as_str().unwrap_or_default();
      let alias = spec
        .strip_prefix("npm:")
        .map(|target| target.rsplit_once('@').map_or(target, |(n, _)| n));
      let path = spec.strip_prefix("file:").or_else(|| spec.strip_prefix("link:"));

      if let Some(path) = path.filter(|path| reaches_outside(root, manifest_dir, path)) {
        found.push(format!("{shown_as}: {name} reaches outside the repository through {path}"));
      } else if is_other_unit(name) || alias.is_some_and(is_other_unit) {
        found.push(format!("{shown_as}: {name} is another unit's package ({spec})"));
      }
    }
  }

  found
}

/// The crates a Rust file names as the first segment of a path: its `use` roots, its `extern crate`
/// lines and every path of two or more segments.
#[derive(Default)]
struct CrateRoots(Vec<String>);

impl CrateRoots {
  fn of_tree(&mut self, tree: &syn::UseTree) {
    match tree {
      syn::UseTree::Path(path) => self.0.push(path.ident.to_string()),
      syn::UseTree::Name(name) => self.0.push(name.ident.to_string()),
      syn::UseTree::Rename(rename) => self.0.push(rename.ident.to_string()),
      syn::UseTree::Glob(_) => {}
      syn::UseTree::Group(group) => group.items.iter().for_each(|tree| self.of_tree(tree)),
    }
  }
}

impl<'ast> Visit<'ast> for CrateRoots {
  fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
    self.of_tree(&item.tree);
  }

  fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
    self.0.push(item.ident.to_string());
  }

  fn visit_path(&mut self, path: &'ast syn::Path) {
    if path.segments.len() > 1 {
      self.0.push(path.segments[0].ident.to_string());
    }

    visit::visit_path(self, path);
  }
}

/// The other units' crates one Rust file names.
fn rust_violations(source: &str, shown_as: &str) -> Vec<String> {
  let file = syn::parse_file(source).unwrap_or_else(|error| panic!("{shown_as} reads as Rust: {error}"));
  let mut roots = CrateRoots::default();

  roots.visit_file(&file);
  roots.0.sort();
  roots.0.dedup();
  roots
    .0
    .into_iter()
    .filter(|root| is_other_unit(&root.replace('_', "-")))
    .map(|root| format!("{shown_as}: names another unit's crate {root}"))
    .collect()
}

#[test]
fn manifests_name_no_other_unit_and_stay_in_the_repository() {
  let root = lexical(Path::new(ROOT));
  let mut found = Vec::new();

  for path in files_under(&root) {
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    let dir = path.parent().expect("a manifest stands in a directory").to_path_buf();

    match name {
      "Cargo.toml" => {
        let text = fs::read_to_string(&path).expect("a manifest is readable");
        let manifest: Table =
          text.parse().unwrap_or_else(|error| panic!("{} reads: {error}", shown(&path)));
        found.extend(cargo_violations(&root, &dir, &manifest, &shown(&path)));
      }
      "package.json" => {
        let text = fs::read_to_string(&path).expect("a manifest is readable");
        let manifest: Json =
          serde_json::from_str(&text).unwrap_or_else(|error| panic!("{} reads: {error}", shown(&path)));
        found.extend(npm_violations(&root, &dir, &manifest, &shown(&path)));
      }
      _ => {}
    }
  }

  assert!(found.is_empty(), "these dependencies break the rules: {found:?}");
}

// The console plugin bundle's manifest is one of those the check above reads. It takes bmo-ui for its own install
// from a file inside the repository.
#[test]
fn the_clients_manifest_is_read_and_takes_bmo_ui_from_inside() {
  let root = lexical(Path::new(ROOT));
  let path = root.join("client").join("package.json");

  assert!(files_under(&root).contains(&path), "{} is not among the files read", shown(&path));

  let manifest: Json = serde_json::from_str(&fs::read_to_string(&path).expect("the manifest is readable"))
    .expect("the manifest reads");

  let spec = manifest["devDependencies"]["bmo-ui"]
    .as_str()
    .expect("bmo-ui is a development dependency");
  let file = spec.strip_prefix("file:").expect("bmo-ui is taken from a file");

  assert!(!reaches_outside(&root, &root.join("client"), file), "{spec}");
  assert!(root.join("client").join(file).is_file(), "{spec} names no file");
  assert_eq!(
    npm_violations(&root, &root.join("client"), &manifest, "client/package.json"),
    Vec::<String>::new()
  );
}

#[test]
fn rust_files_name_no_other_unit() {
  let found: Vec<String> = rust_files(Path::new(ROOT))
    .into_iter()
    .flat_map(|path| {
      let source = fs::read_to_string(&path).expect("a Rust file is readable");
      rust_violations(&source, &shown(&path))
    })
    .collect();

  assert!(found.is_empty(), "these Rust files name other units: {found:?}");
}

#[test]
fn a_cargo_manifest_breaking_the_rules_is_named() {
  let manifest: Table = r#"
    [dependencies]
    serde = "1"
    bmo-chat = { version = "0.1" }
    bmo-agency = { git = "https://example.invalid/agency" }
    renamed = { package = "bmo-receiver", version = "0.1" }
    inside = { path = "crates/inside" }
    outside = { path = "../../elsewhere" }

    [target.'cfg(unix)'.dev-dependencies]
    bmo-ui = "0.1"
    bmo-brain = "0.1"

    [workspace.dependencies]
    bmo-testbed = "0.1"
  "#
  .parse()
  .expect("the sample reads");
  let root = Path::new("/work/repository");
  let mut found = cargo_violations(root, root, &manifest, "sample");

  found.sort();
  assert_eq!(
    found,
    [
      "sample: bmo-agency is another unit's package (bmo-agency)",
      "sample: bmo-brain is another unit's package (bmo-brain)",
      "sample: bmo-testbed is another unit's package (bmo-testbed)",
      "sample: outside reaches outside the repository through ../../elsewhere",
      "sample: renamed is another unit's package (bmo-receiver)",
    ]
  );
}

#[test]
fn a_package_json_breaking_the_rules_is_named() {
  let manifest: Json = serde_json::json!({
    "dependencies": { "vue": "^3", "bmo-ui": "^0.1", "@bmo/web": "^0.1", "near": "file:./near" },
    "devDependencies": { "far": "file:../../far", "shade": "npm:bmo-job@0.1" },
  });
  let root = Path::new("/work/repository");
  let mut found = npm_violations(root, &root.join("client"), &manifest, "sample");

  found.sort();
  assert_eq!(
    found,
    [
      "sample: @bmo/web is another unit's package (^0.1)",
      "sample: far reaches outside the repository through ../../far",
      "sample: shade is another unit's package (npm:bmo-job@0.1)",
    ]
  );
}

#[test]
fn a_rust_file_naming_another_unit_is_named() {
  let source = r#"
    use bmo_chat::Piece;
    use {serde::Deserialize, bmo_policy::fill_marks};
    extern crate bmo_lang;
    use bmo_artifact::settings::Settings;

    fn f() {
      let _ = bmo_tool::cap_items(1);
      let _ = bmo_ui_kit::x();
      let local = 1;
    }
  "#;
  let found = rust_violations(source, "sample");

  assert_eq!(
    found,
    [
      "sample: names another unit's crate bmo_lang",
      "sample: names another unit's crate bmo_policy",
      "sample: names another unit's crate bmo_tool",
      "sample: names another unit's crate bmo_ui_kit",
    ]
  );
}
