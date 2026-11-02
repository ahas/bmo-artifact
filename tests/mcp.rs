//! The harness plugin's MCP server as a harness starts it (`docs/api.md`, "The harness plugin"):
//! `bmo-artifact mcp` answers the handshake at the repository's version and lists exactly its twelve tools;
//! a tool's call reaches the service through the client and a refusal comes back as a tool error.

use crate::support;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};
use support::ROOT;
use support::service::{Service, TOKEN};

/// The server started as a harness starts it, its answers read a line at a time; taken down when dropped.
struct Server {
  child: Child,
  lines: mpsc::Receiver<String>,
}

impl Server {
  fn start(address: &str) -> Self {
    let mut child = Command::new(env!("CARGO_BIN_EXE_bmo-artifact"))
      .args(["mcp", "--address", address])
      .current_dir(ROOT)
      .env("BMO_ARTIFACT_SERVER__TOKEN", TOKEN)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::null())
      .spawn()
      .expect("the server starts");
    let out = child.stdout.take().expect("its output");
    let (send, lines) = mpsc::channel();

    std::thread::spawn(move || {
      for line in BufReader::new(out).lines().map_while(Result::ok) {
        if send.send(line).is_err() {
          break;
        }
      }
    });

    Self { child, lines }
  }

  fn send(&mut self, message: Value) {
    let input = self.child.stdin.as_mut().expect("its input");

    writeln!(input, "{message}").expect("written");
    input.flush().expect("flushed");
  }

  // The answer to request `id`, waited for at most a few seconds.
  fn answer(&self, id: i64) -> Value {
    loop {
      let line = self.lines.recv_timeout(Duration::from_secs(20)).expect("the server answers");
      let message: Value = serde_json::from_str(&line).expect("a JSON-RPC message");

      if message["id"] == id {
        return message;
      }
    }
  }

  fn started(address: &str) -> (Self, Value) {
    let mut server = Self::start(address);

    server.send(json!({
      "jsonrpc": "2.0", "id": 1, "method": "initialize",
      "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "tester", "version": "1" } },
    }));

    let initialized = server.answer(1);

    server.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    (server, initialized)
  }
}

impl Drop for Server {
  fn drop(&mut self) {
    let _ = self.child.kill();
    let _ = self.child.wait();
  }
}

#[test]
fn it_lists_exactly_its_eleven_tools_at_the_repository_version() {
  let (mut server, initialized) = Server::started("http://127.0.0.1:9");

  assert_eq!(initialized["result"]["serverInfo"]["name"], "bmo-artifact");
  assert_eq!(initialized["result"]["serverInfo"]["version"], bmo_artifact::VERSION);

  server.send(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));

  let listed = server.answer(2);
  let names: Vec<&str> = listed["result"]["tools"]
    .as_array()
    .expect("tools")
    .iter()
    .filter_map(|tool| tool["name"].as_str())
    .collect();

  assert_eq!(
    names,
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
  assert!(
    listed["result"]["tools"]
      .as_array()
      .expect("tools")
      .iter()
      .all(|tool| tool["inputSchema"]["type"] == "object")
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_harness_files_a_document_finds_it_and_shares_it_through_the_tools() {
  let service = Service::new().await;
  let address = service.listening().await;
  let answers = tokio::task::spawn_blocking(move || {
    let (mut server, _) = Server::started(&address);
    let mut call = |id: i64, name: &str, arguments: Value| {
      server.send(json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": name, "arguments": arguments },
      }));
      server.answer(id)["result"].clone()
    };
    let filed = call(
      2,
      "artifact_file",
      json!({ "body": { "kind": "record", "title": "Harness notes", "body": "What the harness *found*.", "actorName": "tester", "labels": {} } }),
    );
    let id = filed["structuredContent"]["id"].clone();
    let found = call(3, "artifact_search", json!({ "q": "harness" }));
    let content = call(4, "artifact_content", json!({ "documentId": id }));
    let shared = call(5, "artifact_share", json!({ "documentId": id }));

    (filed, found, content, shared)
  })
  .await
  .expect("the harness ran");
  let (filed, found, content, shared) = answers;

  assert_eq!(filed["structuredContent"]["title"], "Harness notes", "{filed}");
  assert_eq!(
    found["structuredContent"]["items"][0]["id"], filed["structuredContent"]["id"],
    "{found}"
  );
  assert_eq!(
    content["structuredContent"]["content"], "V2hhdCB0aGUgaGFybmVzcyAqZm91bmQqLg==",
    "{content}"
  );
  assert!(
    shared["structuredContent"]["url"]
      .as_str()
      .is_some_and(|url| url.starts_with("https://")),
    "{shared}"
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_harness_publishes_a_document_from_a_file_and_answers_its_link() {
  let service = Service::new().await;
  let address = service.listening().await;
  let folder = tempfile::tempdir().expect("a folder of the machine's own");
  let file = folder.path().join("quarterlies.md");

  std::fs::write(&file, "# Quarterlies\n\nWhat the *quarter* came to.\n").expect("the document's file");
  let path = file.display().to_string();
  let answers = tokio::task::spawn_blocking(move || {
    let (mut server, _) = Server::started(&address);
    let mut call = |id: i64, name: &str, arguments: Value| {
      server.send(json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": name, "arguments": arguments },
      }));
      server.answer(id)["result"].clone()
    };
    let published = call(
      2,
      "artifact_publish",
      json!({ "title": "Quarterlies", "description": "What the quarter came to.", "path": path, "actorName": "tester" }),
    );
    let both = call(
      3,
      "artifact_publish",
      json!({ "title": "Twice", "body": "Words.", "path": path, "actorName": "tester" }),
    );

    (published, both)
  })
  .await
  .expect("the harness ran");
  let (published, both) = answers;
  let document = &published["structuredContent"]["document"];
  let share = &published["structuredContent"]["share"];

  assert_eq!(document["title"], "Quarterlies", "{published}");
  assert_eq!(document["description"], "What the quarter came to.", "{published}");
  assert_eq!(share["expiresAt"], Value::Null, "{published}");
  assert!(share["url"].as_str().is_some_and(|url| url.starts_with("https://")), "{published}");
  assert_eq!(both["isError"], true, "{both}");
}

#[test]
fn a_tool_whose_service_does_not_answer_is_a_tool_error_and_bad_arguments_are_a_bad_request() {
  let (mut server, _) = Server::started("http://127.0.0.1:9");

  server.send(json!({
    "jsonrpc": "2.0", "id": 2, "method": "tools/call",
    "params": { "name": "artifact_document", "arguments": { "documentId": 1 } },
  }));

  let unreached = server.answer(2);

  assert_eq!(unreached["result"]["isError"], true, "{unreached}");

  server.send(json!({
    "jsonrpc": "2.0", "id": 3, "method": "tools/call",
    "params": { "name": "artifact_document", "arguments": { "documentId": "one" } },
  }));

  let refused = server.answer(3);
  let text: Value =
    serde_json::from_str(refused["result"]["content"][0]["text"].as_str().expect("a text")).expect("JSON");

  assert_eq!(
    (refused["result"]["isError"].clone(), text["code"].clone()),
    (json!(true), json!("bad-request"))
  );
}
