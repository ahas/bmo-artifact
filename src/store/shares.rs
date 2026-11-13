//! The share links of documents, as rows of `web_search_shares`, and the notes they leave in
//! `web_search_share_notes` where they stop opening (`docs/artifact.md`, "Share links"). This is the one module
//! that writes either table. Its times are epoch seconds; a link is answered in milliseconds.
//!
//! A link is a row rather than a signature: a link is read back long after it was made, and taken away before
//! its time, neither of which a signed capability can do. What the row guards is the token, which is the link
//! entire: a wide random value from which nothing is derived, naming the one row it was written for.
//!
//! A note is written before the link goes, so a link that ended leaves what it opened behind it: a token that
//! names no live link answers what stood rather than nothing at all.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sea_orm::{ConnectionTrait, QueryResult, Value};

use super::member_shares::shared_with;
use super::{all, one, run};
use crate::plumbing::refusal::Refusal;
use crate::wire::Share;

/// One link's row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
  pub token: String,
  pub document_id: i64,
  /// Epoch seconds.
  pub created_at: i64,
  /// Epoch seconds; none for a link with no end.
  pub expires_at: Option<i64>,
}

impl Link {
  /// The link as the API answers it, its `url` the share address, a `/` and the token.
  pub fn wire(&self, address: &str) -> Share {
    Share {
      token: self.token.clone(),
      document_id: self.document_id,
      url: format!("{}/{}", address.trim_end_matches('/'), self.token),
      created_at: self.created_at * 1000,
      expires_at: self.expires_at.map(|at| at * 1000),
    }
  }
}

/// What a link opened, kept where it stops opening: the token, the document it opened, that document's title,
/// the second it ended and why it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
  pub token: String,
  pub document_id: i64,
  /// The title of the document it opened, empty where the document was gone before the note was written.
  pub title: String,
  /// Epoch seconds.
  pub ended_at: i64,
  /// Why it ended: `expired`, `ended` or `released`.
  pub reason: String,
}

/// The reason a link ended because its time has passed.
pub const EXPIRED: &str = "expired";
/// The reason a link ended because a caller ended it.
pub const ENDED: &str = "ended";
/// The reason a link ended because the document it opened was let go of.
pub const RELEASED: &str = "released";

// Six random bytes make eight URL-safe characters: short enough to read out, wide enough that the links
// kept never meet, and drawn again where one does.
const TOKEN_BYTES: usize = 6;

/// A new token: eight characters of the URL-safe alphabet, so the whole of it stands as one short path
/// segment.
pub fn share_token() -> String {
  URL_SAFE_NO_PAD.encode(rand::random::<[u8; TOKEN_BYTES]>())
}

