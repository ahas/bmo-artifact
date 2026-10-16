//! A document the agent writes into the archive, and one already there handed to a room
//! (`docs/archive.md`, "A document the agent writes"): the data the owner asked to have put in order and
//! kept, as the agent's own markdown with the files handed over copied in beside it, or an HTML page kept
//! as it stands, which is the document itself.
//!
//! Nothing is written unless every part of it can be: the title, the body and every file are checked before
//! the row exists, so a refusal leaves no half-made document behind. A room of other people is handed the
//! document's share link, since the console address is behind the console's sign-in; the owner's private
//! room is handed the console address and never a link.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Datelike, Timelike};
use chrono_tz::Tz;
use clap::Args;
use regex::{Regex, RegexBuilder};
use scraper::{Html, Node};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use crate::clients::voice::Briefing;
use crate::helpers::ceiling::count_text;
use crate::helpers::marks::fill_marks;
use crate::helpers::picture;
use crate::plumbing::verb::{Answer, Shell, Verb, VerbError};
use crate::store::documents::{Attachment, NewDocument, keep_attachment, start_document};
use crate::wording::archive_docs::{ArchiveDocs, Time};
use crate::{Refusal, all, now_ms, one, run};

/// The kinds the agent names with `--kind`; a page is filed by its file alone, and a search is the
/// searcher's.
pub const AGENT_KINDS: [&str; 3] = ["record", "learning", "import"];

const PAGE_KIND: &str = "page";

/// The most one kept file may weigh: a film is the heaviest thing a document keeps.
pub const ATTACHMENT_CEILING: u64 = 1024 * 1024 * 1024;

/// How long a link made for a room lives where no span is named.
pub const SHARE_LIFE_SECONDS: i64 = 7 * 24 * 60 * 60;

/// How long a room's share link is asked to live: some seconds, or with no end, as the console's endless
/// span writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShareLife {
  Seconds(i64),
  Endless,
}

impl Default for ShareLife {
  fn default() -> Self {
    Self::Seconds(SHARE_LIFE_SECONDS)
  }
}

/// About how long a page's description runs when it is taken from its words.
pub const DESCRIPTION_LENGTH: usize = 200;

// How much of a file is read to tell what it is.
const SNIFF_BYTES: usize = 64 * 1024;

/// How a picture kept beside a document is written: inside this many pixels, at this AVIF quality and speed.
pub const KEPT_PICTURE: picture::Archived = picture::Archived {
  max_dimension: 2048,
  quality: 50,
  speed: 6,
};

/// The type a picture kept as AVIF is written down as.
pub const KEPT_PICTURE_TYPE: &str = "image/avif";

const DOC_USAGE: &str = "usage: bmo archive doc [--chat <chatId>] [--by <userId>] --title '<title>' [--kind record|learning|import] [--body <file> | stdin] [--attach <path>]… [--narration <file>] [--share-days <n> | --share-forever] [--replace <id>]
       bmo archive doc [--chat <chatId>] [--by <userId>] --html <file> [--title '<title>'] [--share-days <n> | --share-forever] [--replace <id>]";

const SHARE_USAGE: &str = "usage: bmo archive share <id> --chat <chatId> [--days <n> | --forever]";

/// A document the agent writes into the archive, as the binary takes it.
#[derive(Args)]
#[command(allow_negative_numbers = true)]
pub struct ArchiveDoc {
  /// The room that asked; the settings' default room where none is named.
  #[arg(long)]
  chat: Option<String>,
  /// Who in it asked; the settings' default asker where none is named.
  #[arg(long)]
  by: Option<String>,
  #[arg(long)]
  title: Option<String>,
  #[arg(long, default_value = "record")]
  kind: String,
  /// The body's file; read from standard input where none is named and no page is filed.
  #[arg(long)]
  body: Option<PathBuf>,
  /// A file kept beside the document, each named once.
  #[arg(long = "attach")]
  attach: Vec<String>,
  /// An HTML page filed as the document itself.
  #[arg(long)]
  html: Option<String>,
  /// A script for the ear, spoken in the agent's voice and kept as the document's sound.
  #[arg(long)]
  narration: Option<PathBuf>,
  /// How long the room's share link lives, in days.
  #[arg(long)]
  share_days: Option<String>,
  /// The room's share link made to live with no end.
  #[arg(long)]
  share_forever: bool,
  /// One of the agent's own documents written again under its number.
  #[arg(long)]
  replace: Option<String>,
}

/// One document handed to a room, as the binary takes it.
#[derive(Args)]
#[command(allow_negative_numbers = true)]
pub struct ArchiveShare {
  id: Option<String>,
  #[arg(long)]
  chat: Option<String>,
  #[arg(long)]
  days: Option<String>,
  #[arg(long)]
  forever: bool,
}

/// `archive doc`.
pub struct FileDocument;

impl Verb for FileDocument {
  const NAME: &'static str = "archive doc";
  const ALIASES: &'static [&'static str] = &["archive-doc"];
  const ABOUT: &'static str = "Writes a document into the archive: the data the owner asked to have put in order \
                               and kept, with the files it keeps, or an HTML page as the document itself; and \
                               hands it to the room.";

  type Args = ArchiveDoc;

  async fn run(&self, args: ArchiveDoc, shell: &Shell) -> Result<Answer, VerbError> {
    run_doc(args, shell).await.map(Answer::from)
  }
}

crate::verb!(FileDocument);

/// `archive share`.
pub struct ShareDocument;

impl Verb for ShareDocument {
  const NAME: &'static str = "archive share";
  const ALIASES: &'static [&'static str] = &["archive-share"];
  const ABOUT: &'static str = "Hands one document already in the archive to a room: its share link, or the \
                               console address in the owner's private room.";

  type Args = ArchiveShare;

  async fn run(&self, args: ArchiveShare, shell: &Shell) -> Result<Answer, VerbError> {
    run_share(args, shell).await.map(Answer::from)
  }
}

crate::verb!(ShareDocument);

// A whole number as a script's argument reads one.
fn whole(text: &str) -> Option<i64> {
  text.trim().parse().ok()
}

