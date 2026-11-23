//! The copies of kept pictures drawn small, kept in memory within a bound (`docs/artifact.md`, "A picture
//! drawn small"): the latest made or read stay, the oldest going first once the bound is passed.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, LazyLock, Mutex};

/// How many bytes of copies are kept.
const KEPT_BYTES: usize = 64 * 1024 * 1024;

/// One copy: its bytes and the type they are written in.
pub struct Copy {
  pub bytes: Arc<[u8]>,
  pub media_type: &'static str,
}

#[derive(Default)]
struct Copies {
  by_key: HashMap<String, (Arc<[u8]>, &'static str)>,
  // The keys from the one let go first to the one let go last.
  order: VecDeque<String>,
  bytes: usize,
}

static COPIES: LazyLock<Mutex<Copies>> = LazyLock::new(Mutex::default);

/// The copy kept under `key`, which moves it to the back of the line to be let go.
pub fn kept(key: &str) -> Option<Copy> {
  let mut copies = COPIES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
  let (bytes, media_type) = copies.by_key.get(key).cloned()?;

  copies.order.retain(|one| one != key);
  copies.order.push_back(key.to_owned());

  Some(Copy { bytes, media_type })
}

/// Keeps a copy just made under `key` and hands it back, letting the oldest go past the bound.
pub fn keep(key: String, bytes: Vec<u8>, media_type: &'static str) -> Copy {
  let bytes: Arc<[u8]> = bytes.into();
  let mut copies = COPIES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

  if bytes.len() <= KEPT_BYTES && !copies.by_key.contains_key(&key) {
    copies.bytes += bytes.len();
    copies.by_key.insert(key.clone(), (bytes.clone(), media_type));
    copies.order.push_back(key);

    while copies.bytes > KEPT_BYTES {
      let Some(oldest) = copies.order.pop_front() else { break };

      if let Some((gone, _)) = copies.by_key.remove(&oldest) {
        copies.bytes -= gone.len();
      }
    }
  }

  Copy { bytes, media_type }
}
