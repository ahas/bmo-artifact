//! One harness tool (`docs/api.md`, "The harness plugin"): its name, what it does, the route it calls, its
//! input and output schemas, and the call itself through the service's own client. Each tool registers
//! itself beside its own call, so adding one changes no file but its own; the MCP server lists them and the
//! OpenAPI document states them under `x-tools`.

use std::future::Future;
use std::pin::Pin;

use schemars::generate::SchemaGenerator;
use schemars::{JsonSchema, Schema};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::client::{ArtifactClient, ClientFailure};

pub use inventory;

/// What a tool's call came to short of the route's answer.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolFailure {
  /// The arguments are not the shape the tool takes.
  Arguments(String),
  /// The call reached no answer of the route's shape.
  Client(ClientFailure),
}

impl From<ClientFailure> for ToolFailure {
  fn from(failure: ClientFailure) -> Self {
    Self::Client(failure)
  }
}

impl ToolFailure {
  /// The failure as the tool error's content: the route's status with its `code` and `error`, or the
  /// sentence saying why nothing answered.
  pub fn content(&self) -> Value {
    match self {
      Self::Arguments(why) => json!({ "code": "bad-request", "error": why }),
      Self::Client(ClientFailure::Answered {
        status,
        answer: Some(answer),
        ..
      }) => json!({ "status": status, "code": answer.code, "error": answer.error }),
      Self::Client(failure @ ClientFailure::Answered { status, .. }) => {
        json!({ "status": status, "error": failure.to_string() })
      }
      Self::Client(failure) => json!({ "error": failure.to_string() }),
    }
  }
}

/// A tool's answer: the route's answer as JSON, or why there is none.
pub type Answered = Pin<Box<dyn Future<Output = Result<Value, ToolFailure>> + Send>>;

type Call = Box<dyn Fn(ArtifactClient, Value) -> Answered + Send + Sync>;

/// One tool.
pub struct ArtifactTool {
  pub name: &'static str,
  pub description: &'static str,
  /// The route the tool calls, as its method and path.
  pub route: &'static str,
  input: fn(&mut SchemaGenerator) -> Schema,
  output: fn(&mut SchemaGenerator) -> Schema,
  call: Call,
}

impl ArtifactTool {
  /// The tool whose input is `I` and whose answer is `O`, calling `call` with the arguments read as `I`.
  pub fn new<I, O, F, Fut>(
    name: &'static str,
    description: &'static str,
    route: &'static str,
    call: F,
  ) -> Self
  where
    I: DeserializeOwned + JsonSchema,
    O: Serialize + JsonSchema,
    F: Fn(ArtifactClient, I) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<O, ToolFailure>> + Send + 'static,
  {
    Self {
      name,
      description,
      route,
      input: I::json_schema,
      output: SchemaGenerator::subschema_for::<O>,
      call: Box::new(move |client, arguments| {
        let input = match serde_json::from_value::<I>(arguments) {
          Ok(input) => input,
          Err(error) => {
            let why = format!("the arguments are not the shape this tool takes: {error}");

            return Box::pin(std::future::ready(Err(ToolFailure::Arguments(why))));
          }
        };
        let answer = call(client, input);

        Box::pin(async move {
          let answer = answer.await?;

          serde_json::to_value(answer).map_err(|why| ToolFailure::Arguments(why.to_string()))
        })
      }),
    }
  }

  /// The tool's input, its nested types referred to where `generator` keeps its definitions.
  pub fn input(&self, generator: &mut SchemaGenerator) -> Schema {
    (self.input)(generator)
  }

  /// The tool's output, referred to where `generator` keeps its definitions.
  pub fn output(&self, generator: &mut SchemaGenerator) -> Schema {
    (self.output)(generator)
  }

  /// Calls the tool's route on the service `client` reaches, with the tool's arguments.
  pub fn call(&self, client: ArtifactClient, arguments: Value) -> Answered {
    (self.call)(client, arguments)
  }
}

/// One registered tool, made when it is listed.
pub struct RegisteredTool(pub fn() -> ArtifactTool);

inventory::collect!(RegisteredTool);

/// Registers a tool with the MCP server: `bmo_artifact::tool!(file_tool);`
#[macro_export]
macro_rules! tool {
  ($make:expr) => {
    $crate::mcp::tool::inventory::submit! { $crate::mcp::tool::RegisteredTool($make) }
  };
}

/// Every tool linked into this binary, by name.
pub fn tools() -> Vec<ArtifactTool> {
  let mut tools: Vec<ArtifactTool> =
    inventory::iter::<RegisteredTool>.into_iter().map(|made| (made.0)()).collect();

  tools.sort_by_key(|tool| tool.name);
  tools
}
