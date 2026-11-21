//! `page`: an HTML page kept as it stands. The file is the document and is opened as that page wherever the
//! document is opened, so it has no contents.

use super::kind::Kind;

pub struct PageKind;

impl Kind for PageKind {
  fn name(&self) -> &'static str {
    "page"
  }

  fn description(&self) -> &'static str {
    "An HTML page kept as it stands and opened as that page."
  }

  fn hosted(&self) -> bool {
    true
  }

  fn icon(&self) -> &'static str {
    "web"
  }

  fn contents_depth(&self) -> u8 {
    0
  }
}
