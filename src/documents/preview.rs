//! What a referred page shows of itself before anyone opens it, its title, its description and its picture,
//! read through the page reader for a caller who is not to reach the outside server directly
//! (`docs/api.md`, "References"). What was read is kept under the file root, so a page seen once is not asked
//! again for a while, and neither is one that could not be reached: a preview for a week, a picture for a
//! month, a failure for a day.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex, PoisonError};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::helpers::picture::raster_type;
use crate::now_ms;
use crate::reader::{PageReader, Picture, Preview};

/// How long a preview is kept.
pub const PREVIEW_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;
/// How long a picture is kept.
pub const IMAGE_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000;
/// How long a failure is kept before the page is asked again.
pub const FAILURE_TTL_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewEntry {
  fetched_at: i64,
  preview: Option<Preview>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageEntry {
  fetched_at: i64,
  media_type: Option<String>,
}

/// The previews and pictures read, kept under one folder.
pub struct PreviewCache {
  directory: PathBuf,
  reader: Arc<dyn PageReader>,
  now: Box<dyn Fn() -> i64 + Send + Sync>,
  // One read of an address at a time: a second asker waits for the first and takes what it kept.
  flying: StdMutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl std::fmt::Debug for PreviewCache {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter
      .debug_struct("PreviewCache")
      .field("directory", &self.directory)
      .finish_non_exhaustive()
  }
}

impl PreviewCache {
  pub fn new(directory: PathBuf, reader: Arc<dyn PageReader>) -> Self {
    Self::with_clock(directory, reader, Box::new(now_ms))
  }

  /// The cache on a clock of the caller's.
  pub fn with_clock(
    directory: PathBuf,
    reader: Arc<dyn PageReader>,
    now: Box<dyn Fn() -> i64 + Send + Sync>,
  ) -> Self {
    Self {
      directory,
      reader,
      now,
      flying: StdMutex::default(),
    }
  }

  fn fresh(&self, fetched_at: i64, ttl: i64) -> bool {
    (0..ttl).contains(&((self.now)() - fetched_at))
  }

  fn lane(&self, key: &str) -> Arc<Mutex<()>> {
    let mut flying = self.flying.lock().unwrap_or_else(PoisonError::into_inner);

    Arc::clone(flying.entry(key.to_owned()).or_default())
  }

  fn preview_path(&self, url: &str) -> PathBuf {
    self.directory.join("previews").join(format!("{}.json", key_of(url)))
  }

  // What is kept fresh of a page: a preview, a failure (none), or nothing kept (none outside).
  fn held_preview(&self, path: &Path) -> Option<Option<Preview>> {
    let held: PreviewEntry = read_json(path)?;
    let ttl = if held.preview.is_none() { FAILURE_TTL_MS } else { PREVIEW_TTL_MS };

    self.fresh(held.fetched_at, ttl).then_some(held.preview)
  }

  /// The preview kept fresh for a page, read nowhere.
  pub fn held(&self, url: &str) -> Option<Preview> {
    self.held_preview(&self.preview_path(url)).flatten()
  }

  /// A page's preview: the one kept fresh, else one read now; none where it could not be had.
  pub async fn preview(&self, url: &str) -> Option<Preview> {
    let path = self.preview_path(url);

    if let Some(held) = self.held_preview(&path) {
      return held;
    }

    let lane = self.lane(&format!("page:{url}"));
    let _turn = lane.lock().await;

    if let Some(held) = self.held_preview(&path) {
      return held;
    }

    let preview = self.reader.preview(url).await.ok();
    let entry = PreviewEntry {
      fetched_at: (self.now)(),
      preview: preview.clone(),
    };

    write_atomically(&path, &serde_json::to_vec(&entry).unwrap_or_default());

    preview
  }

  fn held_image(&self, meta: &Path, bytes: &Path) -> Option<Option<Picture>> {
    let held: ImageEntry = read_json(meta)?;
    let ttl = if held.media_type.is_none() { FAILURE_TTL_MS } else { IMAGE_TTL_MS };

    if !self.fresh(held.fetched_at, ttl) {
      return None;
    }

    match held.media_type {
      None => Some(None),
      Some(media_type) => std::fs::read(bytes).ok().map(|bytes| Some(Picture { bytes, media_type })),
    }
  }

  /// The raster picture behind an address, none where it could not be had.
  pub async fn image(&self, url: &str) -> Option<Picture> {
    let key = key_of(url);
    let meta = self.directory.join("images").join(format!("{key}.json"));
    let bytes = self.directory.join("images").join(format!("{key}.bin"));

    if let Some(held) = self.held_image(&meta, &bytes) {
      return held;
    }

    let lane = self.lane(&format!("image:{url}"));
    let _turn = lane.lock().await;

    if let Some(held) = self.held_image(&meta, &bytes) {
      return held;
    }

    let image = self.reader.picture(url).await.ok().and_then(|picture| {
      let media_type = raster_type(&picture.bytes)?;

      Some(Picture {
        media_type: media_type.to_owned(),
        bytes: picture.bytes,
      })
    });

    if let Some(image) = &image {
      write_atomically(&bytes, &image.bytes);
    }

    let entry = ImageEntry {
      fetched_at: (self.now)(),
      media_type: image.as_ref().map(|image| image.media_type.clone()),
    };

    write_atomically(&meta, &serde_json::to_vec(&entry).unwrap_or_default());

    image
  }
}

fn key_of(url: &str) -> String {
  Sha256::digest(url.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
  serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

// Owner-only, whole or not at all.
fn write_atomically(path: &Path, data: &[u8]) {
  use std::io::Write;
  use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

  let Some(directory) = path.parent() else { return };

  if std::fs::DirBuilder::new().recursive(true).mode(0o700).create(directory).is_err() {
    return;
  }

  let staged = path.with_extension(format!("{}.{}.tmp", std::process::id(), rand::random::<u32>()));
  let written = std::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .mode(0o600)
    .open(&staged)
    .and_then(|mut file| file.write_all(data));

  if written.is_err() || std::fs::rename(&staged, path).is_err() {
    let _ = std::fs::remove_file(&staged);
  }
}
