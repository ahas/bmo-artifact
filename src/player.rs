//! bmo-ui's audio player as a hosted page loads it (`docs/api.md`, "Readings"; `docs/layout.md`, "The console
//! bundle"): its script, named by the digest of its bytes, and its words in each locale, read from the folder the
//! build of `client/` writes them into when the service starts.

use std::path::Path;

use serde_json::Value;
use sha2::{Digest, Sha256};

// The files the build writes into the player folder.
const SCRIPT_FILE: &str = "bmo-audio-player.js";
const WORDS_FILE: &str = "words.json";

// How many hexadecimal digits of the script's digest its name carries.
const DIGEST_DIGITS: usize = 16;

/// The language the player speaks where the reader names none its words hold.
pub const PLAYER_LANGUAGE: &str = "en";

/// The player's script: the name its address ends in, carrying the digest of its bytes, and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerScript {
  pub name: String,
  pub bytes: Vec<u8>,
}

/// What the player folder holds; a file it lacks is none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Player {
  pub script: Option<PlayerScript>,
  /// Each locale's player words under its code.
  words: Value,
}

impl Player {
  /// What the folder holds.
  pub fn read(folder: &Path) -> Self {
    let script = std::fs::read(folder.join(SCRIPT_FILE)).ok().map(|bytes| {
      let digest: String = Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect();
      let (stem, extension) = SCRIPT_FILE.split_once('.').unwrap_or((SCRIPT_FILE, ""));

      PlayerScript {
        name: format!("{stem}.{}.{extension}", &digest[..DIGEST_DIGITS]),
        bytes,
      }
    });
    let words = std::fs::read(folder.join(WORDS_FILE))
      .ok()
      .and_then(|words| serde_json::from_slice(&words).ok())
      .unwrap_or_default();

    Self { script, words }
  }

  /// The script under the name carrying its digest, none for any other name.
  pub fn script(&self, name: &str) -> Option<&PlayerScript> {
    self.script.as_ref().filter(|script| script.name == name)
  }

  /// The player's words in the language `language` names, else in English, and the language they are in.
  pub fn words(&self, language: Option<&str>) -> (String, Value) {
    let spoken = language.filter(|language| self.words[*language].is_object()).unwrap_or(PLAYER_LANGUAGE);

    (spoken.to_owned(), self.words[spoken].clone())
  }
}
