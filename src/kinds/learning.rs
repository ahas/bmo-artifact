//! `learning`: study material, laid out as a book. No summary. Its contents are three levels: the top
//! heading a part, the one under it a chapter, the one under that a section; a document with no parts starts
//! at its chapters, and anything deeper stays in the text.

use super::kind::{HeadProperty, Kind, Outline, OutlineFinding, OutlineLevel, heading_outline};
use crate::wording::search::Search;

pub struct LearningKind;

impl Kind for LearningKind {
  fn id(&self) -> &'static str {
    "learning"
  }

  fn summary(&self) -> bool {
    false
  }

  fn searched(&self) -> bool {
    true
  }

  fn head(&self) -> &'static [HeadProperty] {
    &[HeadProperty::Topic, HeadProperty::Keywords]
  }

  fn outline(&self, findings: &[OutlineFinding], words: &Search) -> Outline {
    heading_outline(
      findings,
      Some(&[OutlineLevel::Part, OutlineLevel::Chapter, OutlineLevel::Section]),
      words,
    )
  }
}
