//! The console bundle as a console reaches it (`docs/api.md`, "The console"): the manifest read through
//! bmo-ui's own reading of the contract and agreeing with the declaration the layer carries, the module's path
//! following its bytes, and the guard each kind of route stands behind.

use crate::support;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use axum::http::{Method, StatusCode};
use bmo_artifact::console::ConsoleBundle;
use serde_json::{Value, json};
use support::ROOT;
use support::service::{CONSOLE_TOKEN, Service, TOKEN};

const MODULE: &[u8] = b"export default { routes: [] };\n";
const STYLESHEET: &[u8] = b".artifact-paper { color: red; }\n";
const WORDS: &[u8] = br#"{"en":{"artifact":{"title":{"screen":"Artifact"}}},"ko":{"artifact":{"title":{"screen":"Artifact"}}}}"#;

async fn built() -> Service {
  Service::with_console(&[("bundle.js", MODULE), ("bundle.css", STYLESHEET), ("words.json", WORDS)]).await
}

fn folder_with(files: &[(&str, &[u8])]) -> tempfile::TempDir {
  let folder = tempfile::tempdir().expect("a folder");

  for (name, bytes) in files {
    std::fs::write(folder.path().join(name), bytes).expect("a file");
  }

  folder
}

// The manifest read by bmo-ui's `consoleManifest`, run by Bun in the bundle's own install: the manifest
// read, or the reading's fault.
fn read_by_contract(manifest: &Value) -> Result<Value, String> {
  let mut reading = Command::new("bun")
    .args(["tests/read-manifest.ts"])
    .current_dir(Path::new(ROOT).join("client"))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("bun runs in client/");

  reading
    .stdin
    .take()
    .expect("its input")
    .write_all(manifest.to_string().as_bytes())
    .expect("the manifest is handed over");

  let read = reading.wait_with_output().expect("the reading ends");

  if read.status.success() {
    serde_json::from_slice(&read.stdout).map_err(|why| why.to_string())
  } else {
    Err(String::from_utf8_lossy(&read.stderr).into_owned())
  }
}

#[tokio::test]
async fn the_manifest_keeps_the_contract_and_agrees_with_the_declaration() {
  let service = built().await;
  let answered = service.call_by(Some(CONSOLE_TOKEN), Method::GET, "/console-manifest", None, &[]).await;

  assert_eq!(answered.status, StatusCode::OK, "{}", answered.json());

  let manifest = answered.json();
  let read = read_by_contract(&manifest).expect("the manifest keeps the contract");

  assert_eq!(read["role"], "member");
  assert_eq!(read["ownerKey"], false);
  assert_eq!(read["apiPrefix"], "/");
  assert_eq!(read["words"], "/console/words");
  assert_eq!(
    (read["screenPath"].clone(), read["formerScreenPaths"].clone()),
    (json!("/artifact"), json!(["/archive"]))
  );
  assert_eq!(read["shareReads"], manifest["shareReads"]);
  assert!(
    read["shareReads"]
      .as_array()
      .is_some_and(|reads| reads.contains(&json!("/shares/{token}/document"))),
    "{read}"
  );

  let declaration: Value = serde_json::from_str(
    &std::fs::read_to_string(Path::new(ROOT).join("client/console-bundle.json")).expect("the declaration"),
  )
  .expect("the declaration reads");

  for field in [
    "name",
    "title",
    "icon",
    "screenPath",
    "formerScreenPaths",
    "serviceName",
    "events",
    "websocket",
    "pushKinds",
  ] {
    assert_eq!(manifest[field], declaration[field], "{field}");
  }

  let bundle = manifest["bundle"].as_str().expect("the module's path");
  let stylesheet = manifest["stylesheet"].as_str().expect("the stylesheet's path");
  // A reader with no sign-in loads them to open a share page, so they are answered with no role.
  let module = service.call_by(Some(CONSOLE_TOKEN), Method::GET, bundle, None, &[]).await;
  let sheet = service.call_by(Some(CONSOLE_TOKEN), Method::GET, stylesheet, None, &[]).await;
  let words = service.call_by(Some(CONSOLE_TOKEN), Method::GET, "/console/words", None, &[]).await;

  assert_eq!((module.status, module.bytes.as_slice()), (StatusCode::OK, MODULE));
  assert!(module.header("content-type").starts_with("text/javascript"));
  assert!(module.header("cache-control").contains("immutable"));
  assert_eq!((sheet.status, sheet.bytes.as_slice()), (StatusCode::OK, STYLESHEET));
  assert!(sheet.header("content-type").starts_with("text/css"));
  assert_eq!(
    (words.status, words.json()),
    (StatusCode::OK, serde_json::from_slice::<Value>(WORDS).expect("words"))
  );
}

#[test]
fn a_manifest_out_of_the_contract_is_refused_by_its_reading() {
  let fault = read_by_contract(&json!({ "name": "artifact" })).expect_err("a manifest lacking its fields");

  assert!(fault.contains("role"), "{fault}");
}

