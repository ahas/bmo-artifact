//! The page reader this service reads the web through (`docs/api.md`, "What this service calls"): a page's
//! markup, a picture and a link's preview, each one call of bmo-explorer's published client. The routes hold
//! it as `PageReader`, so a test hands them a reader of its own and reaches no network.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use bmo_explorer::client::{ClientError, Explorer, RasterInput};

/// What a read comes to: its answer, or the sentence why it failed.
pub type Reading<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

/// A page's markup as it was served.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageMarkup {
  /// The address the read ended at, after every redirect.
  pub url: String,
  pub html: String,
}

/// A picture's bytes and its type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
  pub bytes: Vec<u8>,
  pub media_type: String,
}

/// What a page's head declares for a preview.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Preview {
  pub title: Option<String>,
  pub description: Option<String>,
  /// The preview picture's absolute address.
  pub image: Option<String>,
}

/// Where the web is read.
pub trait PageReader: Send + Sync {
  /// A page's markup.
  fn page<'a>(&'a self, url: &'a str) -> Reading<'a, PageMarkup>;
  /// The picture at an address.
  fn picture<'a>(&'a self, url: &'a str) -> Reading<'a, Picture>;
  /// What a page declares for its preview.
  fn preview<'a>(&'a self, url: &'a str) -> Reading<'a, Preview>;
}

/// The reader over bmo-explorer's client.
pub struct ExplorerReader {
  client: Explorer,
}

impl ExplorerReader {
  /// The client opened over the data root, where bmo-explorer keeps its own folder.
  pub fn open(data_root: &Path) -> Self {
    Self {
      client: Explorer::open(data_root),
    }
  }
}

fn said(failure: ClientError) -> String {
  failure.error
}

impl PageReader for ExplorerReader {
  fn page<'a>(&'a self, url: &'a str) -> Reading<'a, PageMarkup> {
    Box::pin(async move {
      let source = self.client.fetch_page(url).await.map_err(said)?;

      if !(200..300).contains(&source.status) {
        return Err(format!("{} answered {}", source.url, source.status));
      }

      Ok(PageMarkup {
        url: source.url,
        html: source.html,
      })
    })
  }

  fn picture<'a>(&'a self, url: &'a str) -> Reading<'a, Picture> {
    Box::pin(async move {
      let raster = self
        .client
        .fetch_raster(&RasterInput {
          url: url.to_owned(),
          width: None,
        })
        .await
        .map_err(said)?;
      let bytes = STANDARD
        .decode(raster.image)
        .map_err(|why| format!("the picture at {url} does not read: {why}"))?;

      Ok(Picture {
        bytes,
        media_type: raster.mime_type,
      })
    })
  }

  fn preview<'a>(&'a self, url: &'a str) -> Reading<'a, Preview> {
    Box::pin(async move {
      let preview = self.client.link_preview(url).await.map_err(said)?;

      Ok(Preview {
        title: preview.title,
        description: preview.description,
        image: preview.image,
      })
    })
  }
}
