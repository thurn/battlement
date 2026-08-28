use std::{fs, io::ErrorKind, path::Path};

use battlement_native::EngineError;
use cozy_chess::Board;
use serde::{Deserialize, Serialize};

use crate::ChessEngine;

const SAVE_FILE: &str = "chess-game.json";

#[derive(Deserialize, Serialize)]
struct SavedGame {
  position: String,
}

pub fn load(directory: &Path) -> Option<Board> {
  serde_json::from_slice::<SavedGame>(&fs::read(directory.join(SAVE_FILE)).ok()?)
    .ok()?
    .position
    .parse()
    .ok()
}

pub fn save(directory: &Path, board: &Board) -> Result<(), String> {
  fs::create_dir_all(directory)
    .map_err(|error| format!("could not create persistent data directory: {error}"))?;
  fs::write(
    directory.join(SAVE_FILE),
    serde_json::to_vec_pretty(&SavedGame {
      position: board.to_string(),
    })
    .map_err(|error| format!("could not serialize chess game: {error}"))?,
  )
  .map_err(|error| format!("could not persist chess game: {error}"))
}

fn clear(directory: &Path) -> Result<(), String> {
  match fs::remove_file(directory.join(SAVE_FILE)) {
    Ok(()) => Ok(()),
    Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
    Err(error) => Err(format!("could not clear persisted chess game: {error}")),
  }
}

impl ChessEngine {
  pub(crate) fn clear_persisted_board(&self) -> Result<(), EngineError> {
    let Some(path) = &self.persistent_data_path else {
      return Ok(());
    };
    self::clear(path).map_err(EngineError::new)
  }

  pub(crate) fn persist_board(&self) -> Result<(), EngineError> {
    let Some(path) = &self.persistent_data_path else {
      return Ok(());
    };
    self::save(path, &self.board).map_err(EngineError::new)
  }
}
