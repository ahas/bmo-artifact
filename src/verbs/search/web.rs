//! What the search runner asks of the web (`docs/api.md`, "Read a page and fetch a picture"): a picture's
//! bytes behind an address, and a page drawn and read whole with every figure it draws captured. The runner
//! reaches the web only through a page reader, which in service is bmo-explorer behind the tool host, where
//! each address is checked and each request paced; each read here is bounded by its own wait.

use std::sync::Arc;
use std::time::Duration;

use crate::clients::tool_host::{PageReader, Raster};
use crate::helpers::picture::raster_type;

/// How long one picture may take to come; the keeping as a whole has its own deadline.
pub const PICTURE_WAIT: Duration = Duration::from_secs(60);

/// How long a page import may take: opening the page, following it, reading it and capturing its figures.
pub const PAGE_WAIT: Duration = Duration::from_secs(150);

/// A figure the page drew itself, as it was captured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Figure {
  pub n: i64,
  pub caption: String,
  /// The captured figure's PNG; none for one that would not capture.
  pub png: Option<Vec<u8>>,
  pub svg: Option<String>,
}

/// A reference the page's own reference list names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
  pub text: String,
  pub url: String,
}

/// The page as it read itself: its address after redirects, title, keywords, its text as the result's body,
/// its references, and its figures, captured or not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
  pub url: String,
  pub title: String,
  pub keywords: Vec<String>,
  pub body: String,
  pub references: Vec<Reference>,
  pub figures: Vec<Figure>,
}

/// Where the runner reaches the web.
#[derive(Clone)]
pub struct WebLayer {
  reader: Arc<dyn PageReader>,
}

impl WebLayer {
  /// The web as `reader` reads it.
  pub fn new(reader: Arc<dyn PageReader>) -> Self {
    Self { reader }
  }

  /// The picture behind an address, its type read off its own bytes; bytes that are no raster picture are
  /// refused.
  pub async fn picture(&self, address: &str) -> Result<Raster, String> {
    let fetched = tokio::time::timeout(PICTURE_WAIT, self.reader.picture(address))
      .await
      .map_err(|_| format!("the picture did not come within {}s", PICTURE_WAIT.as_secs()))??;
    let kind = raster_type(&fetched.bytes).ok_or_else(|| "what came is not a picture".to_owned())?;

    Ok(Raster {
      kind: kind.to_owned(),
      bytes: fetched.bytes,
    })
  }

  /// A page drawn and read whole, each figure carrying its capture where it captured.
  pub async fn page(&self, address: &str) -> Result<Page, String> {
    let reading = tokio::time::timeout(PAGE_WAIT, self.reader.page(address))
      .await
      .map_err(|_| format!("the page was not read within {}s", PAGE_WAIT.as_secs()))??;
    let mut captured = reading.captured;
    let page = reading.page;

    Ok(Page {
      url: page.url,
      title: page.title,
      keywords: page.keywords,
      body: page.body,
      references: page
        .references
        .into_iter()
        .map(|reference| Reference {
          text: reference.text,
          url: reference.url,
        })
        .collect(),
      figures: page
        .figures
        .into_iter()
        .map(|figure| Figure {
          png: captured
            .iter()
            .position(|capture| capture.n == figure.n)
            .map(|at| captured.swap_remove(at).png),
          n: figure.n,
          caption: figure.caption,
          svg: figure.svg,
        })
        .collect(),
    })
  }
}
