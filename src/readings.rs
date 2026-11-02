//! Readings (`docs/artifact.md`, "Readings"): a token opening one document's attachments to the page the
//! console's own reader opens, which is the document's number and the second the reading ends, signed with
//! the service's key. Nothing of a token is kept.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

/// How long a reading lasts, in seconds: twelve hours.
pub const READING_SECONDS: i64 = 12 * 60 * 60;

// How many bytes of the signature a token carries.
const SIGNATURE_BYTES: usize = 16;

/// The key reading tokens are signed with: drawn from the service's token where the settings name one, so a
/// reading outlives a restart, else drawn as the process starts.
#[derive(Clone)]
pub struct ReadingKey([u8; 32]);

impl std::fmt::Debug for ReadingKey {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter.write_str("ReadingKey")
  }
}

impl ReadingKey {
  /// The key for a service whose token is `token`.
  pub fn of(token: &str) -> Self {
    if token.is_empty() {
      Self(rand::random())
    } else {
      Self(Sha256::new().chain_update(b"reading:").chain_update(token.as_bytes()).finalize().into())
    }
  }

  fn signature(&self, opened: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(&self.0).expect("a key of any length");

    mac.update(opened);
    mac.finalize().into_bytes().into()
  }

  /// The token opening `document` from `now`, in epoch seconds, for a reading's length.
  pub fn issue(&self, document: i64, now: i64) -> String {
    let mut token = [document.to_be_bytes(), (now + READING_SECONDS).to_be_bytes()].concat();
    let signature = self.signature(&token);

    token.extend_from_slice(&signature[..SIGNATURE_BYTES]);

    URL_SAFE_NO_PAD.encode(token)
  }

  /// The document a token opens at `now`, none where its signature does not hold or its time has passed.
  pub fn opened(&self, token: &str, now: i64) -> Option<i64> {
    let bytes = URL_SAFE_NO_PAD.decode(token).ok()?;
    let (opened, signature) = (bytes.get(..16)?, bytes.get(16..)?);
    let mut mac = Hmac::<Sha256>::new_from_slice(&self.0).ok()?;

    (signature.len() == SIGNATURE_BYTES).then_some(())?;
    mac.update(opened);
    mac.verify_truncated_left(signature).ok()?;

    let document = i64::from_be_bytes(opened[..8].try_into().ok()?);
    let until = i64::from_be_bytes(opened[8..].try_into().ok()?);

    (now < until).then_some(document)
  }
}

/// Now, in epoch seconds.
pub fn now_seconds() -> i64 {
  chrono::Utc::now().timestamp()
}
