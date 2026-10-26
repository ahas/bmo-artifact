//! `search`: what a research found, in the shape the researcher writes it: the core answer, the three-line
//! summary, the findings in the sections the question shapes, and what could not be confirmed. Its contents
//! are its headings, to two levels: a section and the parts inside it.

use super::kind::Kind;

pub struct SearchKind;

impl Kind for SearchKind {
  fn name(&self) -> &'static str {
    "search"
  }

  fn description(&self) -> &'static str {
    "What a research found: the core answer, the summary, the findings and what stayed unconfirmed, as a markdown body."
  }

  fn contents_depth(&self) -> u8 {
    2
  }
}
