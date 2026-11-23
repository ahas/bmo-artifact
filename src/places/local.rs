//! The local place (`docs/artifact.md`, "The local place"): a document's files under the file root on this
//! machine, each written whole through a staged file and served only from inside the root.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Bytes;
use futures_util::StreamExt as _;
use serde_json::{Map, Value};
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _};

use super::{Grounds, Kept, Pending, Place, PlaceKind, Reading, range_of, unknown_field};
use crate::Refusal;
use crate::wire::StorageField;

/// The local place's name, which its paths begin with.
pub const NAME: &str = "local";

/// The folder every document's own folder stands in, under the file root.
pub const DOCUMENTS: &str = "searches";

/// How much of a file one block of a read carries.
const BLOCK: usize = 64 * 1024;

/// The kind of the local place.
pub struct Local;

impl PlaceKind for Local {
  fn name(&self) -> &'static str {
    NAME
  }

  fn description(&self) -> &'static str {
    "The files are kept on the machine the service runs on, under its file root."
  }

  fn fields(&self) -> Vec<StorageField> {
    Vec::new()
  }

  fn judge(&self, settings: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    unknown_field(settings, &self.fields())?;

    Ok(Map::new())
  }

  fn open(
    &self,
    _: &Map<String, Value>,
    grounds: &Grounds,
    _: Option<&str>,
  ) -> Result<Arc<dyn Place>, Refusal> {
    Ok(Arc::new(LocalPlace::new(&grounds.settings.files.root)))
  }
}

/// The file root on this machine.
pub struct LocalPlace {
  root: PathBuf,
}

/// The folder a document's files stand in under the file root, as a location of the local place.
pub fn document_folder(document_id: i64) -> String {
  format!("{DOCUMENTS}/{document_id}")
}

/// Whether a file stands inside the file root, every link resolved: a file is served from nowhere else.
pub fn inside_root(root: &Path, path: &Path) -> bool {
  match (std::fs::canonicalize(path), std::fs::canonicalize(root)) {
    (Ok(real), Ok(root)) => real.starts_with(root),
    _ => false,
  }
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

fn unread(why: std::io::Error) -> Refusal {
  if why.kind() == std::io::ErrorKind::NotFound {
    Refusal::absent("the file is not on the disk")
  } else {
    Refusal::failed(format!("the file could not be read: {why}"))
  }
}

impl LocalPlace {
  pub fn new(root: &Path) -> Self {
    Self {
      root: root.to_path_buf(),
    }
  }

  // A location as a path on the disk: one under the root, or a whole path kept as it stands.
  fn path(&self, at: &str) -> PathBuf {
    self.root.join(at)
  }

  // A location that stands inside the root, refused as absent otherwise.
  fn served(&self, at: &str) -> Result<PathBuf, Refusal> {
    let path = self.path(at);

    if inside_root(&self.root, &path) {
      Ok(path)
    } else {
      Err(Refusal::absent("the file is not on the disk"))
    }
  }
}

impl Place for LocalPlace {
  fn kind(&self) -> &'static str {
    NAME
  }

  fn folder<'a>(&'a self, document: i64, _: Option<&'a str>) -> Pending<'a, String> {
    Box::pin(async move { Ok(document_folder(document)) })
  }

  fn write<'a>(&'a self, at: &'a str, bytes: Vec<u8>) -> Pending<'a, ()> {
    Box::pin(async move {
      let path = self.path(at);
      let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| write_private(&path, &bytes));

      written.map_err(|why| Refusal::failed(why.to_string()))
    })
  }

  fn read<'a>(&'a self, at: &'a str, range: Option<&'a str>) -> Pending<'a, Reading> {
    Box::pin(async move {
      let path = self.served(at)?;
      let mut file = tokio::fs::File::open(&path).await.map_err(unread)?;
      let total = file.metadata().await.map_err(unread)?.len();
      let span = range_of(range, total);
      let (from, to) = span.unwrap_or((0, total.saturating_sub(1)));

      if total > 0 {
        file.seek(std::io::SeekFrom::Start(from)).await.map_err(unread)?;
      }

      let left = if total == 0 { 0 } else { to - from + 1 };
      let body = futures_util::stream::unfold((file, left), |(mut file, left)| async move {
        if left == 0 {
          return None;
        }

        let mut block = vec![0; usize::try_from(left).unwrap_or(BLOCK).min(BLOCK)];

        match file.read(&mut block).await {
          Ok(0) => None,
          Ok(read) => {
            block.truncate(read);

            Some((Ok(Bytes::from(block)), (file, left - read as u64)))
          }
          Err(why) => Some((Err(why), (file, 0))),
        }
      })
      .boxed();

      Ok(Reading { total, span, body })
    })
  }

  fn exists<'a>(&'a self, at: &'a str) -> Pending<'a, Option<Kept>> {
    Box::pin(async move {
      let Ok(path) = self.served(at) else { return Ok(None) };
      let Ok(written) = tokio::fs::metadata(&path).await else { return Ok(None) };
      let moment = written
        .modified()
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos());

      Ok(written.is_file().then(|| Kept {
        bytes: written.len(),
        version: moment.to_string(),
      }))
    })
  }

  fn delete<'a>(&'a self, at: &'a str) -> Pending<'a, ()> {
    Box::pin(async move {
      let path = self.path(at);

      if !inside_root(&self.root, &path) {
        return Ok(());
      }

      let gone = if path.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };

      match gone {
        Err(why) if why.kind() != std::io::ErrorKind::NotFound => Err(Refusal::failed(why.to_string())),
        _ => Ok(()),
      }
    })
  }
}
