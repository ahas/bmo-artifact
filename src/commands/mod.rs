//! The command's words (`docs/api.md`, "The command"), each registered beside its own run: the service, the
//! schema, the readiness asked, the OpenAPI document, the MCP server, an HTML file read and the version.

pub mod html_read;

use std::time::Duration;

use clap::Args;
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};

use crate::client::ArtifactClient;
use crate::http::imports::abandon;
use crate::plumbing::http::{Standing, router};
use crate::plumbing::verb::{Answer, Shell, Verb, VerbError};
use crate::store::migration;

/// A command that is given nothing.
#[derive(Args)]
pub struct Nothing {}

/// `version`.
pub struct Version;

impl Verb for Version {
  const NAME: &'static str = "version";
  const ABOUT: &'static str = "Prints the repository's version.";

  type Args = Nothing;

  async fn run(&self, _: Nothing, _: &Shell) -> Result<Answer, VerbError> {
    Ok(vec![crate::VERSION.to_owned()].into())
  }
}

crate::verb!(Version);

/// `migrate`.
pub struct Migrate;

impl Verb for Migrate {
  const NAME: &'static str = "migrate";
  const ABOUT: &'static str = "Brings the database the settings name up to this repository's schema.";

  type Args = Nothing;

  async fn run(&self, _: Nothing, shell: &Shell) -> Result<Answer, VerbError> {
    let ran = migration::migrate(shell.archive.db()?).await?;

    Ok(vec![format!("migrations run: {ran}")].into())
  }
}

crate::verb!(Migrate);

/// `serve`.
pub struct Serve;

// Ends once the process is asked to stop, by an interrupt or by its unit.
async fn stopped() {
  let Ok(mut terminate) = signal(SignalKind::terminate()) else {
    let _ = tokio::signal::ctrl_c().await;
    return;
  };

  tokio::select! {
    _ = tokio::signal::ctrl_c() => {}
    _ = terminate.recv() => {}
  }
}

impl Verb for Serve {
  const NAME: &'static str = "serve";
  const ABOUT: &'static str = "Answers the routes on the address the settings name until it is stopped.";

  type Args = Nothing;

  async fn run(&self, _: Nothing, shell: &Shell) -> Result<Answer, VerbError> {
    let listen = shell.settings.server.listen;
    let standing =
      Standing::new(shell.settings.clone()).map_err(|why| VerbError::Refused(why.to_string()))?;

    abandon(&standing).await.map_err(|why| VerbError::Refused(why.error))?;

    let router = router(standing).map_err(|faults| {
      VerbError::Refused(format!("the routes cannot be served: {}", faults.join("; ")))
    })?;
    let listener = TcpListener::bind(listen)
      .await
      .map_err(|why| VerbError::Refused(format!("{listen} cannot be listened on: {why}")))?;

    eprintln!("bmo-artifact {} listens on {listen}", crate::VERSION);

    axum::serve(listener, router)
      .with_graceful_shutdown(stopped())
      .await
      .map_err(|why| VerbError::Refused(format!("the server stopped: {why}")))?;

    Ok(Answer::default())
  }
}

crate::verb!(Serve);

/// `health`.
pub struct Health;

/// How long `health` waits for the service.
pub const HEALTH_WAIT: Duration = Duration::from_secs(5);

impl Verb for Health {
  const NAME: &'static str = "health";
  const ABOUT: &'static str =
    "Asks the service the settings name whether it is ready, and ends 0 where it is.";

  type Args = Nothing;

  async fn run(&self, _: Nothing, shell: &Shell) -> Result<Answer, VerbError> {
    let client = ArtifactClient::of_settings(&shell.settings, None);
    let health = tokio::time::timeout(HEALTH_WAIT, client.health())
      .await
      .map_err(|_| VerbError::Refused("the service did not answer within five seconds".to_owned()))?
      .map_err(|why| VerbError::Refused(why.to_string()))?;

    match health.reason {
      None if health.ready => Ok(vec![format!("ready: bmo-artifact {}", health.version)].into()),
      reason => Err(VerbError::Refused(format!("not ready: {}", reason.unwrap_or_default()))),
    }
  }
}

crate::verb!(Health);

/// `openapi`.
pub struct OpenApi;

impl Verb for OpenApi {
  const NAME: &'static str = "openapi";
  const ABOUT: &'static str = "Prints the OpenAPI document the code generates.";

  type Args = Nothing;

  async fn run(&self, _: Nothing, _: &Shell) -> Result<Answer, VerbError> {
    Ok(vec![crate::openapi::openapi_json().trim_end().to_owned()].into())
  }
}

crate::verb!(OpenApi);

/// What `mcp` is given.
#[derive(Args)]
pub struct McpArgs {
  /// The service's address, where it is not the settings' listening address.
  #[arg(long)]
  address: Option<String>,
}

/// `mcp`.
pub struct Mcp;

impl Verb for Mcp {
  const NAME: &'static str = "mcp";
  const ABOUT: &'static str = "Serves the harness tools over MCP on standard input and output.";

  type Args = McpArgs;

  async fn run(&self, args: McpArgs, shell: &Shell) -> Result<Answer, VerbError> {
    let client = ArtifactClient::of_settings(&shell.settings, args.address.as_deref());

    crate::mcp::serve(client).await.map_err(VerbError::Refused)?;

    Ok(Answer::default())
  }
}

crate::verb!(Mcp);