// The life a link is asked for: a span of days, which must be some time at all, or no end, never both; a
// week where neither is named.
fn life_of(days: Option<&str>, forever: bool) -> Result<ShareLife, ()> {
  match (days, forever) {
    (None, false) => Ok(ShareLife::default()),
    (None, true) => Ok(ShareLife::Endless),
    (Some(_), true) => Err(()),
    (Some(days), false) => {
      let days: f64 = days.trim().parse().map_err(drop)?;
      let seconds = (days * 24.0 * 60.0 * 60.0).round();

      if seconds > 0.0 { Ok(ShareLife::Seconds(seconds as i64)) } else { Err(()) }
    }
  }
}

/// What one document is to be written as, once its arguments are read.
pub struct DocumentInput {
  pub chat_id: i64,
  /// Who asked for it, where anyone did.
  pub asker_id: Option<i64>,
  pub title: String,
  pub kind: String,
  pub body: String,
  pub attach: Vec<String>,
  pub html: Option<String>,
  pub replace: Option<i64>,
}

/// `archive doc` as the binary runs it.
pub async fn run_doc(asked: ArchiveDoc, shell: &Shell) -> Result<Vec<String>, VerbError> {
  let settings = &shell.settings.documents;
  let usage = || VerbError::Refused(DOC_USAGE.to_owned());
  let chat_id = match asked.chat.as_deref() {
    Some(named) => whole(named).ok_or_else(usage)?,
    None => settings.room.ok_or_else(|| {
      VerbError::Refused(
        "archive-doc: no room is named: give --chat <chatId>, or set room under [documents] in the settings."
          .to_owned(),
      )
    })?,
  };
  let asker_id = match asked.by.as_deref() {
    Some(named) => Some(whole(named).ok_or_else(usage)?),
    None => settings.asker,
  };
  let life = life_of(asked.share_days.as_deref(), asked.share_forever).map_err(|()| usage())?;
  let replace = match asked.replace.as_deref() {
    None => None,
    Some(named) => Some(whole(named).filter(|id| *id > 0).ok_or_else(|| {
      VerbError::Refused("archive-doc: --replace names a document by its number.".to_owned())
    })?),
  };
  let web = console_of(shell, "archive-doc")?;
  let body = match (&asked.body, &asked.html) {
    (Some(file), _) => std::fs::read_to_string(file).map_err(|_| {
      VerbError::Refused(format!("archive-doc: the body file {} cannot be read.", file.display()))
    })?,
    (None, None) => {
      use std::io::Read as _;

      let mut read = String::new();

      std::io::stdin()
        .read_to_string(&mut read)
        .map_err(|why| VerbError::Refused(format!("archive-doc: standing input cannot be read: {why}")))?;
      read
    }
    (None, Some(_)) => String::new(),
  };
  let narration = match &asked.narration {
    None => None,
    Some(file) => Some(narration_of(file, asked.html.is_some())?),
  };
  let input = DocumentInput {
    chat_id,
    asker_id,
    title: asked.title.unwrap_or_default(),
    kind: asked.kind,
    body,
    attach: asked.attach,
    html: asked.html,
    replace,
  };
  let documents = Documents::of_shell(shell, shell.documents().await?.clone(), web);
  let written = documents
    .write(&input)
    .await
    .map_err(|why| VerbError::Refused(format!("archive-doc: {why}")))?;
  let owner_room = settings.room == Some(chat_id);
  let handover = documents.hand_over(written.id, owner_room, life).await?;
  let mut lines = vec![format!(
    "archive-doc: document {} {}",
    written.id,
    if replace.is_none() { "written" } else { "written again" }
  )];

  lines.extend(documents.handover_lines(&handover));

  if !written.attachments.is_empty() {
    let kept: Vec<String> = written
      .attachments
      .iter()
      .map(|kept| format!("{} ({})", kept.name, kept.media))
      .collect();

    lines.push(format!("attachments: {}", kept.join(", ")));
  }

  if let Some(script) = narration {
    match narrate(shell, chat_id, written.id, script).await {
      Ok(line) => lines.push(line),
      Err(refusal) => return Err(VerbError::Partly { lines, refusal }),
    }
  }

  Ok(lines)
}

// The console's public address the settings name, or the refusal of a verb that cannot hand a document
// over without it.
fn console_of(shell: &Shell, verb: &str) -> Result<String, VerbError> {
  shell
    .settings
    .console
    .address
    .clone()
    .filter(|address| !address.trim().is_empty())
    .ok_or_else(|| {
      VerbError::Refused(format!(
        "{verb}: the console's address is not set: set address under [console] in the settings."
      ))
    })
}

// The narration's script, read before anything is written: a page carries its own in its listen band, and a
// script with nothing in it would only be refused once the document stands.
fn narration_of(file: &Path, page: bool) -> Result<String, VerbError> {
  if page {
    return Err(VerbError::Refused(
      "archive-doc: a page carries its own narration in its listen band; give it no --narration."
        .to_owned(),
    ));
  }

  let script = std::fs::read_to_string(file).map_err(|_| {
    VerbError::Refused(format!("archive-doc: the narration file {} cannot be read.", file.display()))
  })?;

  if script.trim().is_empty() {
    return Err(VerbError::Refused("archive-doc: the narration is empty.".to_owned()));
  }

  Ok(script)
}

// A written document's narration handed to bmo-voice, spoken in the agent's voice and kept as the document's
// sound (`docs/archive.md`, "A document the agent writes"): the line saying it is being spoken, or why it is
// not.
async fn narrate(shell: &Shell, chat_id: i64, id: i64, script: String) -> Result<String, String> {
  let briefing = Briefing {
    chat_id,
    text: script,
    started_by: format!("archive-doc:{id}"),
    document: Some(id),
  };
  let not_spoken =
    |why: String| format!("archive-doc: the narration of document {id} is not spoken: {why}");

  match shell.voice.brief(&briefing).await {
    Ok(Ok(unit)) => Ok(format!(
      "narration: being spoken as {unit}, kept as the document's sound when it is done"
    )),
    Ok(Err(refused)) => Err(not_spoken(format!("voice: {refused}"))),
    Err(why) => Err(not_spoken(why)),
  }
}

