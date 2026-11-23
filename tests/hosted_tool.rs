//! The tool a tool host places (`docs/api.md`, "The tool a tool host places"): its manifest holds one entry for
//! each harness tool, by the same name, asking the same capability, its schema files equal to what the tools
//! generate; and its executable, handed a call as the tool host hands it, acts for the record's `ownerAccount`,
//! refuses a record naming none and a file of the caller's machine, and reads the service's settings from the
//! data root.

use crate::support;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use bmo_artifact::mcp::hosted::{CAPABILITY, DATA_ROOT_VARIABLE, LOCAL_SETTINGS, NAMESPACE, entries};
use serde_json::{Value, json};
use support::ROOT;
use support::service::{Service, TOKEN};
use toml::Table;

/// The tool's folder.
fn folder() -> PathBuf {
  Path::new(ROOT).join("tools").join("artifact")
}

fn manifest() -> Table {
  std::fs::read_to_string(folder().join("tool.toml"))
    .expect("the manifest stands")
    .parse()
    .expect("the manifest reads as TOML")
}

fn schema_file(name: &str) -> Value {
  let text = std::fs::read_to_string(folder().join(name)).unwrap_or_else(|why| panic!("{name}: {why}"));

  serde_json::from_str(&text).unwrap_or_else(|why| panic!("{name} reads as JSON: {why}"))
}

#[test]
fn the_manifest_names_the_tool_its_executable_runs_as_one_process_a_call() {
  let manifest = manifest();
  let tool = manifest["tool"].as_table().expect("a [tool] table");

  assert_eq!(tool["name"].as_str(), Some("artifact"));
  assert_eq!(tool["protocol"].as_integer(), Some(1));
  assert_eq!(tool["mode"].as_str(), Some("call"));
  assert_eq!(tool["run"].as_array().map(|run| run.len()), Some(1));
  assert_eq!(tool["run"][0].as_str(), Some("artifact-tool"));
}

#[test]
fn the_manifest_holds_one_entry_for_each_harness_tool_with_the_schemas_it_generates() {
  let manifest = manifest();
  let declared = manifest["entry"].as_array().expect("[[entry]] tables");
  let served = entries();

  assert_eq!(
    declared
      .iter()
      .map(|entry| entry["name"].as_str().unwrap_or_default())
      .collect::<Vec<_>>(),
    served.iter().map(|entry| entry.name).collect::<Vec<_>>(),
    "the entries are the harness tools, by name"
  );

  for (entry, tool) in declared.iter().zip(&served) {
    let name = tool.name;
    let read_only = entry
      .get("hints")
      .and_then(|hints| hints.get("read_only"))
      .and_then(|hint| hint.as_bool());

    assert_eq!(entry["namespace"].as_str(), Some(NAMESPACE), "{name}");
    assert_eq!(
      entry["capabilities"]
        .as_array()
        .map(|asked| asked.iter().filter_map(|one| one.as_str()).collect::<Vec<_>>()),
      Some(vec![CAPABILITY]),
      "{name}"
    );
    assert_eq!(entry["description"].as_str(), Some(tool.description.as_str()), "{name}");
    assert_eq!(read_only.unwrap_or(false), tool.read_only, "{name}");
    assert_eq!(entry["input"].as_str(), Some(format!("{name}.json").as_str()), "{name}");
    assert_eq!(entry["output"].as_str(), Some(format!("{name}.output.json").as_str()), "{name}");
    assert_eq!(
      schema_file(&format!("{name}.json")),
      Value::Object(tool.input.clone()),
      "{name}'s input"
    );
    assert_eq!(
      schema_file(&format!("{name}.output.json")),
      Value::Object(tool.output.clone()),
      "{name}'s output"
    );
  }
}

#[test]
fn an_argument_read_with_a_default_is_optional_in_an_entrys_input() {
  let publish = entries()
    .into_iter()
    .find(|entry| entry.name == "artifact_publish")
    .expect("artifact_publish");

  assert_eq!(publish.input["required"], json!(["title", "actorName"]), "{:?}", publish.input);
}

#[test]
fn the_publish_entry_says_it_reads_no_file_of_the_callers_machine() {
  let publish = entries()
    .into_iter()
    .find(|entry| entry.name == "artifact_publish")
    .expect("artifact_publish");

  assert!(publish.description.contains("`path` is refused"), "{}", publish.description);
}

/// A data root whose local settings name the service at `listen` and its token.
fn data_root(listen: &str) -> tempfile::TempDir {
  let root = tempfile::tempdir().expect("a data root");
  let settings = root.path().join(LOCAL_SETTINGS);

  std::fs::create_dir_all(settings.parent().expect("env/")).expect("env/ is made");
  std::fs::write(&settings, format!("[server]\nlisten = \"{listen}\"\ntoken = \"{TOKEN}\"\n"))
    .expect("the local settings are written");
  root
}

/// The session record a call is handed, acting for `owner` where one is named.
fn record(owner: Option<&str>) -> Value {
  let mut record = json!({
    "holderName": "tester",
    "namespaces": [NAMESPACE],
    "capabilities": [CAPABILITY],
    "grants": [],
  });

  if let Some(owner) = owner {
    record["ownerAccount"] = json!(owner);
  }

  record
}

