//! `record`: data put in order and kept. Its contents are its headings, to two levels.

use super::kind::Kind;

pub struct RecordKind;

impl Kind for RecordKind {
  fn name(&self) -> &'static str {
    "record"
  }

  fn description(&self) -> &'static str {
    "Data put in order and kept, as a markdown body."
  }

  fn contents_depth(&self) -> u8 {
    2
  }
}
