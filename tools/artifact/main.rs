//! The `artifact` tool's executable: one call the tool host hands on standard input, answered on standard output
//! (`docs/api.md`, "The tool a tool host places").

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
  bmo_artifact::mcp::hosted::answer_standard_io().await
}