#[test]
fn the_module_path_follows_its_bytes() {
  let one = ConsoleBundle::read(folder_with(&[("bundle.js", MODULE), ("words.json", WORDS)]).path());
  let same = ConsoleBundle::read(folder_with(&[("bundle.js", MODULE), ("words.json", WORDS)]).path());
  let other = ConsoleBundle::read(
    folder_with(&[("bundle.js", b"export default { routes: [1] };\n"), ("words.json", WORDS)]).path(),
  );
  let path = |bundle: &ConsoleBundle| bundle.manifest().expect("a manifest").bundle;

  assert_eq!(path(&one), path(&same));
  assert_ne!(path(&one), path(&other));
  assert!(
    path(&one).starts_with("/console/bundle.") && path(&one).ends_with(".js"),
    "{}",
    path(&one)
  );
  assert_eq!(one.manifest().expect("a manifest").stylesheet, None);
}

#[test]
fn a_folder_with_no_module_or_no_words_holds_no_bundle() {
  assert_eq!(ConsoleBundle::read(folder_with(&[("words.json", WORDS)]).path()).manifest(), None);
  assert_eq!(ConsoleBundle::read(folder_with(&[("bundle.js", MODULE)]).path()).manifest(), None);
}

#[tokio::test]
async fn an_unbuilt_bundle_leaves_the_manifest_and_the_words_not_found() {
  let service = Service::new().await;

  for path in ["/console-manifest", "/console/words"] {
    let answered = service.call_by(Some(CONSOLE_TOKEN), Method::GET, path, None, &[]).await;

    assert_eq!(
      (answered.status, answered.json()["code"].clone()),
      (StatusCode::NOT_FOUND, json!("not-found")),
      "{path}"
    );
  }
}

#[tokio::test]
async fn a_call_without_a_token_is_refused_401() {
  let service = built().await;
  let bundle = service.call(Method::GET, "/console-manifest", None).await.json()["bundle"]
    .as_str()
    .expect("a path")
    .to_owned();

  for path in ["/console-manifest", "/console/words", bundle.as_str(), "/documents"] {
    let answered = service.call_by(None, Method::GET, path, None, &[("owner-role", "member")]).await;

    assert_eq!(answered.status, StatusCode::UNAUTHORIZED, "{path}");
    assert_eq!(answered.json()["code"], "unauthorized", "{path}");

    let wrong = service
      .call_by(Some("wrong"), Method::GET, path, None, &[("owner-role", "member")])
      .await;

    assert_eq!(wrong.status, StatusCode::UNAUTHORIZED, "{path}");
  }
}

#[tokio::test]
async fn the_console_reaches_the_screens_routes_only_under_a_role_allowing_member() {
  let service = built().await;

  for path in ["/documents", "/document-kinds", "/label-values?key=category", "/shares"] {
    for headers in [&[][..], &[("owner-role", "guest")][..], &[("owner-role", "")][..]] {
      let refused = service.call_by(Some(CONSOLE_TOKEN), Method::GET, path, None, headers).await;

      assert_eq!(refused.status, StatusCode::FORBIDDEN, "{path} {headers:?}");
      assert_eq!(refused.json()["code"], "forbidden", "{path} {headers:?}");
    }

    for role in ["member", "administrator"] {
      let answered = service
        .call_by(Some(CONSOLE_TOKEN), Method::GET, path, None, &[("owner-role", role)])
        .await;

      assert_eq!(answered.status, StatusCode::OK, "{path} {role}");
    }
  }
}

#[tokio::test]
async fn the_console_reaches_no_route_the_screens_do_not_use() {
  let service = built().await;
  let member = [("owner-role", "administrator"), ("owner-key", "7")];
  let filed = service
    .call_by(
      Some(CONSOLE_TOKEN),
      Method::POST,
      "/documents",
      Some(
        json!({ "kind": "record", "title": "Kept", "body": "Words.", "actorName": "tester", "labels": {} }),
      ),
      &member,
    )
    .await;

  assert_eq!(
    (filed.status, filed.json()["code"].clone()),
    (StatusCode::FORBIDDEN, json!("forbidden"))
  );

  for path in ["/events", "/health", "/scopes/team-a", "/imports/1"] {
    let refused = service.call_by(Some(CONSOLE_TOKEN), Method::GET, path, None, &member).await;

    assert_eq!(refused.status, StatusCode::FORBIDDEN, "{path}");
  }
}

#[tokio::test]
async fn the_service_token_reaches_every_route_with_no_role() {
  let service = built().await;
  let bundle = service.call(Method::GET, "/console-manifest", None).await.json()["bundle"]
    .as_str()
    .expect("a path")
    .to_owned();

  for path in [bundle.as_str(), "/documents", "/console/words"] {
    let answered = service.call_by(Some(TOKEN), Method::GET, path, None, &[]).await;

    assert_eq!(answered.status, StatusCode::OK, "{path}");
  }
}

#[tokio::test]
async fn a_file_the_manifest_does_not_name_is_not_found() {
  let service = built().await;

  for path in ["/console/bundle.js", "/console/bundle.0000000000000000.js", "/console/words.json"] {
    let answered = service
      .call_by(Some(CONSOLE_TOKEN), Method::GET, path, None, &[("owner-role", "member")])
      .await;

    assert_eq!(answered.status, StatusCode::NOT_FOUND, "{path}");
  }
}
