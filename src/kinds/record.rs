//! `record`: data the owner asked the agent to put in order and keep. No summary unless the body writes one,
//! and its contents are the body's headings to two levels.

use super::kind::{HeadProperty, Kind, Outline, OutlineFinding, OutlineLevel, heading_outline};
use crate::wording::search::Search;

pub struct RecordKind;

impl Kind for RecordKind {
  fn id(&self) -> &'static str {
    "record"
  }

  fn summary(&self) -> bool {
    false
  }

  fn head(&self) -> &'static [HeadProperty] {
    &[HeadProperty::Request]
  }

  fn outline(&self, findings: &[OutlineFinding], words: &Search) -> Outline {
    heading_outline(findings, Some(&[OutlineLevel::Chapter, OutlineLevel::Section]), words)
  }
}
