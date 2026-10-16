//! The pictures a search's result names, kept as this repository's own copies (`docs/archive.md`, "The
//! pictures a search keeps"). A result points at pictures on other people's servers, and those addresses are
//! the first thing about a search to rot, so its IMAGE lines are read, each address fetched once through the
//! page reader, written beside the search's document the way every kept picture is, and remembered under the
//! address it came from. The run itself keeps nothing, being shown the web and its branches and neither a
//! file tool nor a shell.
//!
//! The copies are a record of the search and not the search: past the keeping's deadline what has been kept is
//! what is kept, and the rest of the result goes on naming its addresses.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use regex::Regex;
use sea_orm::DatabaseConnection;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use super::web::WebLayer;
use crate::helpers::picture::archived;
use crate::verbs::archive_docs::{KEPT_PICTURE, KEPT_PICTURE_TYPE};
use crate::{Refusal, run};

/// The most pictures one search keeps: a result is written by a model and could name any number.
pub const KEPT_LIMIT: usize = 100;

// How many are fetched and converted at once.
const LANES: usize = 4;

/// How long the whole keeping may take: the room is waiting, and the answer is never held up for a picture.
pub const KEEPING_DEADLINE: Duration = Duration::from_secs(60);

static IMAGE_LINE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^[ \t]*IMAGE:[ \t]*(\S+)(?:[ \t]*::[ \t]*(.*?))?[ \t]*$").expect("a pattern")
});

