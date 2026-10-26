//! The share links of documents, as rows of `web_search_shares` (`docs/archive.md`, "Share links"). This is
//! the one module that writes the table. Its times are epoch seconds; a link is answered in milliseconds.
//!
//! A link is a row rather than a signature: a link is read back long after it was made, and taken away before
//! its time, neither of which a signed capability can do. What the row guards is the token, which is the link
//! entire: a wide random value from which nothing is derived, naming the one row it was written for.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sea_orm::{ConnectionTrait, QueryResult};

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

/// The links whose time has passed at `now`, in epoch seconds, taken away; the tokens they held.
pub async fn sweep(db: &impl ConnectionTrait, now: i64) -> Result<Vec<String>, Refusal> {
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
/// is given, at most `limit`; and whether more stand past them. Swept first by the caller.
pub async fn list(
  db: &impl ConnectionTrait,
  document_id: Option<i64>,
  after: Option<i64>,
  limit: i64,
) -> Result<(Vec<Link>, bool), Refusal> {
  let rows = all(
    db,
    &format!(
      r#"SELECT {COLUMNS} FROM "web_search_shares"
          WHERE ($1::bigint IS NULL OR "search_id" = $1) AND "created_at" > $2
          ORDER BY "created_at", "share_token" LIMIT $3"#
    ),
    vec![document_id.into(), after.unwrap_or(i64::MIN).into(), (limit + 1).into()],
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
