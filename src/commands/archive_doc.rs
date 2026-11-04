//! A document filed from a shell (`docs/api.md`, "The archive verb"): `bmo archive doc` files it, or rewrites it
//! under its number, and prints its number and the link to hand to a reader.

use std::path::{Path, PathBuf};

use clap::Args;

use crate::client::ArtifactClient;
use crate::kinds::{Kind, RecordKind};
use crate::plumbing::verb::{Answer, Shell, Verb, VerbError};
use crate::wire::{Document, DocumentChange, DocumentInput, Labels, ShareInput};

// The kind an HTML page is filed under.
const PAGE: &str = "page";

/// What `bmo archive doc` is given.
#[derive(Args)]
pub struct ArchiveDocArgs {
  /// Its title.
  #[arg(long, allow_hyphen_values = true)]
  title: Option<String>,
  /// A markdown file: its words.
  #[arg(long)]
  body: Option<PathBuf>,
  /// An HTML file: the page as it stands.
  #[arg(long)]
  html: Option<PathBuf>,
  /// The document rewritten under its number, in place of a new one.
  #[arg(long)]
  replace: Option<i64>,
  /// The room it is held under; the owner's own where left out.
  #[arg(long)]
  chat: Option<i64>,
  /// Who files it; the run's agent where left out.
  #[arg(long)]
  by: Option<String>,
}

/// `archive doc`.
pub struct ArchiveDoc;

impl Verb for ArchiveDoc {
  const NAME: &'static str = "archive doc";
  const ABOUT: &'static str = "Files a document from a shell, or rewrites it under its number, and prints its number \
                               and the permanent link to hand to a reader.";

  type Args = ArchiveDocArgs;

  async fn run(&self, args: ArchiveDocArgs, shell: &Shell) -> Result<Answer, VerbError> {
    archive_doc(&ArtifactClient::of_settings(&shell.settings, None), &args)
      .await
      .map(Answer::from)
  }
}

crate::verb!(ArchiveDoc);

fn refused(why: impl std::fmt::Display) -> VerbError {
  VerbError::Refused(format!("archive doc: {why}"))
}

fn words(file: &Path) -> Result<String, VerbError> {
  std::fs::read_to_string(file).map_err(|why| refused(format!("{}: {why}", file.display())))
}

fn answered(document: &Document, url: &str) -> Vec<String> {
  vec![format!("document {}: {}", document.id, document.title), url.to_owned()]
}

/// The document filed or rewritten: its lines, its number and its link.
pub async fn archive_doc(client: &ArtifactClient, args: &ArchiveDocArgs) -> Result<Vec<String>, VerbError> {
  let (kind, body, html) = match (&args.body, &args.html) {
    (Some(file), None) => (RecordKind.name(), Some(words(file)?), None),
    (None, Some(file)) => (PAGE, None, Some(words(file)?)),
    _ => return Err(refused("name its words with exactly one of --body <file> and --html <file>")),
  };
  let title = args.title.as_deref().map(str::trim).filter(|title| !title.is_empty());

  if let Some(id) = args.replace {
    // A rewrite names every label it keeps, so the labels it stands under are carried across.
    let standing = client.document(id).await.map_err(refused)?;
    let change = DocumentChange {
      title: title.map(str::to_owned),
      body,
      html,
      labels: standing.labels,
    };
    let document = client.rewrite(id, &change).await.map_err(refused)?;
    let share = client
      .share(&ShareInput {
        document_id: id,
        expires_at: None,
      })
      .await
      .map_err(refused)?;

    return Ok(answered(&document, &share.url));
  }

  let Some(title) = title else {
    return Err(refused("name its title with --title <words>"));
  };
  let actor = args
    .by
    .clone()
    .or_else(|| std::env::var("BMO_TURN_AGENT").ok())
    .or_else(|| std::env::var("USER").ok())
    .unwrap_or_default();
  let published = client
    .publish(&DocumentInput {
      kind: kind.to_owned(),
      title: title.to_owned(),
      body,
      html,
      scope: args.chat.map(|room| room.to_string()),
      actor_name: actor,
      labels: Labels::default(),
      description: None,
    })
    .await
    .map_err(refused)?;

  Ok(answered(&published.document, &published.share.url))
}
