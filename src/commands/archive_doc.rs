//! A document filed from a shell (`docs/api.md`, "The archive verb"): `bmo archive doc` files it, or rewrites it
//! under its number, attaches the files it is handed, and prints its number, the link to hand to a reader and
//! the files it attached.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use clap::Args;

use crate::client::ArtifactClient;
use crate::documents::cover::{drawn_names, kept_names as names_drawn_by};
use crate::documents::keeper::{ATTACHMENT_CEILING, SNIFF_BYTES, attachment_kind};
use crate::kinds::{Kind, RecordKind};
use crate::plumbing::verb::{Answer, Shell, Verb, VerbError};
use crate::wire::{
  AttachmentInput, Document, DocumentChange, DocumentInput, Labels, PageQuery, ShareInput,
};

// The most attachments one read of a document's files answers.
const MOST_PER_PAGE: i64 = 200;

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
  /// A file attached to it under its own name; named again for each file. A picture is one the markdown body
  /// draws by that name, as ![caption](name), at the place its words speak of it.
  #[arg(long = "file")]
  files: Vec<PathBuf>,
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
  const ABOUT: &'static str = "Files a document from a shell, or rewrites it under its number, attaches the files it is \
                               handed, and prints its number, the permanent link to hand to a reader and the files it \
                               attached.";

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

fn answered(document: &Document, url: &str, attached: Vec<String>) -> Vec<String> {
  [format!("document {}: {}", document.id, document.title), url.to_owned()]
    .into_iter()
    .chain(attached)
    .collect()
}

/// A file to attach, read whole before anything is filed: the name it is kept under and its bytes.
struct Handed {
  name: String,
  bytes: Vec<u8>,
}

// Every file named, read and held to what the service keeps: a file that cannot be read, one over the ceiling of
// one file and two that share a name are refused here, so nothing is filed for them.
fn handed(files: &[PathBuf]) -> Result<Vec<Handed>, VerbError> {
  let mut handed: Vec<Handed> = Vec::with_capacity(files.len());

  for file in files {
    let Some(name) = file.file_name().map(|name| name.to_string_lossy().into_owned()) else {
      return Err(refused(format!("{}: names no file", file.display())));
    };
    let bytes = std::fs::read(file).map_err(|why| refused(format!("{}: {why}", file.display())))?;

    if bytes.len() > ATTACHMENT_CEILING {
      return Err(refused(format!(
        "{} weighs {} bytes, over the ceiling of {ATTACHMENT_CEILING}",
        file.display(),
        bytes.len()
      )));
    }

    if handed.iter().any(|one| one.name == name) {
      return Err(refused(format!("two files are named {name}")));
    }

    handed.push(Handed { name, bytes });
  }

  Ok(handed)
}

// What the service reads a handed file as, from its first bytes and its name.
fn kind_of(file: &Handed) -> (&'static str, String) {
  attachment_kind(&file.bytes[..file.bytes.len().min(SNIFF_BYTES)], &file.name)
}

// A document's pictures stand in its body: every picture handed with a markdown body is one the body draws by its
// name, and one it leaves out is refused before anything is filed, with the form that draws it.
fn drawn(body: &str, files: &[Handed]) -> Result<(), VerbError> {
  let targets = drawn_names(body);
  let draws = |name: &str| targets.iter().any(|target| names_drawn_by(target).contains(&name));
  let left_out: Vec<&str> = files
    .iter()
    .filter(|file| kind_of(file).0 == "image" && !draws(&file.name))
    .map(|file| file.name.as_str())
    .collect();

  if left_out.is_empty() {
    return Ok(());
  }

  Err(refused(format!(
    "the body draws no picture named {}; a document's pictures stand in its body, each drawn where its words \
     speak of it as ![caption](name), so nothing was filed",
    left_out.join(", ")
  )))
}

// The names of the files a document keeps, read page by page.
async fn kept_names(client: &ArtifactClient, document_id: i64) -> Result<HashSet<String>, VerbError> {
  let mut names = HashSet::new();
  let mut after = None;

  loop {
    let page = client
      .attachments(
        document_id,
        &PageQuery {
          after,
          limit: Some(MOST_PER_PAGE),
        },
      )
      .await
      .map_err(refused)?;

    after = page.items.last().map(|one| one.id);
    names.extend(page.items.into_iter().map(|one| one.name));

    if !page.more || after.is_none() {
      return Ok(names);
    }
  }
}

// Each file attached to the document in the order it was named, its type the one the service reads from its
// first bytes and its name; a line naming each.
async fn attach(
  client: &ArtifactClient,
  document_id: i64,
  files: Vec<Handed>,
) -> Result<Vec<String>, VerbError> {
  let mut lines = Vec::with_capacity(files.len());

  for file in files {
    let (_, media_type) = kind_of(&file);
    let input = AttachmentInput {
      content: STANDARD.encode(&file.bytes),
      name: file.name,
      media_type,
      role: None,
    };
    let attachment = client.attach(document_id, &input).await.map_err(|why| {
      refused(format!("document {document_id} stands, but {} is not attached: {why}", input.name))
    })?;

    lines.push(format!("attachment {}: {}", attachment.id, attachment.name));
  }

  Ok(lines)
}

/// The document filed or rewritten with the files it is handed: its lines, its number, its link and the files
/// attached.
pub async fn archive_doc(client: &ArtifactClient, args: &ArchiveDocArgs) -> Result<Vec<String>, VerbError> {
  let (kind, body, html) = match (&args.body, &args.html) {
    (Some(file), None) => (RecordKind.name(), Some(words(file)?), None),
    (None, Some(file)) => (PAGE, None, Some(words(file)?)),
    _ => return Err(refused("name its words with exactly one of --body <file> and --html <file>")),
  };
  let title = args.title.as_deref().map(str::trim).filter(|title| !title.is_empty());
  let files = handed(&args.files)?;

  if let Some(body) = &body {
    drawn(body, &files)?;
  }

  if let Some(id) = args.replace {
    // A rewrite names every label it keeps, so the labels it stands under are carried across.
    let standing = client.document(id).await.map_err(refused)?;

    // A kept file is never replaced, so a name the document keeps is refused before it is rewritten.
    if !files.is_empty() {
      let kept = kept_names(client, id).await?;

      if let Some(file) = files.iter().find(|one| kept.contains(&one.name)) {
        return Err(refused(format!(
          "document {id} already keeps a file named {}; attach a changed file under a new name",
          file.name
        )));
      }
    }

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

    let attached = attach(client, id, files).await?;

    return Ok(answered(&document, &share.url, attached));
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

  let attached = attach(client, published.document.id, files).await?;

  Ok(answered(&published.document, &published.share.url, attached))
}
