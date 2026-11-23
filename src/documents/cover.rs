//! A document's cover picture (`docs/api.md`, "Documents"): the first picture its body draws by a kept
//! file's name (`docs/artifact.md`, "Attachments"), else the first picture it keeps, drawn as a link's
//! preview card shows it.

use percent_encoding::percent_decode_str;
use pulldown_cmark::{Event, Parser, Tag};
use sea_orm::ConnectionTrait;
use url::Url;

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

/// The names a target a body draws may name a kept file by: the target whole, and where it is a path with no
/// scheme, its last part. A web address names no kept file but by its whole.
pub fn kept_names(target: &str) -> Vec<&str> {
  let last = target.rsplit('/').next().unwrap_or(target);

  if last == target || last.is_empty() || Url::parse(target).is_ok() {
    vec![target]
  } else {
    vec![target, last]
  }
}

/// The pictures of `names` a markdown body draws by no picture of that name, in the order given: a document's
/// pictures stand in its body (`docs/artifact.md`, "Attachments"), so these are the ones left out of it.
pub fn left_out<'a>(body: &str, names: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
  let targets = drawn_names(body);

  names
    .into_iter()
    .filter(|name| !targets.iter().any(|target| kept_names(target).contains(name)))
    .collect()
}

/// The sentence refusing pictures a body leaves out, which names them and the form that draws one.
pub fn left_out_refusal(names: &[&str]) -> String {
  format!(
    "the body draws no picture named {}; a document's pictures stand in its body, each drawn where its words \
     speak of it as ![caption](name)",
    names.join(", ")
  )
}

/// The path of the kept file a document's cover is drawn from: the first picture its body draws, else the first it
/// keeps; none where it keeps no picture.
pub async fn cover_file<C: ConnectionTrait>(
  db: &C,
  document_id: i64,
  body: Option<&str>,
) -> Result<Option<String>, Refusal> {
  for target in body.map(drawn_names).unwrap_or_default() {
    for name in kept_names(&target) {
      if let Some(kept) = attachments::named(db, document_id, name).await?
        && kept.attachment.kind == "image"
      {
        return Ok(Some(kept.path));
      }
    }
  }

  Ok(attachments::first_of_kind(db, document_id, "image").await?.map(|kept| kept.path))
}

/// The cover drawn as a preview card shows it: upright, inside its edge, laid on white, as JPEG. None where
/// the file is no picture.
pub fn cover_card(bytes: &[u8]) -> Option<Vec<u8>> {
  card_picture(bytes)
}
