//! `page`: an HTML page kept as it stands. The file is the document and is opened as that page wherever the
//! document is opened, so it has no summary and no contents; its head shows the description taken from the
//! page.

use super::kind::{HeadProperty, Kind, Outline, OutlineFinding};
use crate::wording::search::Search;

pub struct PageKind;

impl Kind for PageKind {
  fn id(&self) -> &'static str {
    "page"
  }

  fn summary(&self) -> bool {
    false
  }

  fn hosted(&self) -> bool {
    true
  }

  fn head(&self) -> &'static [HeadProperty] {
    &[HeadProperty::Description]
  }

  fn outline(&self, _findings: &[OutlineFinding], _words: &Search) -> Outline {
    Outline::default()
  }
}