/// The tool's executable handed one call as the tool host hands it, in a clean environment holding the data root
/// alone and started in a folder holding the settings file; its exit and the tool result it wrote.
fn called(root: Option<&Path>, entry: &str, arguments: Value, session: Value) -> Value {
  let handed = json!({
    "protocol": 1,
    "kind": "call",
    "entry": format!("{NAMESPACE}/{entry}"),
    "arguments": arguments,
    "session": session,
  });
  let mut command = Command::new(env!("CARGO_BIN_EXE_artifact-tool"));

  command
    .current_dir(ROOT)
    .env_clear()
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

  if let Some(root) = root {
    command.env(DATA_ROOT_VARIABLE, root);
  }

  let mut child = command.spawn().expect("the tool starts");

  child
    .stdin
    .take()
    .expect("its input")
    .write_all(handed.to_string().as_bytes())
    .expect("the call is handed");

  let ended = child.wait_with_output().expect("the tool ends");

  assert!(ended.status.success(), "{}", String::from_utf8_lossy(&ended.stderr));
  serde_json::from_slice(&ended.stdout).expect("one tool result")
}

fn text(result: &Value) -> &str {
  result["content"][0]["text"].as_str().unwrap_or_default()
}

#[test]
fn a_record_acting_for_no_account_is_refused() {
  let root = data_root("127.0.0.1:9");
  let arguments = json!({ "q": "notes" });

  for session in [record(None), record(Some(""))] {
    let result = called(Some(root.path()), "artifact_search", arguments.clone(), session);

    assert_eq!(result["isError"], true, "{result}");
    assert!(text(&result).contains("acts for no account"), "{result}");
  }
}

#[test]
fn a_file_of_the_callers_machine_is_refused() {
  let root = data_root("127.0.0.1:9");
  let arguments = json!({ "title": "Notes", "path": "/a/file.md", "actorName": "tester" });
  let result = called(Some(root.path()), "artifact_publish", arguments, record(Some("11")));

  assert_eq!(result["isError"], true, "{result}");
  assert!(text(&result).contains("reads no file"), "{result}");
  assert!(text(&result).contains("`body`"), "{result}");
}

#[test]
fn a_call_handed_no_data_root_and_an_entry_it_does_not_hold_are_refused() {
  let arguments = json!({ "q": "notes" });
  let unrooted = called(None, "artifact_search", arguments.clone(), record(Some("11")));
  let unknown = called(None, "artifact_nothing", arguments, record(Some("11")));

  assert_eq!(unrooted["isError"], true, "{unrooted}");
  assert!(text(&unrooted).starts_with("ERROR: "), "{unrooted}");
  assert!(text(&unrooted).contains(DATA_ROOT_VARIABLE), "{unrooted}");
  assert_eq!(unknown["isError"], true, "{unknown}");
  assert!(text(&unknown).contains("no entry artifact/artifact_nothing"), "{unknown}");
}

#[test]
fn a_service_that_does_not_answer_is_a_failed_tool_result() {
  let root = data_root("127.0.0.1:9");
  let result =
    called(Some(root.path()), "artifact_document", json!({ "documentId": 1 }), record(Some("11")));

  assert_eq!(result["isError"], true, "{result}");
  assert!(text(&result).contains("did not answer"), "{result}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_publishes_for_the_records_account_and_finds_its_documents_alone() {
  let service = Service::new().await;
  let address = service.listening().await;
  let listen = address.trim_start_matches("http://").to_owned();
  let answers = tokio::task::spawn_blocking(move || {
    let root = data_root(&listen);
    let published = called(
      Some(root.path()),
      "artifact_publish",
      json!({ "title": "Hosted notes", "body": "What the *hosted* tool filed.", "actorName": "tester" }),
      record(Some("11")),
    );
    let own = called(Some(root.path()), "artifact_search", json!({ "q": "hosted" }), record(Some("11")));
    let other = called(Some(root.path()), "artifact_search", json!({ "q": "hosted" }), record(Some("12")));
    let missing =
      called(Some(root.path()), "artifact_document", json!({ "documentId": 999 }), record(Some("11")));

    (published, own, other, missing)
  })
  .await
  .expect("the calls ran");
  let (published, own, other, missing) = answers;
  let document = &published["structuredContent"]["document"];

  assert_eq!(published["isError"], false, "{published}");
  assert_eq!(document["title"], "Hosted notes", "{published}");
  assert_eq!(document["owner"], "11", "{published}");
  assert!(
    published["structuredContent"]["share"]["url"]
      .as_str()
      .is_some_and(|url| url.starts_with("https://")),
    "{published}"
  );
  assert_eq!(own["structuredContent"]["items"][0]["id"], document["id"], "{own}");
  assert_eq!(other["structuredContent"]["items"], json!([]), "{other}");

  let failure: Value = serde_json::from_str(text(&missing)).expect("the failure as JSON");

  assert_eq!(missing["isError"], true, "{missing}");
  assert_eq!(
    (failure["status"].clone(), failure["code"].clone()),
    (json!(404), json!("not-found")),
    "{missing}"
  );
}