/// Whether a token is shaped like one issued here. Anything else opens nothing.
pub fn is_share_token(token: &str) -> bool {
  (8..=128).contains(&token.len())
    && token.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

const COLUMNS: &str = r#""share_token", "search_id", "created_at", "expires_at""#;

fn link_of(row: &QueryResult) -> Result<Link, Refusal> {
  Ok(Link {
    token: row.try_get("", "share_token")?,
    document_id: row.try_get("", "search_id")?,
    created_at: row.try_get("", "created_at")?,
    expires_at: row.try_get("", "expires_at")?,
  })
}

// The note one link leaves, written from the link's own row while that row still stands: what it opened, its
// document's title, the second it ended and why. The row keeps the token and the title it opened, never the
// body: a note says what stood, not what it said.
const NOTED: &str = r#"INSERT INTO "web_search_share_notes" ("share_token", "search_id", "title", "ended_at", "reason")
   SELECT s."share_token", s."search_id", COALESCE(d."title", ''), "#;

/// The notes the links whose time has passed leave, at `now` in epoch seconds, written before those links go.
async fn note_expired(db: &impl ConnectionTrait, now: i64) -> Result<(), Refusal> {
  run(
    db,
    &format!(
      r#"{NOTED} s."expires_at", $2
           FROM "web_search_shares" s
           LEFT JOIN "searches" d ON d."id" = s."search_id"
          WHERE s."expires_at" IS NOT NULL AND s."expires_at" <= $1
         ON CONFLICT ("share_token") DO NOTHING"#
    ),
    vec![now.into(), EXPIRED.into()],
  )
  .await
  .map(drop)
}

/// The note the live link of one document leaves at `now`, in epoch seconds, written before the link goes; none
/// where that document has no live link.
pub async fn note_of_document(
  db: &impl ConnectionTrait,
  document_id: i64,
  now: i64,
  reason: &str,
) -> Result<(), Refusal> {
  run(
    db,
    &format!(
      r#"{NOTED} $2, $3
           FROM "web_search_shares" s
           LEFT JOIN "searches" d ON d."id" = s."search_id"
          WHERE s."search_id" = $1
         ON CONFLICT ("share_token") DO NOTHING"#
    ),
    vec![document_id.into(), now.into(), reason.into()],
  )
  .await
  .map(drop)
}

/// The note one token left, none where it names none.
pub async fn note_by_token(db: &impl ConnectionTrait, token: &str) -> Result<Option<Note>, Refusal> {
  if !is_share_token(token) {
    return Ok(None);
  }

  one(
    db,
    r#"SELECT "share_token", "search_id", "title", "ended_at", "reason"
         FROM "web_search_share_notes" WHERE "share_token" = $1"#,
    vec![token.into()],
  )
  .await?
  .as_ref()
  .map(|row| {
    Ok(Note {
      token: row.try_get("", "share_token")?,
      document_id: row.try_get("", "search_id")?,
      title: row.try_get("", "title")?,
      ended_at: row.try_get("", "ended_at")?,
      reason: row.try_get("", "reason")?,
    })
  })
  .transpose()
}

/// The links whose time has passed at `now`, in epoch seconds, taken away; the tokens they held. Each leaves a
/// note first, so a link somebody was handed answers what it opened long after it stopped opening.
pub async fn sweep(db: &impl ConnectionTrait, now: i64) -> Result<Vec<String>, Refusal> {
  note_expired(db, now).await?;

  all(
    db,
    r#"DELETE FROM "web_search_shares" WHERE "expires_at" IS NOT NULL AND "expires_at" <= $1
       RETURNING "share_token""#,
    vec![now.into()],
  )
  .await?
  .iter()
  .map(|row| row.try_get::<String>("", "share_token").map_err(Refusal::from))
  .collect()
}

/// The live links, oldest first, of one document where it is named, made after `after` seconds where that
/// is given, at most `limit`; and whether more stand past them. Read for a `member`, they are the links of the
/// documents shared with that account alone. Swept first by the caller.
pub async fn list(
  db: &impl ConnectionTrait,
  document_id: Option<i64>,
  after: Option<i64>,
  limit: i64,
  member: Option<&str>,
) -> Result<(Vec<Link>, bool), Refusal> {
  let shared = match member {
    Some(_) => format!(
      r#" AND EXISTS (SELECT 1 FROM "searches" d WHERE d."id" = "web_search_shares"."search_id" AND {})"#,
      shared_with("$4")
    ),
    None => String::new(),
  };
  let mut values: Vec<Value> =
    vec![document_id.into(), after.unwrap_or(i64::MIN).into(), (limit + 1).into()];

  values.extend(member.map(Value::from));

  let rows = all(
    db,
    &format!(
      r#"SELECT {COLUMNS} FROM "web_search_shares"
          WHERE ($1::bigint IS NULL OR "search_id" = $1) AND "created_at" > $2{shared}
          ORDER BY "created_at", "share_token" LIMIT $3"#
    ),
    values,
  )
  .await?;
  let mut links = rows.iter().map(link_of).collect::<Result<Vec<_>, _>>()?;
  let more = links.len() as i64 > limit;

  links.truncate(usize::try_from(limit).unwrap_or(usize::MAX));

  Ok((links, more))
}

/// The link of one document as its row stands.
pub async fn of_document(db: &impl ConnectionTrait, document_id: i64) -> Result<Option<Link>, Refusal> {
  one(
    db,
    &format!(r#"SELECT {COLUMNS} FROM "web_search_shares" WHERE "search_id" = $1"#),
    vec![document_id.into()],
  )
  .await?
  .as_ref()
  .map(link_of)
  .transpose()
}

/// The row one token names, none where it names none or is not shaped as a token.
pub async fn by_token(db: &impl ConnectionTrait, token: &str) -> Result<Option<Link>, Refusal> {
  if !is_share_token(token) {
    return Ok(None);
  }

  one(
    db,
    &format!(r#"SELECT {COLUMNS} FROM "web_search_shares" WHERE "share_token" = $1"#),
    vec![token.into()],
  )
  .await?
  .as_ref()
  .map(link_of)
  .transpose()
}

/// A link written for a document that has none, made at `now` and ending at `expires_at`, both epoch
/// seconds: the link, none where one already stands. The looking and the writing are one statement, so two
/// askings at once cannot both find the document free.
pub async fn create(
  db: &impl ConnectionTrait,
  document_id: i64,
  created_by: &str,
  now: i64,
  expires_at: Option<i64>,
) -> Result<Option<Link>, Refusal> {
  // A token that meets one already kept is drawn again; a document that already has its link gets none.
  for _ in 0..8 {
    let written = one(
      db,
      &format!(
        r#"INSERT INTO "web_search_shares" ("share_token", "search_id", "created_by", "created_at", "expires_at")
           VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING RETURNING {COLUMNS}"#
      ),
      vec![share_token().into(), document_id.into(), created_by.into(), now.into(), expires_at.into()],
    )
    .await?;

    if let Some(row) = written.as_ref() {
      return link_of(row).map(Some);
    }

    if one(
      db,
      r#"SELECT 1 FROM "web_search_shares" WHERE "search_id" = $1"#,
      vec![document_id.into()],
    )
    .await?
    .is_some()
    {
      return Ok(None);
    }
  }

  Err(Refusal::failed("no share token could be drawn that was not already kept".to_owned()))
}

/// The link one token names taken away: whether there was one to take.
pub async fn remove(db: &impl ConnectionTrait, token: &str) -> Result<bool, Refusal> {
  Ok(run(db, r#"DELETE FROM "web_search_shares" WHERE "share_token" = $1"#, vec![token.into()]).await? > 0)
}
