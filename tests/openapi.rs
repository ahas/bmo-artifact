//! The OpenAPI document (`docs/api.md`, "The wire"): the committed one is what the code generates, and it
//! states every route, event and tool the API document names, at the repository's version.

use crate::support;

use std::collections::BTreeSet;
use std::path::Path;

use bmo_artifact::openapi::{openapi, openapi_json};
use support::ROOT;

#[test]
fn the_committed_document_is_what_the_code_generates() {
  let committed = std::fs::read_to_string(Path::new(ROOT).join("docs/openapi.json"))
    .expect("docs/openapi.json is committed");

  assert!(
    committed == openapi_json(),
    "docs/openapi.json differs: write `bmo-artifact openapi` into it"
  );
}

#[test]
fn it_states_every_route_event_and_tool_at_the_repository_version() {
  let document = openapi();
  let routes: BTreeSet<String> = document["paths"]
    .as_object()
    .expect("paths")
    .iter()
    .flat_map(|(path, methods)| {
      methods
        .as_object()
        .expect("methods")
        .keys()
        .map(move |method| format!("{} {path}", method.to_uppercase()))
    })
    .collect();
  let wanted: BTreeSet<String> = [
    "GET /health",
    "GET /events",
    "GET /documents",
    "POST /documents",
    "GET /documents/{documentId}",
    "PATCH /documents/{documentId}",
    "DELETE /documents/{documentId}",
    "GET /documents/{documentId}/content",
    "GET /documents/{documentId}/picture",
    "GET /documents/{documentId}/attachments",
    "POST /documents/{documentId}/attachments",
    "GET /documents/{documentId}/attachments/{attachmentId}",
    "GET /documents/{documentId}/references",
    "GET /documents/{documentId}/references/{referenceId}",
    "GET /documents/{documentId}/references/{referenceId}/image",
    "GET /documents/{documentId}/members",
    "POST /documents/{documentId}/members",
    "PATCH /documents/{documentId}/members/{user}",
    "DELETE /documents/{documentId}/members/{user}",
    "GET /documents/{documentId}/units",
    "POST /documents/{documentId}/units",
    "DELETE /documents/{documentId}/units/{unit}",
    "GET /document-kinds",
    "GET /label-values",
    "GET /shares",
    "POST /shares",
    "POST /publications",
    "GET /shares/{token}",
    "DELETE /shares/{token}",
    "GET /shares/{token}/head",
    "GET /shares/{token}/document",
    "GET /shares/{token}/kind",
    "GET /shares/{token}/content",
    "GET /shares/{token}/picture",
    "GET /shares/{token}/attachments",
    "GET /shares/{token}/attachments/{attachmentId}",
    "GET /shares/{token}/references",
    "GET /shares/{token}/references/{referenceId}",
    "GET /shares/{token}/references/{referenceId}/image",
    "GET /readings/{token}/attachments/{attachmentId}",
    "GET /player/{token}",
    "POST /imports",
    "GET /imports/{importId}",
    "GET /scopes/{scope}",
    "DELETE /scopes/{scope}",
  ]
  .into_iter()
  .map(str::to_owned)
  .collect();

  assert_eq!(routes, wanted);

  let reach = |path: &str, method: &str| document["paths"][path][method]["x-console"].clone();

  assert_eq!(reach("/shares/{token}", "get"), "open");
  assert_eq!(reach("/shares/{token}/head", "get"), "open");
  assert_eq!(reach("/shares/{token}", "delete"), "screen");
  assert_eq!(reach("/documents/{documentId}/members", "post"), "screen");
  assert_eq!(reach("/documents/{documentId}/units", "post"), "screen");

  for read in bmo_artifact::http::share_reads::SHARE_READS {
    assert_eq!(reach(read, "get"), "open", "{read}");
  }
  assert_eq!(reach("/documents", "get"), "screen");
  assert_eq!(reach("/documents", "post"), "closed");
  assert_eq!(reach("/publications", "post"), "closed");
  assert_eq!(document["info"]["version"], bmo_artifact::VERSION);
  assert_eq!(document["x-calls"], serde_json::json!(["explorer", "web"]));
  assert_eq!(document["x-harness"]["plugin"], "bmo-artifact");
  assert_eq!(document["x-events"].as_array().map(Vec::len), Some(8));

  let tools: Vec<&str> = document["x-tools"]
    .as_array()
    .expect("tools")
    .iter()
    .filter_map(|tool| tool["name"].as_str())
    .collect();

  assert_eq!(
    tools,
    [
      "artifact_attach",
      "artifact_content",
      "artifact_document",
      "artifact_documents",
      "artifact_file",
      "artifact_import",
      "artifact_publish",
      "artifact_rewrite",
      "artifact_search",
      "artifact_share",
      "artifact_shares",
      "artifact_unshare",
    ]
  );
}
