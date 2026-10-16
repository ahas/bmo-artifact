//! bmo-gate (`docs/api.md`, "bmo-gate"): the knowledge a piece of work is handed, picked by the gate the way
//! it picks a turn's reference.

use serde::{Deserialize, Serialize};

use super::json::Unit;
use crate::verbs::research::Piece;

#[derive(Serialize)]
struct Asked<'a> {
  chat_id: i64,
  lines: &'a [String],
}

#[derive(Deserialize)]
struct Picked {
  pieces: Vec<PickedPiece>,
}

#[derive(Deserialize)]
struct PickedPiece {
  path: String,
  text: String,
}

/// bmo-gate at the address the settings name, where they name one.
#[derive(Clone, Debug)]
pub struct Gate {
  unit: Unit,
}

impl Gate {
  pub fn new(address: Option<String>) -> Self {
    Self {
      unit: Unit::new("bmo-gate", "gate", address),
    }
  }

  /// The pieces of knowledge a piece of work in a room is handed for its own lines.
  pub async fn work_knowledge(&self, chat_id: i64, lines: &[String]) -> Result<Vec<Piece>, String> {
    let picked: Picked = self.unit.post("/knowledge/work", &Asked { chat_id, lines }).await?;

    Ok(
      picked
        .pieces
        .into_iter()
        .map(|piece| Piece {
          path: piece.path,
          text: piece.text,
        })
        .collect(),
    )
  }
}
