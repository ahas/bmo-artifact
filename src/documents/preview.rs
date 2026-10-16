//! What a page shows of itself before anyone opens it — its title, its picture, its icon — and the pictures
//! a text points at, fetched for a reader who is not to reach the outside server directly
//! (`docs/archive.md`, "Where a person reads it"). Every fetch goes through bmo-explorer by the tool host,
//! which checks the address and paces each host. What was fetched is kept under the file root, so a page
//! seen once is not asked again for a while, and neither is one that could not be reached.

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex as StdMutex, PoisonError};
use std::time::Duration;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::clients::tool_host::{PageReader, Raster, ToolHost};
use crate::helpers::picture::raster_type;
use crate::now_ms;

/// How much of a page is read for its head.
pub const PREVIEW_BYTE_CAP: usize = 512 * 1024;
/// How long a preview or a picture is waited for; one that takes longer is given up and not kept.
pub const PREVIEW_WAIT: Duration = Duration::from_secs(10);
/// How long a preview, a picture and a failure are kept.
pub const PREVIEW_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;
pub const IMAGE_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000;
pub const FAILURE_TTL_MS: i64 = 24 * 60 * 60 * 1000;

/// What a page says of itself for sharing, its addresses resolved against the page.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagePreview {
  pub title: Option<String>,
  #[serde(default)]
  pub description: Option<String>,
  #[serde(default)]
  pub site_name: Option<String>,
  pub image: Option<String>,
  pub icon: Option<String>,
}

/// A page's head as bmo-explorer fetched it: the address after every redirect, its content type, and its
/// markup where it is markup.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct FetchedPage {
  pub url: String,
  #[serde(default)]
  pub r#type: String,
  #[serde(default)]
  pub html: Option<String>,
}

/// Why a fetch gave nothing: it ran out of time, which says nothing of the page and is not kept, or it was
/// refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Missed {
  Deadline,
  Refused(String),
}

/// What fetching comes to.
pub type Fetching<'a, T> = Pin<Box<dyn Future<Output = Result<T, Missed>> + Send + 'a>>;

/// The two fetches a preview asks of the web.
pub trait Previewing: Send + Sync {
  /// A page's head.
  fn page<'a>(&'a self, address: &'a str) -> Fetching<'a, PagePreview>;
  /// The raster picture behind an address.
  fn picture<'a>(&'a self, address: &'a str) -> Fetching<'a, Raster>;
}

fn absolute(href: Option<&str>, base: &str) -> Option<String> {
  let href = href?.trim();

  if href.is_empty() {
    return None;
  }

  let url = url::Url::parse(base).ok()?.join(href).ok()?;

  matches!(url.scheme(), "http" | "https").then(|| url.to_string())
}

fn tidy(text: Option<&str>) -> Option<String> {
  let value = text?.split_whitespace().collect::<Vec<_>>().join(" ");

  (!value.is_empty()).then(|| value.chars().take(300).collect())
}

/// The preview a page's markup gives, read from its tags without running anything. A page names no icon
/// often enough that the site's conventional one stands in.
pub fn read_preview(html: &str, base: &str) -> PagePreview {
  let cut: String = html.chars().take(PREVIEW_BYTE_CAP).collect();
  let page = Html::parse_document(&cut);
  let selector = |css: &str| Selector::parse(css).expect("a selector");
  let mut meta: HashMap<String, String> = HashMap::new();
  let mut icons: Vec<(String, String)> = Vec::new();

  for tag in page.select(&selector("meta")) {
    let attributes = tag.value();
    let key = attributes
      .attr("property")
      .or_else(|| attributes.attr("name"))
      .unwrap_or("")
      .to_lowercase();

    if let Some(content) = attributes.attr("content")
      && !key.is_empty()
    {
      meta.entry(key).or_insert_with(|| content.to_owned());
    }
  }

  for tag in page.select(&selector("link")) {
    let attributes = tag.value();
    let rel: Vec<String> = attributes
      .attr("rel")
      .unwrap_or("")
      .to_lowercase()
      .split_whitespace()
      .map(str::to_owned)
      .collect();

    if let Some(href) = attributes.attr("href")
      && rel.iter().any(|one| one == "icon")
    {
      icons.push((rel.join(" "), href.to_owned()));
    }
  }

  let title = page.select(&selector("title")).next().map(|tag| tag.text().collect::<String>());
  let named = |key: &str| meta.get(key).map(String::as_str);
  let icon = icons
    .iter()
    .find(|(rel, _)| !rel.contains("apple-touch-icon"))
    .or(icons.first())
    .map(|(_, href)| href.as_str());

  PagePreview {
    title: tidy(named("og:title"))
      .or_else(|| tidy(named("twitter:title")))
      .or_else(|| tidy(title.as_deref())),
    description: tidy(named("og:description"))
      .or_else(|| tidy(named("twitter:description")))
      .or_else(|| tidy(named("description"))),
    site_name: tidy(named("og:site_name")).or_else(|| tidy(named("application-name"))),
    image: absolute(named("og:image:secure_url").or(named("og:image")).or(named("twitter:image")), base),
    icon: absolute(Some(icon.unwrap_or("/favicon.ico")), base),
  }
}

