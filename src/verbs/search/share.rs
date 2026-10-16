//! The address a long search answer is handed back with (`docs/archive.md`, "Where a person reads it"): the owner's
//! private chat is given the console's own address behind the login, and every other room the search's share
//! link, the console's own kind made through the same rows, so the owner sees it on the search and can take
//! it away early.

use crate::verbs::archive_docs::{Documents, ShareLife};

/// Past this many characters, counted as the console counts them, a result is told in a short summary and
/// left to its document: a message longer than a screen is read by nobody.
pub const LONG_RESULT_CHARS: usize = 2500;

/// Whether a result is too long to read in a room.
pub fn is_long(result: &str) -> bool {
  result.encode_utf16().count() > LONG_RESULT_CHARS
}

/// The address a room is handed for one search's document: the console's in the owner's private chat and the
/// share link everywhere else; none where the search left no document to hand out.
pub async fn room_link(documents: &Documents, search_id: i64, owner: bool) -> Option<String> {
  if !documents.has_document(search_id).await {
    return None;
  }

  match documents.hand_over(search_id, owner, ShareLife::default()).await {
    Ok(handed) => handed.give,
    Err(why) => {
      eprintln!("search {search_id}'s link could not be made: {}", why.detail);
      None
    }
  }
}
