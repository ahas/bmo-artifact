//! `import`: a page brought in whole through an import. Its contents are the page's own headings, their
//! levels kept.

use super::kind::Kind;

pub struct ImportKind;

impl Kind for ImportKind {
  fn name(&self) -> &'static str {
    "import"
  }

  fn description(&self) -> &'static str {
    "A web page brought in whole, as the markdown its import wrote."
  }

  fn icon(&self) -> &'static str {
    "public"
  }

  fn contents_depth(&self) -> u8 {
    3
  }
}
