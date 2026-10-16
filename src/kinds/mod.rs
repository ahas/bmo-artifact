//! The kinds of document the archive holds (`docs/archive.md`, "Kinds"): a search, a page imported whole,
//! study material, a record the agent keeps and an HTML page the archive hosts.
//!
//! A kind is its own module and answers for itself through `Kind`: what it is called, whether it opens with
//! a summary, which properties its head shows and how its body becomes a table of contents. This module is
//! the catalogue and nothing else: the read API, the share page and the search runner ask a document's kind
//! through it, so none of them branches on a kind's name.

pub mod kind;

mod import;
mod learning;
mod page;
mod record;
mod search;

pub use kind::{HeadProperty, Kind, Outline, OutlineEntry, OutlineFinding, OutlineLevel};
pub use search::{finding_contents, finding_name, finding_title};

/// Every kind there is, in the order a filter reads best. A kind is added by writing its module and naming
/// it here.
pub static KINDS: [&dyn Kind; 5] = [
  &search::SearchKind,
  &import::ImportKind,
  &learning::LearningKind,
  &record::RecordKind,
  &page::PageKind,
];

/// The kind a row's word names. A word the catalogue does not know reads as a search, so the document is
/// still drawn.
pub fn kind(id: Option<&str>) -> &'static dyn Kind {
  known(id).unwrap_or(KINDS[0])
}

/// The kind a word names, none where the catalogue holds no such kind.
pub fn known(id: Option<&str>) -> Option<&'static dyn Kind> {
  KINDS.iter().copied().find(|kind| Some(kind.id()) == id)
}