fn is_web_address(value: &str) -> bool {
  url::Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

/// The web addresses the result's IMAGE lines name, in order, each once; a line already naming a kept file is
/// passed over.
pub fn image_addresses(result: &str) -> Vec<String> {
  let mut addresses: Vec<String> = vec![];

  for line in result.split('\n') {
    if let Some(found) = IMAGE_LINE.captures(line) {
      let address = &found[1];

      if is_web_address(address) && !addresses.iter().any(|known| known == address) {
        addresses.push(address.to_owned());
      }
    }
  }

  addresses
}

/// The file one address is kept as: named by the address's own digest, so a picture named twice is one file.
pub fn kept_path(artifacts: &Path, search_id: i64, address: &str) -> PathBuf {
  let digest: String =
    Sha256::digest(address.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect();

  artifacts
    .join(search_id.to_string())
    .join("images")
    .join(format!("{}.avif", &digest[..32]))
}

/// The result as the room's session is handed it: every IMAGE line whose address was kept names the file
/// instead, which the photo verb takes exactly as it takes an address; a line not kept is left alone.
pub fn with_kept(result: &str, kept: &BTreeMap<String, String>) -> String {
  if kept.is_empty() {
    return result.to_owned();
  }

  result
    .split('\n')
    .map(|line| {
      let Some(found) = IMAGE_LINE.captures(line) else { return line.to_owned() };
      let Some(path) = kept.get(&found[1]) else { return line.to_owned() };

      match found.get(2).map(|caption| caption.as_str()).filter(|caption| !caption.is_empty()) {
        None => format!("IMAGE: {path}"),
        Some(caption) => format!("IMAGE: {path} :: {caption}"),
      }
    })
    .collect::<Vec<_>>()
    .join("\n")
}

/// The source an inline SVG was drawn from as a file of its own, which has to carry the namespace the page
/// left to the HTML around it.
pub fn standalone_svg(svg: &str) -> String {
  static NAMED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<svg\b[^>]*\bxmlns=").expect("a pattern"));

  if NAMED.is_match(svg) {
    svg.to_owned()
  } else {
    svg.replacen("<svg", r#"<svg xmlns="http://www.w3.org/2000/svg""#, 1)
  }
}

/// One figure a page import captured: the address the result names it by, its PNG, and the SVG it was drawn
/// from where it was one.
pub struct CapturedFigure {
  pub address: String,
  pub png: Vec<u8>,
  pub svg: Option<String>,
}

/// Where a search's pictures go and are written down.
pub struct Keeping<'a> {
  pub db: &'a DatabaseConnection,
  pub search_id: i64,
  /// Where the searches' folders live: the data root's `searches`.
  pub artifacts: &'a Path,
  /// How long the keeping of a result's pictures may take: `KEEPING_DEADLINE` for a run.
  pub deadline: Duration,
}

impl Keeping<'_> {
  // A picture's bytes converted the way every kept picture is and written under the address's own name,
  // owner-only and whole or not at all, and its row written; where it was kept. A picture that will not
  // convert is kept as it came.
  async fn keep(&self, address: &str, bytes: Vec<u8>, kind: String) -> Result<String, String> {
    let (bytes, kind) = tokio::task::spawn_blocking(move || match archived(&bytes, KEPT_PICTURE) {
      Some(avif) => (avif, KEPT_PICTURE_TYPE.to_owned()),
      None => (bytes, kind),
    })
    .await
    .map_err(|why| why.to_string())?;
    let path = kept_path(self.artifacts, self.search_id, address);

    write_private(&path, &bytes).map_err(|why| why.to_string())?;
    self.note(address, &path, &kind, bytes.len()).await.map_err(|why| why.detail)?;

    Ok(path.to_string_lossy().into_owned())
  }

  async fn note(&self, address: &str, path: &Path, kind: &str, bytes: usize) -> Result<(), Refusal> {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();

    run(
      self.db,
      r#"INSERT INTO "archive_attachments" ("search_id", "media", "url", "path", "type", "bytes", "name", "place", "kept_at")
         VALUES ($1, 'image', $2, $3, $4, $5, $6, NULL, $7)
         ON CONFLICT ("search_id", "path") DO UPDATE SET
           "media" = excluded."media", "url" = excluded."url", "type" = excluded."type", "bytes" = excluded."bytes",
           "name" = excluded."name", "place" = excluded."place", "kept_at" = excluded."kept_at""#,
      vec![
        self.search_id.into(),
        address.into(),
        path.to_string_lossy().into_owned().into(),
        kind.into(),
        i64::try_from(bytes).unwrap_or(i64::MAX).into(),
        name.into(),
        (crate::now_ms() / 1000).into(),
      ],
    )
    .await?;

    Ok(())
  }

  /// Every picture the result named, kept, beside those `already` kept by other means under the addresses the
  /// result names them by: the address of each one kept and its file. One that could not be fetched or was no
  /// picture is not in it, and the result goes on naming its address.
  pub async fn result_pictures(
    &self,
    web: &WebLayer,
    result: &str,
    already: BTreeMap<String, String>,
  ) -> BTreeMap<String, String> {
    let closes = Instant::now() + self.deadline;
    let addresses: Vec<String> = image_addresses(result)
      .into_iter()
      .filter(|address| !already.contains_key(address))
      .take(KEPT_LIMIT.saturating_sub(already.len()))
      .collect();
    let lanes = LANES.min(addresses.len());
    let queue = Mutex::new(addresses.into_iter());
    let kept = Mutex::new(already);
    let lane = async || {
      loop {
        let Some(address) = queue.lock().await.next() else { return };

        if Instant::now() >= closes {
          let left = queue.lock().await.by_ref().count() + 1;

          eprintln!("the search's pictures ran out of time with {left} left; those keep their addresses");
          return;
        }

        let stored = match web.picture(&address).await {
          Ok(picture) => self.keep(&address, picture.bytes, picture.kind).await,
          Err(why) => Err(why),
        };

        match stored {
          Ok(path) => {
            kept.lock().await.insert(address, path);
          }
          Err(why) => eprintln!("the picture at {address} was not kept: {why}"),
        }
      }
    };

    futures_util::future::join_all((0..lanes).map(|_| lane())).await;
    kept.into_inner()
  }

  /// Every figure a page import captured, kept the way a fetched picture is under the address the result names
  /// it by, an SVG's own source written beside its picture: the address of each one kept and its file.
  pub async fn captured(&self, figures: Vec<CapturedFigure>) -> BTreeMap<String, String> {
    let mut kept = BTreeMap::new();

    for figure in figures.into_iter().take(KEPT_LIMIT) {
      match self.keep(&figure.address, figure.png, "image/png".to_owned()).await {
        Ok(path) => {
          if let Some(svg) = &figure.svg
            && let Err(why) =
              write_private(&Path::new(&path).with_extension("svg"), standalone_svg(svg).as_bytes())
          {
            eprintln!("the figure at {} kept no drawing: {why}", figure.address);
          }

          kept.insert(figure.address, path);
        }
        Err(why) => eprintln!("the figure at {} was not kept: {why}", figure.address),
      }
    }

    kept
  }
}

// Owner-only from the moment the file exists, and whole or not at all.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
  use std::io::Write;
  use std::os::unix::fs::OpenOptionsExt;

  if let Some(folder) = path.parent() {
    std::fs::create_dir_all(folder)?;
  }

  let staged = path.with_extension("tmp");
  let mut file = std::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .mode(0o600)
    .open(&staged)?;

  file.write_all(bytes)?;
  std::fs::rename(&staged, path)
}
