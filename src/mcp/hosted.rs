//! The tool a tool host places (`docs/api.md`, "The tool a tool host places"): the harness tools served as the
//! entries of the tool host's namespace `artifact`. Each call is one process the host starts at home, handed the
//! call and the session record on standard input and answered with one tool result on standard output, made
//! through the service's own client acting for the record's `ownerAccount`.

use std::path::Path;
use std::process::ExitCode;

use rmcp::model::{CallToolResult, ContentBlock, JsonObject};
use serde::Deserialize;
use serde_json::{Map, Value};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use super::tool::{ArtifactTool, LocalFile, tools};
use super::{failed, listed};
use crate::client::ArtifactClient;
use crate::settings::Settings;

/// The call protocol the tool speaks.
pub const PROTOCOL: u64 = 1;
/// The namespace its entries stand in.
pub const NAMESPACE: &str = "artifact";
/// What a session's record holds for it to be shown the entries.
pub const CAPABILITY: &str = "documents.file";
/// The variable naming the data root, which the session's record hands every call.
pub const DATA_ROOT_VARIABLE: &str = "BMO_DATA_DIR";
/// The deployment's local settings, under the data root.
pub const LOCAL_SETTINGS: &str = "env/artifact.toml";

/// What a hosted call is refused with where its record names no account.
const NO_ACCOUNT: &str = "the session acts for no account, so the hosted artifact tools refuse it: they act \
  for the account a session's record names and never reach every document";

/// What the tool host hands a call, as far as this tool reads it.
#[derive(Debug, Deserialize)]
struct Handed {
  protocol: u64,
  kind: String,
  #[serde(default)]
  entry: Option<String>,
  #[serde(default)]
  arguments: Option<Value>,
  session: Record,
}

/// The session record a call is made under, as far as this tool reads it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
  #[serde(default)]
  owner_account: Option<String>,
}

/// One entry of the tool's manifest, as the harness tool it serves states it.
pub struct Entry {
  pub name: &'static str,
  pub description: String,
  pub read_only: bool,
  pub input: JsonObject,
  pub output: JsonObject,
}

/// Every entry the tool serves: one for each harness tool, by the same name.
pub fn entries() -> Vec<Entry> {
  tools()
    .iter()
    .map(|tool| {
      let listed = listed(tool);

      Entry {
        name: tool.name,
        description: description(tool),
        read_only: tool.read_only(),
        input: listed.input_schema.as_ref().clone(),
        output: listed.output_schema.as_deref().cloned().unwrap_or_default(),
      }
    })
    .collect()
}

/// A tool's description as its entry states it: the harness tool's own, and for each argument naming a file on
/// the caller's machine, that it is refused here.
pub fn description(tool: &ArtifactTool) -> String {
  tool.local_files().iter().fold(tool.description.to_owned(), |description, file| {
    format!(
      "{description} Run by a tool host at home, it reads no file of the caller's machine: `{}` is refused, \
       and the words go as `{}`.",
      file.argument, file.instead
    )
  })
}

fn refused(why: impl Into<String>) -> CallToolResult {
  CallToolResult::error(vec![ContentBlock::text(why.into())])
}

fn local_file_refusal(file: &LocalFile) -> String {
  format!(
    "`{}` names a file on the machine the caller stands on, and this tool runs at home, where it reads no file \
     of that machine: give the words as `{}` instead",
    file.argument, file.instead
  )
}

/// The tool result answering what the tool host handed, the service's settings read from `settings_dir` with the
/// local settings under `data_root` laid over them and this process's environment over both.
pub async fn answer(handed: &str, settings_dir: &Path, data_root: Option<&Path>) -> CallToolResult {
  let mut result = match serde_json::from_str::<Handed>(handed) {
    Ok(handed) => called(handed, settings_dir, data_root).await,
    Err(why) => refused(format!("ERROR: what the tool host handed does not read as a call: {why}")),
  };

  // The protocol's tool result carries its content, its error mark and its structured content alone.
  result.result_type = None;
  result
}

async fn called(handed: Handed, settings_dir: &Path, data_root: Option<&Path>) -> CallToolResult {
  if handed.protocol != PROTOCOL {
    return refused(format!(
      "ERROR: this tool speaks the call protocol {PROTOCOL}, not {}",
      handed.protocol
    ));
  }

  if handed.kind != "call" {
    return refused(format!("ERROR: this tool answers calls alone, not `{}`", handed.kind));
  }

  let entry = handed.entry.unwrap_or_default();
  let tools = tools();
  let Some(tool) = entry
    .strip_prefix(NAMESPACE)
    .and_then(|rest| rest.strip_prefix('/'))
    .and_then(|name| tools.iter().find(|tool| tool.name == name))
  else {
    return refused(format!("this tool holds no entry {entry}"));
  };
  let Some(account) = handed.session.owner_account.filter(|account| !account.is_empty()) else {
    return refused(NO_ACCOUNT);
  };
  let arguments = handed.arguments.unwrap_or_else(|| Value::Object(Map::new()));

  if let Some(file) = tool
    .local_files()
    .iter()
    .find(|file| arguments.get(file.argument).is_some_and(|value| !value.is_null()))
  {
    return refused(local_file_refusal(file));
  }

  let Some(data_root) = data_root else {
    return refused(format!(
      "ERROR: the call was handed no {DATA_ROOT_VARIABLE}, so the service's settings could not be read"
    ));
  };
  // The sentence names the files alone: what a settings file fails on may quote its values.
  let Ok(settings) = Settings::layered(settings_dir, &data_root.join(LOCAL_SETTINGS), true, None) else {
    return refused(format!(
      "ERROR: the service's settings could not be read from the placed settings.toml and \
       ${DATA_ROOT_VARIABLE}/{LOCAL_SETTINGS}"
    ));
  };
  let client = ArtifactClient::of_settings(&settings, None).for_account(Some(account));

  match tool.call(client, arguments).await {
    Ok(answer) => CallToolResult::structured(answer),
    Err(failure) => failed(&failure),
  }
}

/// One call answered on standard input and output: the settings read from the folder the host started the tool
/// in, the data root from `BMO_DATA_DIR`. Ends 0 once the result is written, 1 where it could not be.
pub async fn answer_standard_io() -> ExitCode {
  let mut handed = String::new();

  if let Err(why) = tokio::io::stdin().read_to_string(&mut handed).await {
    eprintln!("what the tool host handed could not be read: {why}");
    return ExitCode::FAILURE;
  }

  let data_root = std::env::var_os(DATA_ROOT_VARIABLE).filter(|root| !root.is_empty());
  let result = answer(&handed, Path::new("."), data_root.as_deref().map(Path::new)).await;
  let line = match serde_json::to_string(&result) {
    Ok(line) => line + "\n",
    Err(why) => {
      eprintln!("the tool result could not be written: {why}");
      return ExitCode::FAILURE;
    }
  };
  let mut out = tokio::io::stdout();

  match out.write_all(line.as_bytes()).await.and(out.flush().await) {
    Ok(()) => ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("the tool result could not be written: {why}");
      ExitCode::FAILURE
    }
  }
}
