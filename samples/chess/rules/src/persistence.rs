use std::{fs, path::Path};

use cozy_chess::Board;
use serde::{Deserialize, Serialize};

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
