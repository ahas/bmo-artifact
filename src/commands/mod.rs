//! The command's words (`docs/api.md`, "The command"), each registered beside its own run: the service, the
//! schema, the readiness asked, the OpenAPI document, the MCP server, the nouns it declares, a document filed
//! from a shell, an HTML file read and the version.

pub mod archive_doc;
pub mod html_read;

use std::path::PathBuf;

use clap::Args;
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};

use crate::client::ArtifactClient;
use crate::http::imports::abandon;
use crate::plumbing::http::{Standing, router};
use crate::plumbing::verb::{Answer, FAILURE, Shell, Verb, VerbError};
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

/// The nouns `bmo` dispatches to this command (`docs/api.md`, "The command"): a document filed from a shell, and
/// the page read, each run on the machine its file stands on. No noun carries a mark.
pub const NOUNS: &[&str] = &["archive doc", "html-read"];

/// `nouns`.
pub struct Nouns;

impl Verb for Nouns {
  const NAME: &'static str = "nouns";
  const ABOUT: &'static str = "Prints the nouns bmo dispatches to this command, one a line.";

  type Args = Nothing;

  async fn run(&self, _: Nothing, _: &Shell) -> Result<Answer, VerbError> {
    Ok(NOUNS.iter().map(|noun| (*noun).to_owned()).collect::<Vec<String>>().into())
  }
}

crate::verb!(Nouns);

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

/// What `import` is given.
#[derive(Args)]
pub struct ImportArgs {
  /// The export directory the item's export hand wrote.
  #[arg(long)]
  pub export: PathBuf,
  /// The PostgreSQL address; the settings' own database where none is given.
  #[arg(long)]
  pub database: Option<String>,
  /// Reads the export and reports what it would write, writing nothing.
  #[arg(long)]
  pub dry_run: bool,
}

/// `import`: the migration plan's move of the old archive's document tables into this store (TODO #1923).
pub struct Import;

impl Verb for Import {
  const NAME: &'static str = "import";
  const ABOUT: &'static str =
    "Moves the old archive's document tables from the item's export into the store (TODO #1923).";

  type Args = ImportArgs;

  async fn run(&self, args: ImportArgs, shell: &Shell) -> Result<Answer, VerbError> {
    let database = args.database.unwrap_or_else(|| shell.settings.database.url.clone());
    let report = crate::import::run(&database, &args.export, args.dry_run)
      .await
      .map_err(VerbError::Refused)?;
    let failed = !report.sample.failures.is_empty();

    Ok(Answer {
      lines: report.lines(),
      code: if failed { FAILURE } else { 0 },
    })
  }
}

crate::verb!(Import);

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

    if !standing.console.built() {
      eprintln!(
        "the console bundle is not built in {}; its manifest answers not-found",
        shell.settings.console.bundle.display()
      );
    }

    let stopping = standing.clone();
    let router = router(standing).map_err(|faults| {
      VerbError::Refused(format!("the routes cannot be served: {}", faults.join("; ")))
    })?;
    let listener = TcpListener::bind(listen)
      .await
      .map_err(|why| VerbError::Refused(format!("{listen} cannot be listened on: {why}")))?;

    eprintln!("bmo-artifact {} listens on {listen}", crate::VERSION);

    axum::serve(listener, router)
      .with_graceful_shutdown(async move {
        stopped().await;
        stopping.stop();
      })
      .await
      .map_err(|why| VerbError::Refused(format!("the server stopped: {why}")))?;

    Ok(Answer::default())
  }
}

crate::verb!(Serve);

/// `health`.
pub struct Health;

impl Verb for Health {
  const NAME: &'static str = "health";
  const ABOUT: &'static str =
    "Asks the service the settings name whether it is ready, and ends 0 where it is.";

  type Args = Nothing;

  async fn run(&self, _: Nothing, shell: &Shell) -> Result<Answer, VerbError> {
    let client = ArtifactClient::of_settings(&shell.settings, None);
    let health = client.health().await.map_err(|why| VerbError::Refused(why.to_string()))?;

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
