//! A document's cover picture (`docs/api.md`, "Documents"): the first picture its body draws by a kept
//! file's name, else the first picture it keeps, drawn as a link's preview card shows it.

use std::path::PathBuf;

use percent_encoding::percent_decode_str;
use pulldown_cmark::{Event, Parser, Tag};
use sea_orm::ConnectionTrait;

use super::ground::card_picture;
use crate::Refusal;
use crate::store::attachments;

/// The names a markdown body draws pictures by, in its order, their percent-encoding undone.
pub fn drawn_names(body: &str) -> Vec<String> {
  Parser::new(body)
    .filter_map(|event| match event {
      Event::Start(Tag::Image { dest_url, .. }) => {
        Some(percent_decode_str(&dest_url).decode_utf8_lossy().into_owned())
      }
      _ => None,
    })
    .collect()
}

/// The kept file a document's cover is drawn from: the first picture its body draws, else the first it
/// keeps; none where it keeps no picture.
pub async fn cover_file<C: ConnectionTrait>(
  db: &C,
  document_id: i64,
  body: Option<&str>,
) -> Result<Option<PathBuf>, Refusal> {
  for name in body.map(drawn_names).unwrap_or_default() {
    if let Some(kept) = attachments::named(db, document_id, &name).await?
      && kept.attachment.kind == "image"
    {
      return Ok(Some(PathBuf::from(kept.path)));
    }
  }

  Ok(
    attachments::first_of_kind(db, document_id, "image")
      .await?
      .map(|kept| PathBuf::from(kept.path)),
  )
}

/// The cover drawn as a preview card shows it: upright, inside its edge, laid on white, as JPEG. None where
/// the file is no picture.
pub fn cover_card(bytes: &[u8]) -> Option<Vec<u8>> {
  card_picture(bytes)
}
