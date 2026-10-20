//! `learning`: study material, laid out as a book. Its contents are three levels: the top heading a part,
//! the one under it a chapter, the one under that a section.

use super::kind::Kind;

pub struct LearningKind;

impl Kind for LearningKind {
  fn name(&self) -> &'static str {
    "learning"
  }

  fn description(&self) -> &'static str {
    "Study material laid out as a book of parts, chapters and sections, as a markdown body."
  }

  fn contents_depth(&self) -> u8 {
    3
  }
}
