//! The web as the read API's previews reach it, stood in for: every page previews with a picture and no
//! icon, every picture is a PNG, and every address asked is written down.

use std::sync::Mutex;

use bmo_artifact::clients::tool_host::Raster;
use bmo_artifact::documents::preview::{Fetching, PagePreview, Previewing};

/// The bytes every picture answers with.
pub const PNG: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

#[derive(Default)]
pub struct FakeWeb {
  pub fetched: Mutex<Vec<String>>,
}

impl FakeWeb {
  /// What was fetched since this was last asked.
  pub fn taken(&self) -> Vec<String> {
    std::mem::take(&mut *self.fetched.lock().unwrap())
  }
}

impl Previewing for FakeWeb {
  fn page<'a>(&'a self, address: &'a str) -> Fetching<'a, PagePreview> {
    self.fetched.lock().unwrap().push(format!("preview {address}"));

    let host = url::Url::parse(address)
      .ok()
      .and_then(|url| url.host_str().map(str::to_owned))
      .unwrap_or_default();

    Box::pin(std::future::ready(Ok(PagePreview {
      title: Some(format!("title {host}")),
      image: Some("https://img.example.com/card.png".to_owned()),
      ..PagePreview::default()
    })))
  }

  fn picture<'a>(&'a self, address: &'a str) -> Fetching<'a, Raster> {
    self.fetched.lock().unwrap().push(format!("image {address}"));

    Box::pin(std::future::ready(Ok(Raster {
      kind: "image/png".to_owned(),
      bytes: PNG.to_vec(),
    })))
  }
}
