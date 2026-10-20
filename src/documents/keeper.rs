//! One file kept beside a document (`docs/archive.md`, "Attachments"): read for what it is from its bytes
//! and its name, written under the document's own folder, a picture redrawn the way a kept picture is, and
//! its row written. Nothing is written for a refusal.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};
use sea_orm::ConnectionTrait;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use crate::Refusal;
use crate::helpers::picture;
use crate::store::attachments::{self, NewAttachment};
use crate::wire::Attachment;

/// What a kept file is read as: `image`, `audio`, `video`, `text`, `page` or `file`.
pub type Media = &'static str;

/// The most one file handed over through the API may weigh.
pub const ATTACHMENT_CEILING: usize = 64 * 1024 * 1024;

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

/// A file to keep: its name, its bytes, the type its giver says it is, its role, and the address it came
/// from where it came from one.
pub struct Handed<'a> {
  pub name: &'a str,
  pub bytes: Vec<u8>,
  pub media_type: Option<&'a str>,
  pub role: Option<&'a str>,
  pub url: Option<&'a str>,
}

/// The folder a document's files stand in under the file root.
pub fn document_folder(root: &Path, document_id: i64) -> PathBuf {
  root.join("searches").join(document_id.to_string())
}

/// One file kept beside a document: refused where it has no name, is over the ceiling or bears a name the
/// document already keeps; otherwise written under `attachments/`, a picture as AVIF where it can be, and its
/// row written. The attachment as the API answers it.
pub async fn keep<C: ConnectionTrait>(
  db: &C,
  root: &Path,
  document_id: i64,
  handed: Handed<'_>,
) -> Result<Attachment, Refusal> {
  let name = handed.name.trim();

  if name.is_empty() {
    return Err(Refusal::bad("an attachment needs a name"));
  }

  if handed.bytes.len() > ATTACHMENT_CEILING {
    return Err(Refusal::too_large(format!(
      "{name} weighs {} bytes, over the ceiling of {ATTACHMENT_CEILING}",
      handed.bytes.len()
    )));
  }

  if attachments::named(db, document_id, name).await?.is_some() {
    return Err(Refusal::bad(format!("document {document_id} already keeps a file named {name}")));
  }

  let head = &handed.bytes[..handed.bytes.len().min(SNIFF_BYTES)];
  let (media, sniffed) = attachment_kind(head, name);
  let kind = match (sniffed.as_str(), handed.media_type) {
    ("application/octet-stream" | "text/plain", Some(given)) if !given.trim().is_empty() => {
      given.trim().to_owned()
    }
    _ => sniffed,
  };
  let digest: String = Sha256::digest(name.as_bytes())
    .iter()
    .map(|byte| format!("{byte:02x}"))
    .collect::<String>()[..12]
    .to_owned();
  let folder = document_folder(root, document_id).join("attachments");
  let extension = Path::new(name)
    .extension()
    .map(|ext| format!(".{}", ext.to_string_lossy().to_lowercase()))
    .unwrap_or_default();
  let (bytes, kind, target) = if media == "image" {
    let bytes = handed.bytes;
    let (bytes, redrawn) = tokio::task::spawn_blocking(move || {
      let redrawn = picture::archived(&bytes, KEPT_PICTURE);

      (bytes, redrawn)
    })
    .await
    .map_err(|why| Refusal::failed(why.to_string()))?;
    let stem = Path::new(name)
      .file_stem()
      .map(|stem| stem.to_string_lossy().into_owned())
      .unwrap_or_default();

    match redrawn {
      Some(avif) => (
        avif,
        KEPT_PICTURE_TYPE.to_owned(),
        folder.join(format!("{digest}-{}.avif", file_name(&stem))),
      ),
      None => (bytes, kind, folder.join(format!("{digest}-{}{extension}", file_name(&stem)))),
    }
  } else {
    (handed.bytes, kind, folder.join(format!("{digest}-{}", file_name(name))))
  };

  std::fs::create_dir_all(&folder).map_err(|why| Refusal::failed(why.to_string()))?;
  write_private(&target, &bytes).map_err(|why| Refusal::failed(why.to_string()))?;

  attachments::insert(
    db,
    NewAttachment {
      document_id,
      media,
      url: handed.url,
      path: &target.to_string_lossy(),
      media_type: &kind,
      byte_count: i64::try_from(bytes.len()).unwrap_or(i64::MAX),
      name,
      role: handed.role.map(str::trim).filter(|role| !role.is_empty()),
    },
  )
  .await
}

/// A file written whole and owner-only, through a staged file beside it, so a reader never sees half of it.
pub fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
  use std::io::Write as _;
  use std::os::unix::fs::OpenOptionsExt as _;

  let staged = path.with_file_name(format!(
    "{}.{}.tmp",
    path.file_name().unwrap_or_default().to_string_lossy(),
    std::process::id()
  ));

  std::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .mode(0o600)
    .open(&staged)?
    .write_all(bytes)?;
  std::fs::rename(&staged, path)
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
