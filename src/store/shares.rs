//! The share links of documents, as rows of `web_search_shares` (`docs/archive.md`, "The share page"). This
//! is the one module that writes the table: the share API behind the console's sign-in and the CLI verbs
//! that hand a document to a room both make, read and take away links through it. Every time here is epoch
//! seconds.
//!
//! A link is a row rather than a signature: the owner reads a link back long after it was made, and takes
//! it away before its time — neither of which a signed capability can do. What the row guards is the token,
//! which is the link entire: a wide random value from which nothing is derived, naming the one row it was
//! written for.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sea_orm::{ConnectionTrait, QueryResult};
use serde::Serialize;

use crate::{Refusal, one, run};

/// One live link: its token, the document it opens, who made it and when, and when it stops opening,
/// none where it never does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Share {
  pub token: String,
  pub search_id: i64,
  pub created_by: String,
  pub created_at: i64,
  pub expires_at: Option<i64>,
}

/// The spans a link may be made to live, by the key the console offers each under, and how long each lasts:
/// none for the one that never ends, which has no length rather than a long one.
pub const SHARE_SPANS: [(&str, Option<i64>); 6] = [
  ("1h", Some(60 * 60)),
  ("6h", Some(6 * 60 * 60)),
  ("24h", Some(24 * 60 * 60)),
  ("7d", Some(7 * 24 * 60 * 60)),
  ("30d", Some(30 * 24 * 60 * 60)),
  ("forever", None),
];

/// How long the span a key names lasts; none where the console offers no such span.
pub fn span_seconds(key: &str) -> Option<Option<i64>> {
  SHARE_SPANS.iter().find(|(one, _)| *one == key).map(|(_, seconds)| *seconds)
}

const TOKEN_BYTES: usize = 32;

/// A new link: wide enough that guessing one is not a road, and URL-safe so the whole of it stands as one
/// path segment.
pub fn share_token() -> String {
  URL_SAFE_NO_PAD.encode(rand::random::<[u8; TOKEN_BYTES]>())
}

/// Whether a token is shaped like one issued here. Anything else opens nothing.
pub fn is_share_token(token: &str) -> bool {
  (22..=128).contains(&token.len())
    && token.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

const COLUMNS: &str = r#""share_token", "search_id", "created_by", "created_at", "expires_at""#;

fn share_of(row: &QueryResult) -> Result<Share, Refusal> {
  Ok(Share {
    token: row.try_get("", "share_token")?,
    search_id: row.try_get("", "search_id")?,
    created_by: row.try_get("", "created_by")?,
    created_at: row.try_get("", "created_at")?,
    expires_at: row.try_get("", "expires_at")?,
  })
}

/// Links whose time has passed, gone; how many. Every read and every making sweeps, so no dead secret is
/// left lying in the archive. Nothing relies on this having run: a link is refused by its own expiry being
/// read, not by its row being absent. A link with no expiry has no time to have passed.
pub async fn sweep(db: &impl ConnectionTrait, now: i64) -> Result<u64, Refusal> {
  run(
    db,
    r#"DELETE FROM "web_search_shares" WHERE "expires_at" IS NOT NULL AND "expires_at" <= $1"#,
    vec![now.into()],
  )
  .await
}

/// The link of one document as its row stands, not swept first.
pub async fn standing(db: &impl ConnectionTrait, search_id: i64) -> Result<Option<Share>, Refusal> {
  one(
    db,
    &format!(r#"SELECT {COLUMNS} FROM "web_search_shares" WHERE "search_id" = $1"#),
    vec![search_id.into()],
  )
  .await?
  .map(|row| share_of(&row))
  .transpose()
}

/// The live link of one document, none where it has none.
pub async fn read(db: &impl ConnectionTrait, search_id: i64, now: i64) -> Result<Option<Share>, Refusal> {
  sweep(db, now).await?;
  standing(db, search_id).await
}

/// A link written for a document that has none: the link, none where one already stands. The looking and
/// the writing are one statement, so two askings at once cannot both find the document free.
pub async fn insert(db: &impl ConnectionTrait, share: &Share) -> Result<Option<Share>, Refusal> {
  one(
    db,
    &format!(
      r#"INSERT INTO "web_search_shares" ({COLUMNS}) VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT DO NOTHING RETURNING {COLUMNS}"#
    ),
    vec![
      share.token.clone().into(),
      share.search_id.into(),
      share.created_by.clone().into(),
      share.created_at.into(),
      share.expires_at.into(),
    ],
  )
  .await?
  .map(|row| share_of(&row))
  .transpose()
}

/// A link made for a document that has none, living `seconds` from `now` — or with no end, where that is
/// none. None where one already stands: a link is given another life by being taken away and made again, so
/// this never quietly replaces one somebody is still handing out.
pub async fn create(
  db: &impl ConnectionTrait,
  search_id: i64,
  created_by: &str,
  now: i64,
  seconds: Option<i64>,
) -> Result<Option<Share>, Refusal> {
  sweep(db, now).await?;

  let share = Share {
    token: share_token(),
    search_id,
    created_by: created_by.to_owned(),
    created_at: now,
    expires_at: seconds.map(|seconds| now + seconds),
  };

  insert(db, &share).await
}

/// A standing link made to live with no end.
pub async fn make_endless(db: &impl ConnectionTrait, search_id: i64) -> Result<(), Refusal> {
  run(
    db,
    r#"UPDATE "web_search_shares" SET "expires_at" = NULL WHERE "search_id" = $1"#,
    vec![search_id.into()],
  )
  .await?;

  Ok(())
}

/// The row one token names, as it stands: not swept first, so a caller can tell a link whose time has passed
/// from an address that never named one.
pub async fn by_token(db: &impl ConnectionTrait, token: &str) -> Result<Option<Share>, Refusal> {
  one(
    db,
    &format!(r#"SELECT {COLUMNS} FROM "web_search_shares" WHERE "share_token" = $1"#),
    vec![token.into()],
  )
  .await?
  .map(|row| share_of(&row))
  .transpose()
}

/// The link of one document taken away: whether there was one to take.
pub async fn remove(db: &impl ConnectionTrait, search_id: i64) -> Result<bool, Refusal> {
  Ok(
    run(db, r#"DELETE FROM "web_search_shares" WHERE "search_id" = $1"#, vec![search_id.into()]).await? > 0,
  )
}

/// Why a token opens nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closed {
  /// No link was ever made under it, or it was taken away.
  Unknown,
  /// Its time has passed.
  Expired,
}

/// The document one token opens at `now`, refused where it opens nothing. The expiry is read here every
/// time: the sweep is housekeeping, never the guard, and a row with no expiry passes, which is the whole of
/// what makes such a link endless.
pub async fn opened_by(
  db: &impl ConnectionTrait,
  token: &str,
  now: i64,
) -> Result<Result<i64, Closed>, Refusal> {
  if !is_share_token(token) {
    return Ok(Err(Closed::Unknown));
  }

  Ok(match by_token(db, token).await? {
    None => Err(Closed::Unknown),
    Some(share) if share.expires_at.is_some_and(|at| at <= now) => Err(Closed::Expired),
    Some(share) => Ok(share.search_id),
  })
}
