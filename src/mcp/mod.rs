//! The harness plugin's MCP server, `bmo-artifact mcp` (`docs/api.md`, "The harness plugin"): the tools over
//! the service one client reaches, each call made through that client.

pub mod tool;
pub mod tools;

use std::sync::Arc;

use rmcp::model::{
  CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, InitializeResult,
  JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use schemars::Schema;
use schemars::generate::SchemaSettings;
use serde_json::{Map, Value};

use crate::client::ArtifactClient;
use tool::{ArtifactTool, ToolFailure, tools};

/// What a harness does first with the server, which the OpenAPI document's `x-harness` states too.
pub const ENTRY: &str = "A harness files what it wrote with artifact_file, finds earlier documents with \
  artifact_search, and hands a reader a link with artifact_share.";

/// What the server tells a harness of itself when it starts.
const INSTRUCTIONS: &str = "Keeps documents, finds them again and shares them by link. File a markdown \
  record or learning book, or an HTML page, with artifact_file; find documents with artifact_search or list \
  them with artifact_documents; read one with artifact_document and its file with artifact_content; rewrite \
  it with artifact_rewrite and attach files with artifact_attach; hand a reader a link with artifact_share, \
  list links with artifact_shares and end one with artifact_unshare; bring a web page in whole with \
  artifact_import. A refusal comes back as a tool error carrying the service's status, code and sentence.";

/// The server over the service the client reaches.
pub struct ArtifactServer {
  client: ArtifactClient,
  tools: Arc<Vec<ArtifactTool>>,
}

impl ArtifactServer {
  pub fn new(client: ArtifactClient) -> Self {
    Self {
      client,
      tools: Arc::new(tools()),
    }
  }

  /// The tools as MCP lists them, each input and output a schema of its own with its definitions inside.
  pub fn listed(&self) -> Vec<Tool> {
    self.tools.iter().map(listed).collect()
  }
}

fn listed(tool: &ArtifactTool) -> Tool {
  let mut generator = SchemaSettings::draft2020_12()
    .for_serialize()
    .with(|settings| settings.meta_schema = None)
    .into_generator();
  let input = tool.input(&mut generator);
  let output = tool.output(&mut generator);
  let definitions = generator.take_definitions(true);
  let mut listed = Tool::new(tool.name, tool.description, standing_alone(input, &definitions));

  listed.output_schema = Some(Arc::new(standing_alone(output, &definitions)));
  listed
}

/// A schema made to stand on its own: an object at its root, with every definition it refers to beside it.
fn standing_alone(schema: Schema, definitions: &Map<String, Value>) -> JsonObject {
  let mut object = match schema.to_value() {
    Value::Object(object) if object.contains_key("$ref") => {
      let mut wrapped = Map::new();

      wrapped.insert("allOf".into(), Value::Array(vec![Value::Object(object)]));
      wrapped.insert("type".into(), "object".into());
      wrapped
    }
    Value::Object(object) => object,
    _ => {
      let mut any = Map::new();

      any.insert("type".into(), "object".into());
      any
    }
  };

  if !definitions.is_empty() {
    object.insert("$defs".into(), Value::Object(definitions.clone()));
  }

  object
}

impl ServerHandler for ArtifactServer {
  fn get_info(&self) -> InitializeResult {
    let mut info = InitializeResult::new(ServerCapabilities::builder().enable_tools().build());
    let mut server = Implementation::default();

    server.name = "bmo-artifact".to_owned();
    server.version = crate::VERSION.to_owned();
    info.server_info = server;
    info.instructions = Some(INSTRUCTIONS.to_owned());
    info
  }

  async fn list_tools(
    &self,
    _: Option<PaginatedRequestParams>,
    _: RequestContext<RoleServer>,
  ) -> Result<ListToolsResult, ErrorData> {
    Ok(ListToolsResult::with_all_items(self.listed()))
  }

  async fn call_tool(
    &self,
    request: CallToolRequestParams,
    _: RequestContext<RoleServer>,
  ) -> Result<CallToolResponse, ErrorData> {
    let Some(tool) = self.tools.iter().find(|tool| tool.name == request.name) else {
      return Err(ErrorData::invalid_params(format!("there is no tool {}", request.name), None));
    };
    let arguments = Value::Object(request.arguments.unwrap_or_default());
    let result = match tool.call(self.client.clone(), arguments).await {
      Ok(answer) => CallToolResult::structured(answer),
      Err(failure) => failed(&failure),
    };

    Ok(CallToolResponse::Complete(result))
  }
}

/// A failed call as a tool error whose text is the failure's content. It carries no structured content, which
/// would be read against the tool's output schema.
fn failed(failure: &ToolFailure) -> CallToolResult {
  CallToolResult::error(vec![ContentBlock::text(failure.content().to_string())])
}

/// Serves the tools on standard input and output until the harness ends the session.
pub async fn serve(client: ArtifactClient) -> Result<(), String> {
  let served = ArtifactServer::new(client)
    .serve(rmcp::transport::stdio())
    .await
    .map_err(|why| format!("the MCP session did not start: {why}"))?;

  served
    .waiting()
    .await
    .map(|_| ())
    .map_err(|why| format!("the MCP session ended in a failure: {why}"))
}