/// `archive share` as the binary runs it.
pub async fn run_share(asked: ArchiveShare, shell: &Shell) -> Result<Vec<String>, VerbError> {
  let usage = || VerbError::Refused(SHARE_USAGE.to_owned());
  let id = asked.id.as_deref().and_then(whole).filter(|id| *id > 0).ok_or_else(usage)?;
  let chat_id = asked.chat.as_deref().and_then(whole).ok_or_else(usage)?;
  let life = life_of(asked.days.as_deref(), asked.forever).map_err(|()| usage())?;
  let web = console_of(shell, "archive-share")?;
  let db = shell.documents().await?;
  let documents = Documents::of_shell(shell, db.clone(), web);
  let row = one(
    db,
    r#"SELECT "query", "title", "document" FROM "searches" WHERE "id" = $1"#,
    vec![id.into()],
  )
  .await?
  .ok_or_else(|| VerbError::Refused(format!("archive-share: there is no document {id} in the archive.")))?;
  let document: Option<String> = row.try_get("", "document")?;

  if document.as_deref().is_none_or(str::is_empty) {
    return Err(VerbError::Refused(format!("archive-share: {id} kept no document to hand out.")));
  }

  let title: Option<String> = row.try_get("", "title")?;
  let query: String = row.try_get("", "query")?;
  let owner_room = shell.settings.documents.room == Some(chat_id);
  let handover = documents.hand_over(id, owner_room, life).await?;
  let mut lines = vec![format!("archive-share: document {id}, {}", title.unwrap_or(query))];

  lines.extend(documents.handover_lines(&handover));

  if handover.give.is_none() {
    return Err(VerbError::Partly {
      lines,
      refusal: String::new(),
    });
  }

  Ok(lines)
}

/// What a kept file is, as a document's files are told apart.
pub type Media = &'static str;

/// One file kept beside a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
  pub media: Media,
  pub path: PathBuf,
  pub kind: String,
  pub bytes: u64,
  pub name: String,
}

/// A document written: its number and the files it keeps.
pub struct Written {
  pub id: i64,
  pub attachments: Vec<Kept>,
}

/// What one document is handed to a room with: the address to give, the console's, and the link where one
/// was made; `give` is none where a room of other people was to have a link and none could be made.
pub struct Handover {
  pub owner: bool,
  pub give: Option<String>,
  pub console: String,
  pub share: Option<(String, Option<i64>)>,
}

impl Handover {
  /// The lines a verb answers with: the address to give the room first, then the console's and the link's,
  /// each saying why where there is none. `time` tells the moment a link stops opening.
  pub fn lines(&self, time: impl Fn(i64) -> String) -> Vec<String> {
    let link = if self.owner {
      "none — this is the owner's private room, which is given the console address and never a share link"
        .to_owned()
    } else {
      match &self.share {
        None => "none could be made".to_owned(),
        Some((address, None)) => format!("{address} (never ends)"),
        Some((address, Some(until))) => format!("{address} (until {})", time(*until)),
      }
    };

    vec![
      format!(
        "give: {}",
        self.give.clone().unwrap_or_else(|| {
          "none — this room cannot open the console address, which is behind the console's sign-in, and no \
           share link could be made"
            .to_owned()
        })
      ),
      format!("console: {}", self.console),
      format!("link: {link}"),
    ]
  }
}

/// A moment as the documents tell it, in `zone`, by the wording's time line.
pub fn document_time(seconds: i64, zone: Tz, time: &Time) -> String {
  let Some(at) = chrono::DateTime::from_timestamp(seconds, 0).map(|at| at.with_timezone(&zone)) else {
    return seconds.to_string();
  };
  let (afternoon, hour) = at.hour12();

  fill_marks(
    &time.line,
    &[
      ("year", &at.year().to_string()),
      ("month", &at.month().to_string()),
      ("day", &at.day().to_string()),
      ("half", if afternoon { &time.afternoon } else { &time.morning }),
      ("hour", &hour.to_string()),
      ("minute", &format!("{:02}", at.minute())),
    ],
  )
}

/// The documents of the archive the agent writes, under the file root their files go beneath.
pub struct Documents {
  db: DatabaseConnection,
  artifacts: PathBuf,
  web: String,
  writer: String,
  sender: String,
  wording: ArchiveDocs,
  zone: Tz,
}

impl Documents {
  /// The documents a shell's settings name, handed out under the console's public address `web`.
  pub fn of_shell(shell: &Shell, db: DatabaseConnection, web: String) -> Self {
    let settings = &shell.settings;

    Self {
      db,
      artifacts: settings.files.root.join("searches"),
      web: web.trim_end_matches('/').to_owned(),
      writer: settings.documents.writer.clone(),
      sender: settings.documents.sender.clone(),
      wording: shell.wording.archive_docs.clone(),
      zone: settings.time.zone,
    }
  }

  /// Where the owner reads one document on the console.
  pub fn console_address(&self, id: i64) -> String {
    format!("{}/archive/{id}", self.web)
  }

  /// A moment as this archive's documents tell it.
  pub fn time(&self, seconds: i64) -> String {
    document_time(seconds, self.zone, &self.wording.time)
  }

  /// The lines one document is handed over with.
  pub fn handover_lines(&self, handover: &Handover) -> Vec<String> {
    handover.lines(|seconds| self.time(seconds))
  }

