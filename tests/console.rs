//! The console face as this service stands it up (`docs/api.md`, "The console"): the manifest it serves read
//! through bmo-chat's face reading over the bmo-ui release the bundle vendors, the face's guard laid over the
//! screens' routes, and how the console probe reaches the service. The face's own guard and paths are the
//! face's tests'.

use crate::support;

use std::collections::HashMap;
use std::path::Path;

use axum::http::{Method, StatusCode};
use bmo_artifact::console;
use bmo_artifact::http::share_reads::SHARE_READS;
use bmo_artifact::settings::{Console, Settings};
use bmo_console_face::read_manifest;
use serde_json::json;
use support::ROOT;
use support::service::{
  CONSOLE_TOKEN, MODULE, STYLESHEET, Service, TOKEN, WORDS, built_bundle, release_layer, vendored_release,
};

#[tokio::test]
async fn the_manifest_served_keeps_the_contract_and_names_the_vendored_layer() {
  let service = Service::new().await;
  let answered = service.call_by(Some(CONSOLE_TOKEN), Method::GET, "/console-manifest", None, &[]).await;

  assert_eq!(answered.status, StatusCode::OK, "{}", answered.json());

  let read = read_manifest(&answered.json(), &vendored_release()).expect("the manifest keeps the contract");

  assert_eq!(read["layer"], release_layer());
  assert_eq!(
    (read["name"].clone(), read["serviceName"].clone(), read["title"].clone()),
    (json!("artifact"), json!("artifact"), json!("artifact.title.screen"))
  );
  assert_eq!(
    (read["screenPath"].clone(), read["formerScreenPaths"].clone()),
    (json!("/artifact"), json!(["/archive"]))
  );
  assert_eq!((read["role"].clone(), read["ownerKey"].clone()), (json!("member"), json!(false)));
  assert_eq!(read["apiPrefix"], "/");
  assert_eq!(read["shareReads"], json!(SHARE_READS));
  assert_eq!(read["shareHead"], "/shares/{token}/head");
  assert_eq!(read["calls"], json!([]));
}

// The services the screens call through the console are the deployment's to name (docs/api.md, "The console").
#[test]
fn the_manifest_names_the_calls_the_settings_name() {
  let folder = tempfile::tempdir().expect("a folder");

  for (name, bytes) in built_bundle() {
    std::fs::write(folder.path().join(name), bytes).expect("a file");
  }

  let face = console::face(&Console {
    token: CONSOLE_TOKEN.to_owned(),
    bundle: folder.path().to_path_buf(),
    calls: vec!["gate".to_owned()],
  })
  .expect("the face stands")
  .expect("a face");

  assert_eq!(face.manifest()["calls"], json!(["gate"]));
}

#[test]
fn no_face_stands_without_a_console_token_or_over_a_bundle_naming_no_layer() {
  let folder = tempfile::tempdir().expect("a folder");

  for (name, bytes) in [("bundle.js", MODULE), ("bundle.css", STYLESHEET), ("words.json", WORDS)] {
    std::fs::write(folder.path().join(name), bytes).expect("a file");
  }

  let console = |token: &str| Console {
    token: token.to_owned(),
    bundle: folder.path().to_path_buf(),
    calls: Vec::new(),
  };

  assert!(matches!(console::face(&console("")), Ok(None)));
  assert!(console::face(&console(CONSOLE_TOKEN)).is_err());
}

// Where no face stands the console is left with nothing, so it leaves the service out.
#[tokio::test]
async fn where_no_face_stands_the_console_token_reaches_nothing() {
  let service = Service::with_console(&[("bundle.js", MODULE), ("words.json", WORDS)]).await;

  for path in ["/console-manifest", "/documents", "/shares/unknown"] {
    let refused = service
      .call_by(Some(CONSOLE_TOKEN), Method::GET, path, None, &[("owner-role", "member")])
      .await;

    assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{path}");
  }

  let answered = service.call_by(Some(TOKEN), Method::GET, "/documents", None, &[]).await;

  assert_eq!(answered.status, StatusCode::OK);
}

#[tokio::test]
async fn the_console_reaches_the_screens_routes_only_under_a_role_allowing_member() {
  let service = Service::new().await;

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

    let answered = service.call_by(Some(TOKEN), Method::GET, path, None, &[]).await;

    assert_eq!(answered.status, StatusCode::OK, "{path} with the service's token");
  }
}

#[tokio::test]
async fn the_console_reaches_no_route_the_screens_do_not_use() {
  let service = Service::new().await;
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

#[test]
fn the_probe_reaches_the_loopback_address_where_the_service_listens_on_every_one() {
  let environment = HashMap::from([
    ("BMO_ARTIFACT_SERVER__LISTEN".to_owned(), "0.0.0.0:3230".to_owned()),
    ("BMO_ARTIFACT_SERVER__TOKEN".to_owned(), TOKEN.to_owned()),
    ("BMO_ARTIFACT_CONSOLE__TOKEN".to_owned(), CONSOLE_TOKEN.to_owned()),
  ]);
  let settings = Settings::read(Path::new(ROOT), Some(environment)).expect("the settings read");

  assert_eq!(
    console::reach(&settings),
    json!({ "address": "http://127.0.0.1:3230", "consoleToken": CONSOLE_TOKEN, "token": TOKEN })
  );
}