/// A page's preview out of what bmo-explorer fetched of it: a page that is not markup has nothing to
/// preview but its site's icon, which is an answer rather than a failure.
pub fn preview_of(fetched: &FetchedPage) -> PagePreview {
  let markup = fetched.r#type.split(';').next().map(str::trim).is_some_and(|kind| {
    kind.eq_ignore_ascii_case("text/html") || kind.eq_ignore_ascii_case("application/xhtml+xml")
  });

  match (&fetched.html, markup) {
    (Some(html), true) => read_preview(html, &fetched.url),
    _ => PagePreview {
      icon: absolute(Some("/favicon.ico"), &fetched.url),
      ..PagePreview::default()
    },
  }
}

impl Previewing for ToolHost {
  fn page<'a>(&'a self, address: &'a str) -> Fetching<'a, PagePreview> {
    Box::pin(async move {
      match tokio::time::timeout(PREVIEW_WAIT, self.fetch_page(address)).await {
        Err(_) => Err(Missed::Deadline),
        Ok(Err(why)) => Err(Missed::Refused(why)),
        Ok(Ok(fetched)) => Ok(preview_of(&fetched)),
      }
    })
  }

  fn picture<'a>(&'a self, address: &'a str) -> Fetching<'a, Raster> {
    Box::pin(async move {
      let fetched = match tokio::time::timeout(PREVIEW_WAIT, PageReader::picture(self, address)).await {
        Err(_) => return Err(Missed::Deadline),
        Ok(Err(why)) => return Err(Missed::Refused(why)),
        Ok(Ok(fetched)) => fetched,
      };
      let kind = raster_type(&fetched.bytes)
        .ok_or_else(|| Missed::Refused(format!("not a raster picture: {address}")))?;

      Ok(Raster {
        kind: kind.to_owned(),
        bytes: fetched.bytes,
      })
    })
  }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewEntry {
  fetched_at: i64,
  preview: Option<PagePreview>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageEntry {
  fetched_at: i64,
  r#type: Option<String>,
}

/// Previews and pictures kept by the address they came from. A success is kept for days and a failure for
/// one; a failure that only ran out of time is not kept at all. Two readers asking for the same address at
/// once share one fetch.
pub struct PreviewCache {
  directory: PathBuf,
  web: Arc<dyn Previewing>,
  now: Box<dyn Fn() -> i64 + Send + Sync>,
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
  /// Kept under `directory`, fetched through `web`.
  pub fn new(directory: PathBuf, web: Arc<dyn Previewing>) -> Self {
    Self::with_clock(directory, web, Box::new(now_ms))
  }

  /// The same, on a clock of the caller's own.
  pub fn with_clock(
    directory: PathBuf,
    web: Arc<dyn Previewing>,
    now: Box<dyn Fn() -> i64 + Send + Sync>,
  ) -> Self {
    Self {
      directory,
      web,
      now,
      flying: StdMutex::default(),
    }
  }

  fn fresh(&self, fetched_at: i64, ttl: i64) -> bool {
    let age = (self.now)() - fetched_at;

    (0..ttl).contains(&age)
  }

  // The lock one address's fetch is made under.
  fn lane(&self, key: &str) -> Arc<Mutex<()>> {
    let mut flying = self.flying.lock().unwrap_or_else(PoisonError::into_inner);

    Arc::clone(flying.entry(key.to_owned()).or_default())
  }

  fn held_preview(&self, path: &Path) -> Option<Option<PagePreview>> {
    let held: PreviewEntry = read_json(path)?;
    let ttl = if held.preview.is_none() { FAILURE_TTL_MS } else { PREVIEW_TTL_MS };

    self.fresh(held.fetched_at, ttl).then_some(held.preview)
  }

  /// A page's preview, none where it could not be had.
  pub async fn preview(&self, url: &str) -> Option<PagePreview> {
    let path = self.directory.join("previews").join(format!("{}.json", key_of(url)));

    if let Some(held) = self.held_preview(&path) {
      return held;
    }

    let lane = self.lane(&format!("preview:{url}"));
    let _turn = lane.lock().await;

    if let Some(held) = self.held_preview(&path) {
      return held;
    }

    let preview = match self.web.page(url).await {
      Ok(preview) => Some(preview),
      Err(Missed::Deadline) => return None,
      Err(Missed::Refused(_)) => None,
    };
    let entry = PreviewEntry {
      fetched_at: (self.now)(),
      preview: preview.clone(),
    };

    write_atomically(&path, &serde_json::to_vec(&entry).unwrap_or_default());

    preview
  }

  fn held_image(&self, meta: &Path, bytes: &Path) -> Option<Option<Raster>> {
    let held: ImageEntry = read_json(meta)?;
    let ttl = if held.r#type.is_none() { FAILURE_TTL_MS } else { IMAGE_TTL_MS };

    if !self.fresh(held.fetched_at, ttl) {
      return None;
    }

    match held.r#type {
      None => Some(None),
      Some(kind) => std::fs::read(bytes).ok().map(|bytes| Some(Raster { kind, bytes })),
    }
  }

  /// The picture behind an address, none where it could not be had.
  pub async fn image(&self, url: &str) -> Option<Raster> {
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

    let image = match self.web.picture(url).await {
      Ok(image) => Some(image),
      Err(Missed::Deadline) => return None,
      Err(Missed::Refused(_)) => None,
    };

    if let Some(image) = &image {
      write_atomically(&bytes, &image.bytes);
    }

    let entry = ImageEntry {
      fetched_at: (self.now)(),
      r#type: image.as_ref().map(|image| image.kind.clone()),
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