  /// The document written whole: its row, its body as its one finding, its files, the document file. The
  /// input is refused first, and nothing is written for a refusal.
  pub async fn write(&self, input: &DocumentInput) -> Result<Written, String> {
    let plan = plan(input, ATTACHMENT_CEILING)?;
    let id = match input.replace {
      None => self.start(input, &plan).await.map_err(|why| why.detail)?,
      Some(id) => self.restart(id, input, &plan).await?,
    };
    let written_at: i64 =
      one(&self.db, r#"SELECT "started_at" FROM "searches" WHERE "id" = $1"#, vec![id.into()])
        .await
        .map_err(|why| why.detail)?
        .and_then(|row| row.try_get("", "started_at").ok())
        .unwrap_or_else(|| now_ms() / 1000);

    self.add_finding(id, &plan.body).await.map_err(|why| why.detail)?;

    if let Some(topic) = &plan.topic {
      run(
        &self.db,
        r#"UPDATE "searches" SET "topic" = COALESCE($1, "topic") WHERE "id" = $2"#,
        vec![topic.clone().into(), id.into()],
      )
      .await
      .map_err(|why| why.detail)?;
    }

    let mut attachments = Vec::new();

    for path in &plan.attach {
      attachments.push(self.keep(id, Path::new(path)).await?);
    }

    let names: Vec<&str> = attachments.iter().map(|kept| kept.name.as_str()).collect();
    let directory = self.artifacts.join(id.to_string());
    let path = directory.join("document.md");
    let text = self.render(&plan, written_at, &names);

    std::fs::create_dir_all(&directory).map_err(|why| why.to_string())?;
    write_private(&path.with_extension("md.tmp"), text.as_bytes()).map_err(|why| why.to_string())?;
    std::fs::rename(path.with_extension("md.tmp"), &path).map_err(|why| why.to_string())?;
    run(
      &self.db,
      r#"UPDATE "searches" SET "document" = $1, "documented" = 1 WHERE "id" = $2"#,
      vec![path.to_string_lossy().into_owned().into(), id.into()],
    )
    .await
    .map_err(|why| why.detail)?;

    Ok(Written { id, attachments })
  }

  async fn start(&self, input: &DocumentInput, plan: &Plan) -> Result<i64, Refusal> {
    start_document(
      &self.db,
      NewDocument {
        chat_id: input.chat_id,
        request: &plan.title,
        title: &plan.title,
        kind: &plan.kind,
        asker_id: input.asker_id,
        author: &self.writer,
      },
    )
    .await
  }

  // A document written again: refused unless it is the agent's own and none of the files handed over is one
  // it already keeps, since those are let go before the new ones are copied in. What it kept is removed from
  // its folder, and only from there.
  async fn restart(&self, id: i64, input: &DocumentInput, plan: &Plan) -> Result<i64, String> {
    let folder = self.artifacts.join(id.to_string());
    let inside = |path: &Path| {
      let (Ok(real), Ok(folder)) = (std::fs::canonicalize(path), std::fs::canonicalize(&folder)) else {
        return false;
      };

      real.starts_with(&folder) && real != folder
    };

    if let Some(own) = plan.attach.iter().find(|path| inside(Path::new(path))) {
      return Err(format!(
        "{own} is a file document {id} already keeps; attach the file it was copied from."
      ));
    }

    let kept = self.let_go(id, input, plan).await.map_err(|why| why.detail)?;
    let Some(kept) = kept else {
      return Err(format!("there is no document {id} of the agent's own to write again."));
    };

    for path in kept.iter().map(PathBuf::from).filter(|path| inside(path)) {
      let _ = std::fs::remove_file(path);
    }

    Ok(id)
  }

  // The row of a document of the agent's own taken back to its number alone: its body, its files and its head
  // let go, the paths of the files it kept answered; none where it is no document of the agent's.
  async fn let_go(
    &self,
    id: i64,
    input: &DocumentInput,
    plan: &Plan,
  ) -> Result<Option<Vec<String>>, Refusal> {
    let tx = self.db.begin().await?;
    let author: Option<Option<String>> =
      one(&tx, r#"SELECT "author" FROM "searches" WHERE "id" = $1 FOR UPDATE"#, vec![id.into()])
        .await?
        .map(|row| row.try_get("", "author"))
        .transpose()?;

    if author.flatten().as_deref() != Some(self.writer.as_str()) {
      return Ok(None);
    }

    let paths = all(
      &tx,
      r#"SELECT "path" FROM "archive_attachments" WHERE "search_id" = $1"#,
      vec![id.into()],
    )
    .await?
    .iter()
    .map(|row| row.try_get::<String>("", "path"))
    .collect::<Result<Vec<_>, _>>()?;

    run(
      &tx,
      r#"UPDATE "searches" SET "query" = $1, "started_depth" = $2, "ended_depth" = $2, "kind" = $2, "title" = $1,
                               "topic" = NULL, "keywords" = NULL, "rule" = NULL,
                               "asker_id" = COALESCE($3, "asker_id"), "finished_at" = $4
         WHERE "id" = $5"#,
      vec![
        plan.title.clone().into(),
        plan.kind.clone().into(),
        input.asker_id.into(),
        (now_ms() / 1000).into(),
        id.into(),
      ],
    )
    .await?;

    for table in ["search_sources", "search_findings", "archive_attachments"] {
      run(&tx, &format!(r#"DELETE FROM "{table}" WHERE "search_id" = $1"#), vec![id.into()]).await?;
    }

    tx.commit().await?;

    Ok(Some(paths))
  }

  async fn add_finding(&self, id: i64, text: &str) -> Result<(), Refusal> {
    let place: i64 = one(
      &self.db,
      r#"SELECT COALESCE(MAX("place"), 0) AS "place" FROM "search_findings" WHERE "search_id" = $1"#,
      vec![id.into()],
    )
    .await?
    .map(|row| row.try_get::<i64>("", "place"))
    .transpose()?
    .unwrap_or(0);

    run(
      &self.db,
      r#"INSERT INTO "search_findings" ("search_id", "run_id", "branch", "place", "text") VALUES ($1, NULL, NULL, $2, $3)"#,
      vec![id.into(), (place + 1).into(), text.into()],
    )
    .await
    .map(drop)
  }

  /// One file kept beside a document: read for what it is and copied under the document's own folder, a
  /// picture written the way a kept picture is, and its row written. The file handed over is never moved.
  pub async fn keep(&self, id: i64, path: &Path) -> Result<Kept, String> {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();

    self.keep_named(id, path, &name).await
  }

  /// The same file kept under a name of its own rather than the one it stands under on this machine.
  pub async fn keep_named(&self, id: i64, path: &Path, name: &str) -> Result<Kept, String> {
    let shown = path.display();
    let name = name.to_owned();
    let meta = std::fs::metadata(path).map_err(|_| format!("there is no file at {shown}"))?;

    if !meta.is_file() {
      return Err(format!("{shown} is not a file"));
    }

    if meta.len() > ATTACHMENT_CEILING {
      return Err(format!("{shown} weighs {} bytes, over the ceiling of {ATTACHMENT_CEILING}", meta.len()));
    }

    let head = read_head(path).map_err(|_| format!("{shown} cannot be read"))?;
    let (media, kind) = attachment_kind(&head, &name);
    let digest: String = Sha256::digest(path.to_string_lossy().as_bytes())
      .iter()
      .map(|byte| format!("{byte:02x}"))
      .collect::<String>()[..12]
      .to_owned();
    let folder = self.artifacts.join(id.to_string()).join("attachments");
    let extension = Path::new(&name)
      .extension()
      .map(|ext| format!(".{}", ext.to_string_lossy().to_lowercase()))
      .unwrap_or_default();

    std::fs::create_dir_all(&folder).map_err(|why| why.to_string())?;

    let kept = if media == "image" {
      let bytes = std::fs::read(path).map_err(|_| format!("{shown} cannot be read"))?;
      let redrawn = tokio::task::spawn_blocking(move || {
        let redrawn = picture::archived(&bytes, KEPT_PICTURE);

        (bytes, redrawn)
      })
      .await
      .map_err(|why| why.to_string())?;
      let stem = Path::new(&name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
      let (bytes, kind, ending) = match redrawn {
        (_, Some(avif)) => (avif, KEPT_PICTURE_TYPE.to_owned(), ".avif".to_owned()),
        (bytes, None) => (bytes, kind, extension),
      };
      let target = folder.join(format!("{digest}-{}{ending}", file_name(&stem)));

      write_private(&target.with_extension("tmp"), &bytes).map_err(|why| why.to_string())?;
      std::fs::rename(target.with_extension("tmp"), &target).map_err(|why| why.to_string())?;

      Kept {
        media,
        path: target,
        kind,
        bytes: bytes.len() as u64,
        name: name.clone(),
      }
    } else {
      let target = folder.join(format!("{digest}-{}", file_name(&name)));
      let copying =
        target.with_file_name(format!("{}.tmp", target.file_name().unwrap_or_default().to_string_lossy()));

      std::fs::copy(path, &copying).map_err(|why| why.to_string())?;
      std::fs::rename(&copying, &target).map_err(|why| why.to_string())?;

      Kept {
        media,
        path: target,
        kind,
        bytes: meta.len(),
        name: name.clone(),
      }
    };

    keep_attachment(
      &self.db,
      Attachment {
        search_id: id,
        media: kept.media,
        url: None,
        path: &kept.path.to_string_lossy(),
        kind: &kept.kind,
        bytes: i64::try_from(kept.bytes).unwrap_or(i64::MAX),
        name: &kept.name,
        place: None,
      },
    )
    .await
    .map_err(|why| why.detail)?;

    Ok(kept)
  }

  /// Whether one document keeps a file a reader opens; none of a search that answered one value.
  pub async fn has_document(&self, id: i64) -> bool {
    one(&self.db, r#"SELECT "document" FROM "searches" WHERE "id" = $1"#, vec![id.into()])
      .await
      .ok()
      .flatten()
      .and_then(|row| row.try_get::<Option<String>>("", "document").ok().flatten())
      .is_some_and(|document| !document.is_empty())
  }

  /// One document handed to a room: the console address in the owner's private room, and the share link
  /// everywhere else — the one that stands, or one made now living `life`. A link asked to live with no end
  /// takes the end off the one that stands.
  pub async fn hand_over(&self, id: i64, owner: bool, life: ShareLife) -> Result<Handover, Refusal> {
    let share = if owner { None } else { self.share_of(id, life).await? };
    let console = self.console_address(id);

    Ok(Handover {
      owner,
      give: if owner {
        Some(console.clone())
      } else {
        share.as_ref().map(|(address, _)| address.clone())
      },
      console,
      share,
    })
  }

  // The link to one document, made where none stands, with when it stops opening; none where the document
  // keeps nothing to hand out.
  async fn share_of(&self, id: i64, life: ShareLife) -> Result<Option<(String, Option<i64>)>, Refusal> {
    let document: Option<String> =
      one(&self.db, r#"SELECT "document" FROM "searches" WHERE "id" = $1"#, vec![id.into()])
        .await?
        .map(|row| row.try_get::<Option<String>>("", "document"))
        .transpose()?
        .flatten();

    if document.as_deref().is_none_or(str::is_empty) {
      return Ok(None);
    }

    let now = now_ms() / 1000;
    let tx = self.db.begin().await?;

    run(
      &tx,
      r#"DELETE FROM "web_search_shares" WHERE "expires_at" IS NOT NULL AND "expires_at" <= $1"#,
      vec![now.into()],
    )
    .await?;

    let standing = live_share(&tx, id).await?;
    let share = match standing {
      Some((token, Some(_))) if life == ShareLife::Endless => {
        run(
          &tx,
          r#"UPDATE "web_search_shares" SET "expires_at" = NULL WHERE "search_id" = $1"#,
          vec![id.into()],
        )
        .await?;

        (token, None)
      }
      Some(share) => share,
      None => {
        let token = URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>());
        let expires = match life {
          ShareLife::Seconds(seconds) => Some(now + seconds),
          ShareLife::Endless => None,
        };

        run(
          &tx,
          r#"INSERT INTO "web_search_shares" ("share_token", "search_id", "created_by", "created_at", "expires_at")
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING"#,
          vec![token.into(), id.into(), self.sender.clone().into(), now.into(), expires.into()],
        )
        .await?;

        live_share(&tx, id)
          .await?
          .ok_or_else(|| Refusal::bad("the share link was not written"))?
      }
    };

    tx.commit().await?;

    Ok(Some((format!("{}/share/{}", self.web, share.0), share.1)))
  }

  /// The document file: the title, the kind and the facts of the document as its header, and the body under
  /// it as the agent wrote it.
  fn render(&self, plan: &Plan, written_at: i64, attachments: &[&str]) -> String {
    render(&self.wording, &plan.kind, &plan.title, &self.time(written_at), attachments, &plan.body)
  }
}

async fn live_share(db: &impl ConnectionTrait, id: i64) -> Result<Option<(String, Option<i64>)>, Refusal> {
  one(
    db,
    r#"SELECT "share_token", "expires_at" FROM "web_search_shares" WHERE "search_id" = $1"#,
    vec![id.into()],
  )
  .await?
  .map(|row| Ok((row.try_get("", "share_token")?, row.try_get("", "expires_at")?)))
  .transpose()
  .map_err(|why: sea_orm::DbErr| Refusal::from(why))
}

// Owner-only from the moment the file exists.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
  use std::io::Write as _;
  use std::os::unix::fs::OpenOptionsExt as _;

  std::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .mode(0o600)
    .open(path)?
    .write_all(bytes)
}

/// What is written, once every part has been checked: the row's kind and title, the body that stands as its
/// one finding, the files it keeps and the topic its head shows.
pub struct Plan {
  pub kind: String,
  pub title: String,
  pub body: String,
  pub attach: Vec<String>,
  pub topic: Option<String>,
}

// Why one file cannot be kept, or none where it can.
fn file_refusal(path: &str, ceiling: u64) -> Option<String> {
  let Ok(meta) = std::fs::metadata(path) else {
    return Some(format!("{path} cannot be read."));
  };

  if !meta.is_file() {
    return Some(format!("{path} is not a file."));
  }

  if std::fs::File::open(path).is_err() {
    return Some(format!("{path} cannot be read."));
  }

  (meta.len() > ceiling).then(|| {
    format!(
      "{path} weighs {} bytes, over the ceiling of {}.",
      count_text(usize::try_from(meta.len()).unwrap_or(usize::MAX)),
      count_text(usize::try_from(ceiling).unwrap_or(usize::MAX))
    )
  })
}

/// What a document is to be, or the reason it cannot be, in words the chat session reads and passes on.
pub fn plan(input: &DocumentInput, ceiling: u64) -> Result<Plan, String> {
  if let Some(html) = &input.html {
    return page_plan(input, html, ceiling);
  }

  if !AGENT_KINDS.contains(&input.kind.as_str()) {
    return Err(format!(
      "--kind is one of {}; a page is filed with --html <file>, and a search is the searcher's to write.",
      AGENT_KINDS.join(", ")
    ));
  }

  if input.title.trim().is_empty() {
    return Err("a document needs a title: --title '<title>'.".to_owned());
  }

  if input.body.trim().is_empty() {
    return Err("the body is empty: give it with --body <file> or on standing input.".to_owned());
  }

  if let Some(refusal) = input.attach.iter().find_map(|path| file_refusal(path, ceiling)) {
    return Err(refusal);
  }

  Ok(Plan {
    kind: input.kind.clone(),
    title: input.title.trim().to_owned(),
    body: input.body.trim().to_owned(),
    attach: input.attach.clone(),
    topic: None,
  })
}

// A page: the file is the document, and its title and description are read out of it without a byte of it
// changed.
fn page_plan(input: &DocumentInput, html: &str, ceiling: u64) -> Result<Plan, String> {
  if !input.body.trim().is_empty() {
    return Err("a page is its own document: give it no body.".to_owned());
  }

  if !input.attach.is_empty() {
    return Err("a page carries what it draws inside itself: give it no --attach.".to_owned());
  }

  if let Some(refusal) = file_refusal(html, ceiling) {
    return Err(refusal);
  }

  let path = Path::new(html);
  let head = read_head(path).map_err(|_| format!("{html} cannot be read."))?;
  let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();

  if attachment_kind(&head, &name).0 != "page" {
    return Err(format!("{html} is not an HTML page."));
  }

  let text = String::from_utf8_lossy(&std::fs::read(path).map_err(|_| format!("{html} cannot be read."))?)
    .into_owned();
  let reading = read_html(&text, PICTURE_STEM);
  let title = Some(input.title.trim().to_owned())
    .filter(|title| !title.is_empty())
    .or(reading.title.clone());
  let Some(title) = title else {
    return Err("the page has no <title>: give it one with --title '<title>'.".to_owned());
  };
  let description = page_description(&reading);

  Ok(Plan {
    kind: PAGE_KIND.to_owned(),
    body: description.clone().unwrap_or_else(|| title.clone()),
    title,
    attach: vec![html.to_owned()],
    topic: description,
  })
}

/// The document file: the title, the kind by its label in `wording`, the request, the moment it was
/// written as `time` tells it and the files attached as its header, and the body under it.
pub fn render(
  wording: &ArchiveDocs,
  kind: &str,
  title: &str,
  time: &str,
  attachments: &[&str],
  body: &str,
) -> String {
  let header = &wording.header;
  let label = wording.kinds.get(kind).map_or(kind, String::as_str);
  let mut lines = vec![
    format!("# {title}"),
    String::new(),
    fill_marks(&header.kind, &[("kind", label)]),
    fill_marks(&header.request, &[("request", title)]),
    fill_marks(&header.written, &[("time", time)]),
  ];

  if !attachments.is_empty() {
    lines.push(fill_marks(&header.attached, &[("names", &attachments.join(", "))]));
  }

  lines.push(String::new());
  lines.push(body.trim().to_owned());

  format!("{}\n", lines.join("\n"))
}

fn read_head(path: &Path) -> std::io::Result<Vec<u8>> {
  use std::io::Read as _;

  let mut head = Vec::with_capacity(SNIFF_BYTES);

  std::fs::File::open(path)?.take(SNIFF_BYTES as u64).read_to_end(&mut head)?;

  Ok(head)
}

// What each file ending names, where a file's own bytes do not settle what it is.
fn named_type(extension: &str) -> Option<(Media, &'static str)> {
  Some(match extension {
    ".jpg" | ".jpeg" => ("image", "image/jpeg"),
    ".png" => ("image", "image/png"),
    ".gif" => ("image", "image/gif"),
    ".webp" => ("image", "image/webp"),
    ".avif" => ("image", "image/avif"),
    ".svg" => ("image", "image/svg+xml"),
    ".mp3" => ("audio", "audio/mpeg"),
    ".m4a" => ("audio", "audio/mp4"),
    ".aac" => ("audio", "audio/aac"),
    ".ogg" | ".oga" | ".opus" => ("audio", "audio/ogg"),
    ".wav" => ("audio", "audio/wav"),
    ".flac" => ("audio", "audio/flac"),
    ".weba" => ("audio", "audio/webm"),
    ".mp4" | ".m4v" => ("video", "video/mp4"),
    ".mov" => ("video", "video/quicktime"),
    ".webm" => ("video", "video/webm"),
    ".mkv" => ("video", "video/x-matroska"),
    ".ogv" => ("video", "video/ogg"),
    ".avi" => ("video", "video/x-msvideo"),
    ".txt" | ".log" => ("text", "text/plain"),
    ".md" => ("text", "text/markdown"),
    ".csv" => ("text", "text/csv"),
    ".tsv" => ("text", "text/tab-separated-values"),
    ".json" => ("text", "application/json"),
    ".yaml" | ".yml" => ("text", "text/yaml"),
    ".html" | ".htm" => ("page", "text/html"),
    ".pdf" => ("file", "application/pdf"),
    ".zip" => ("file", "application/zip"),
    _ => return None,
  })
}

// What an HTML document opens with, after any byte-order mark, space and comment. The file is only told
// apart here, never read: the sniff looks at the first bytes of a head already cut, and it must let comments
// stand before the doctype, which the content sniffers on the registry do not.
static HTML_START: LazyLock<Regex> = LazyLock::new(|| {
  RegexBuilder::new(r"^\u{FEFF}?\s*(?:<!--[\s\S]*?-->\s*)*<(?:!doctype\s+html|html)[\s>]")
    .case_insensitive(true)
    .build()
    .expect("the HTML opening is a pattern")
});

/// What a file is, read from its first bytes and, where the bytes do not settle it, its name: the bytes
/// decide first, and the name decides between the forms one container holds. A text is what decodes as UTF-8
/// with no NUL in it; anything else is a file that is only handed over.
pub fn attachment_kind(head: &[u8], name: &str) -> (Media, String) {
  let extension = Path::new(name)
    .extension()
    .map(|ext| format!(".{}", ext.to_string_lossy().to_lowercase()))
    .unwrap_or_default();
  let named = named_type(&extension);
  let at = |offset: usize, value: &[u8]| head.get(offset..offset + value.len()) == Some(value);
  let owned = |(media, kind): (Media, &str)| (media, kind.to_owned());
  let named_as = |media: Media| named.filter(|(named, _)| *named == media);

  if let Some(raster) = picture::raster_type(head) {
    return ("image", raster.to_owned());
  }

  if at(0, b"OggS") {
    return owned(named_as("video").unwrap_or(("audio", "audio/ogg")));
  }

  if at(0, &[0x1a, 0x45, 0xdf, 0xa3]) {
    let webm = if extension == ".mkv" { "video/x-matroska" } else { "video/webm" };

    return owned(named_as("audio").unwrap_or(("video", webm)));
  }

  if at(0, b"fLaC") {
    return owned(("audio", "audio/flac"));
  }

  if at(0, b"RIFF") && at(8, b"WAVE") {
    return owned(("audio", "audio/wav"));
  }

  if at(0, b"RIFF") && at(8, b"AVI ") {
    return owned(("video", "video/x-msvideo"));
  }

  if at(4, b"ftyp") {
    if at(8, b"M4A ") || at(8, b"M4B ") {
      return owned(("audio", "audio/mp4"));
    }

    if at(8, b"qt  ") {
      return owned(("video", "video/quicktime"));
    }

    return owned(named_as("audio").unwrap_or(("video", "video/mp4")));
  }

  match head {
    [0xff, second, ..] if second & 0xf6 == 0xf0 => return owned(("audio", "audio/aac")),
    [0xff, second, ..] if second & 0xe0 == 0xe0 => return owned(("audio", "audio/mpeg")),
    _ if at(0, b"ID3") => return owned(("audio", "audio/mpeg")),
    _ if at(0, b"%PDF") => return owned(("file", "application/pdf")),
    _ => {}
  }

  if !head.contains(&0) && is_text(head) {
    // An SVG is markup and decodes as text, and it is still a picture; an HTML document is markup too, and
    // it is a page, whether its name says so or its first tag does.
    if let Some(named) = named_as("image").or_else(|| named_as("page")) {
      return owned(named);
    }

    if HTML_START.is_match(&String::from_utf8_lossy(&head[..head.len().min(256)])) {
      return owned(("page", "text/html"));
    }

    return owned(named_as("text").unwrap_or(("text", "text/plain")));
  }

  owned(named_as("file").unwrap_or(("file", "application/octet-stream")))
}

// Whether a head decodes as UTF-8, a character cut at the sniffing limit no reason to call it binary.
fn is_text(head: &[u8]) -> bool {
  match std::str::from_utf8(head) {
    Ok(_) => true,
    Err(error) => head.len() == SNIFF_BYTES && error.error_len().is_none(),
  }
}

/// A name that can stand as a file under the document's folder: letters of any script, digits and a few
/// marks, and nothing that could climb out of it.
pub fn file_name(name: &str) -> String {
  static UNSAFE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\p{L}\p{N}._-]+").expect("a pattern"));

  let composed: String = name.nfc().collect();
  let cleaned = UNSAFE.replace_all(&composed, "_");
  let cleaned: String = cleaned.trim_start_matches('.').chars().take(100).collect();

  if cleaned.is_empty() { "file".to_owned() } else { cleaned }
}

/// An HTML page read for what it says: its title, what its description meta says of it, and its words a
/// block to a line, a heading keeping its level and a list item its dash, a picture standing as a markdown
/// picture where it stood; scripts and styles are not words. A picture the page carries inside itself as a data
/// address is taken out as a file of its own, named after the reading's stem in the order the pictures stood, and
/// stands in the words under that name.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Reading {
  pub title: Option<String>,
  pub description: Option<String>,
  pub lines: Vec<String>,
  pub pictures: Vec<Picture>,
}

/// A picture a page carried inside itself: the name it stands under in the reading and is kept by, its type, and
/// its bytes.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Picture {
  pub name: String,
  pub kind: String,
  pub bytes: Vec<u8>,
}

/// What the pictures taken out of a page are named after where nothing else is asked.
pub const PICTURE_STEM: &str = "picture";

// The ending a picture's file takes by its type; any other type ends in its own subtype.
const PICTURE_ENDINGS: [(&str, &str); 6] = [
  ("jpeg", "jpg"),
  ("png", "png"),
  ("gif", "gif"),
  ("webp", "webp"),
  ("avif", "avif"),
  ("svg+xml", "svg"),
];

// The tags that start a line of their own, and those whose insides are not words.
const BLOCKS: [&str; 31] = [
  "address",
  "article",
  "aside",
  "blockquote",
  "br",
  "button",
  "dd",
  "details",
  "div",
  "dl",
  "dt",
  "figcaption",
  "figure",
  "footer",
  "form",
  "header",
  "hr",
  "li",
  "main",
  "nav",
  "ol",
  "p",
  "pre",
  "section",
  "small",
  "summary",
  "table",
  "td",
  "th",
  "tr",
  "ul",
];
const UNREAD: [&str; 6] = ["script", "style", "noscript", "template", "svg", "head"];

#[derive(Default)]
struct Walk {
  reading: Reading,
  stem: String,
  line: String,
  prefix: String,
  title: Option<String>,
  in_title: bool,
  unread: usize,
}

fn collapsed(text: &str) -> String {
  text.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Walk {
  // What a picture's source stands as in the words: a data address taken out under the next name, anything else
  // as it is given.
  fn picture(&mut self, source: &str, alt: &str) -> String {
    let taken = data_url::DataUrl::process(source).ok().and_then(|data| {
      let kind = data.mime_type();

      (kind.type_ == "image").then(|| (format!("{}/{}", kind.type_, kind.subtype), data.decode_to_vec()))
    });
    let Some((kind, Ok((bytes, _)))) = taken else {
      return format!("![{alt}]({source})");
    };
    let subtype = kind.trim_start_matches("image/");
    let ending = PICTURE_ENDINGS
      .iter()
      .find(|(named, _)| *named == subtype)
      .map_or_else(|| subtype.split('+').next().unwrap_or(subtype), |(_, ending)| ending);
    let name = format!("{}-{:02}.{ending}", self.stem, self.reading.pictures.len() + 1);

    self.reading.pictures.push(Picture {
      name: name.clone(),
      kind,
      bytes,
    });

    format!("![{alt}]({name})")
  }

  fn flush(&mut self) {
    let words = collapsed(&std::mem::take(&mut self.line));

    if !words.is_empty() {
      self.reading.lines.push(format!("{}{words}", std::mem::take(&mut self.prefix)));
    }
  }

  fn children(&mut self, node: ego_tree::NodeRef<'_, Node>) {
    for child in node.children() {
      self.node(child);
    }
  }

  fn node(&mut self, node: ego_tree::NodeRef<'_, Node>) {
    match node.value() {
      Node::Text(text) => {
        if self.in_title {
          self.title.get_or_insert_with(String::new).push_str(text);
        } else if self.unread == 0 {
          self.line.push_str(text);
        }
      }
      Node::Element(element) => {
        let tag = element.name().to_ascii_lowercase();

        if tag == "meta" {
          let named = element.attr("name").or_else(|| element.attr("property")).unwrap_or_default();

          if self.reading.description.is_none()
            && (named.eq_ignore_ascii_case("description") || named.eq_ignore_ascii_case("og:description"))
          {
            let content = collapsed(element.attr("content").unwrap_or_default());

            if !content.is_empty() {
              self.reading.description = Some(content);
            }
          }

          return;
        }

        if tag == "title" {
          self.in_title = true;
          self.children(node);
          self.in_title = false;
          return;
        }

        if UNREAD.contains(&tag.as_str()) {
          self.unread += 1;
          self.children(node);
          self.unread -= 1;
          return;
        }

        if self.unread > 0 {
          return self.children(node);
        }

        // Words in two neighbouring elements are two words, however the page set them side by side.
        self.line.push(' ');

        let heading = tag
          .strip_prefix('h')
          .and_then(|level| level.parse::<usize>().ok())
          .filter(|level| (1..=6).contains(level));

        if tag == "img" {
          self.flush();

          let line =
            self.picture(element.attr("src").unwrap_or_default(), element.attr("alt").unwrap_or_default());

          self.reading.lines.push(line);
          return;
        }

        if heading.is_some() || BLOCKS.contains(&tag.as_str()) {
          self.flush();

          if let Some(level) = heading {
            self.prefix = format!("{} ", "#".repeat(level));
          }

          if tag == "li" {
            self.prefix = "- ".to_owned();
          }

          self.children(node);

          if tag != "br" && tag != "hr" && self.unread == 0 {
            self.flush();
          }

          return;
        }

        self.children(node);
      }
      _ => self.children(node),
    }
  }
}

/// The reading of one page, the pictures inside it named after `stem`.
pub fn read_html(html: &str, stem: &str) -> Reading {
  let document = Html::parse_document(html);
  let mut walk = Walk {
    stem: stem.to_owned(),
    ..Walk::default()
  };

  walk.node(document.tree.root());
  walk.flush();

  let title = walk.title.map(|title| collapsed(&title)).filter(|title| !title.is_empty());

  Reading {
    title,
    ..walk.reading
  }
}

/// What a page is said to be where it is listed: its own description, and failing that the start of its
/// words — neither its title again, nor a picture, nor a heading's or a list's mark — cut at a space near the
/// length and marked as cut. None for a page with no words at all.
pub fn page_description(reading: &Reading) -> Option<String> {
  static MARK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:#{1,6} |- )").expect("a pattern"));

  if let Some(description) = &reading.description {
    return Some(description.clone());
  }

  let words: Vec<String> = reading
    .lines
    .iter()
    .filter(|line| !line.starts_with("!["))
    .map(|line| MARK.replace(line, "").into_owned())
    .enumerate()
    .filter(|(index, line)| !(*index == 0 && Some(line) == reading.title.as_ref()))
    .map(|(_, line)| line)
    .collect();
  let words = words.join(" ").trim().to_owned();

  if words.is_empty() {
    return None;
  }

  if words.chars().count() <= DESCRIPTION_LENGTH {
    return Some(words);
  }

  let cut: String = words.chars().take(DESCRIPTION_LENGTH).collect();
  let space = cut.char_indices().rev().find(|(_, character)| *character == ' ').map(|(at, _)| at);
  let kept = match space {
    Some(at) if cut[..at].chars().count() > DESCRIPTION_LENGTH / 2 => &cut[..at],
    _ => cut.as_str(),
  };

  Some(format!("{}…", kept.trim_end()))
}
