//! `import`: a page brought in whole. No summary of the agent's making — the page's own stands where it has
//! one — and its contents are the page's own headings, every level of them kept. The importer writes a
//! page's top headings as `SECTION:` lines and the ones under them as markdown, and both stand, sections
//! first.

use super::kind::{HeadProperty, Kind, Outline, OutlineFinding, heading_outline};
use crate::wording::search::Search;

pub struct ImportKind;

impl Kind for ImportKind {
  fn id(&self) -> &'static str {
    "import"
  }

  fn summary(&self) -> bool {
    false
  }

  fn head(&self) -> &'static [HeadProperty] {
    &[HeadProperty::Source]
  }

  fn outline(&self, findings: &[OutlineFinding], words: &Search) -> Outline {
    heading_outline(findings, None, words)
  }
}
