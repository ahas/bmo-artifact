//! The shape every kind of document answers (`docs/artifact.md`, "Kinds").

use crate::wire::DocumentKind;

/// One kind of document.
pub trait Kind: Sync {
  /// The word a document of this kind is filed and answered under.
  fn name(&self) -> &'static str;
  /// What a document of this kind is, as a sentence.
  fn description(&self) -> &'static str;
  /// Whether a document of this kind is the HTML page it keeps, filed as HTML and opened as that page
  /// wherever it is opened; any other kind is filed as a markdown body.
  fn hosted(&self) -> bool {
    false
  }
  /// How many levels of its headings its table of contents lists, 0 for none.
  fn contents_depth(&self) -> u8;

  /// The kind as `GET /document-kinds` answers it.
  fn wire(&self) -> DocumentKind {
    DocumentKind {
      name: self.name().to_owned(),
      description: self.description().to_owned(),
      hosted: self.hosted(),
      contents_depth: self.contents_depth(),
    }
  }
}
