//! The kinds a document is filed as (`docs/archive.md`, "Kinds"): a record, study material, a page
//! imported whole and an HTML page the service hosts.
//!
//! A kind is its own module and answers for itself through `Kind`. This module is the catalogue and nothing
//! else: the routes ask a document's kind through it, so none of them branches on a kind's name.

pub mod kind;

mod import;
mod learning;
mod page;
mod record;

pub use kind::Kind;
pub use record::RecordKind;

/// Every kind there is, in the order `GET /document-kinds` lists them. A kind is added by writing its module
/// and naming it here.
pub static KINDS: [&dyn Kind; 4] =
  [&record::RecordKind, &learning::LearningKind, &import::ImportKind, &page::PageKind];

/// The kind a row's word names. A word the catalogue does not know reads as a record, so the document is
/// still answered.
pub fn kind(name: Option<&str>) -> &'static dyn Kind {
  known(name).unwrap_or(&RecordKind)
}

/// The kind a word names, none where the catalogue holds no such kind.
pub fn known(name: Option<&str>) -> Option<&'static dyn Kind> {
  KINDS.iter().copied().find(|kind| Some(kind.name()) == name)
}
