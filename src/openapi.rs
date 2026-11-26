//! The service's OpenAPI document (`docs/api.md`, "The wire"): every route linked in with the operation it
//! declares beside itself, the body schemas generated from the types of `wire`, the events, the harness tools
//! and the plugin. `bmo-artifact openapi` prints it, and the repository keeps it as `docs/openapi.json`.

use std::collections::BTreeMap;

use schemars::generate::SchemaSettings;
use serde_json::{Map, Value, json};

use crate::events::EventKind;
use crate::mcp::ENTRY;
use crate::mcp::tool::tools;
use crate::plumbing::http::routes;
use crate::wire::WireError;

/// Where the schemas the routes refer to stand in the document.
const SCHEMAS: &str = "/components/schemas";

const DESCRIPTION: &str = "Documents kept, searched and shared: records, learning books, imports and pages, \
  each with its body, attachments, references and cover picture; share links that open a document for whoever \
  holds the token; page imports read through bmo-explorer's client. The paths carry no prefix; whatever serves \
  the service to a browser mounts it under its own.";

/// The service's OpenAPI document.
pub fn openapi() -> Value {
  let mut generator = SchemaSettings::draft2020_12()
    .for_serialize()
    .with(|settings| {
      settings.definitions_path = SCHEMAS.into();
      settings.meta_schema = None;
    })
    .into_generator();
  let error = generator.subschema_for::<WireError>();
  let mut by_path: BTreeMap<&str, BTreeMap<String, Value>> = BTreeMap::new();
  let mut tags: Vec<&str> = Vec::new();

  for route in routes() {
    let operation = route.operation(&mut generator);
    let reach = route.console();

    if !tags.contains(&operation.tag) {
      tags.push(operation.tag);
    }

    let mut value = operation.value();

    // The console's token is refused wherever it reaches the route under a guard or not at all.
    if reach.forbids_console()
      && let Some(codes) = value["x-codes"].as_array_mut()
      && !codes.iter().any(|code| code == "forbidden")
    {
      codes.push("forbidden".into());
    }

    value["x-console"] = reach.word().into();
    by_path
      .entry(route.path())
      .or_default()
      .insert(route.method().as_str().to_lowercase(), value);
  }

  tags.sort_unstable();

  let paths: Map<String, Value> = by_path
    .into_iter()
    .map(|(path, methods)| (path.to_owned(), Value::Object(methods.into_iter().collect())))
    .collect();
  let x_tools: Vec<Value> = tools()
    .iter()
    .map(|tool| {
      json!({
        "name": tool.name,
        "description": tool.description,
        "input": tool.input(&mut generator),
        "output": tool.output(&mut generator),
        "route": tool.route,
      })
    })
    .collect();

  json!({
    "openapi": "3.1.0",
    "info": { "title": "bmo-artifact", "version": env!("CARGO_PKG_VERSION"), "description": DESCRIPTION },
    "servers": [{ "url": "/", "description": "the artifact service" }],
    "security": [{ "bearer": [] }],
    "x-layer": "upstream",
    "x-stage": 3,
    "x-calls": ["explorer", "node", "web"],
    "x-harness": { "plugin": "bmo-artifact", "mcp": "bmo-artifact mcp", "entry": ENTRY },
    "tags": tags.iter().map(|tag| json!({ "name": tag })).collect::<Vec<_>>(),
    "paths": paths,
    "components": {
      "schemas": generator.take_definitions(true),
      "responses": {
        "Error": {
          "description": "A refusal, in the common error shape.",
          "content": { "application/json": { "schema": error } },
        },
      },
      "securitySchemes": {
        "bearer": {
          "type": "http",
          "scheme": "bearer",
          "description": "The service's token, or the console's, as the settings name them; the console's reaches \
            each operation as its x-console says, under the Owner-Role and Owner-Key headers where it is a screen's; \
            the console face, its x-console face, takes the console's alone.",
        },
      },
    },
    "x-events": EventKind::ALL.iter().map(|kind| kind.name()).collect::<Vec<_>>(),
    "x-tools": x_tools,
  })
}

/// The document as the repository keeps it: two-space indented JSON with a closing newline.
pub fn openapi_json() -> String {
  let mut text = serde_json::to_string_pretty(&openapi()).unwrap_or_default();

  text.push('\n');
  text
}
