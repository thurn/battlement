//! The chess-specific adapter: board squares and asset names are visual vocabulary.
//! Expected assets never come from the renderer's piece-to-prefab function.
use battlement::{PrefabAddress, Vector3};
use cozy_chess::{Color, Piece, Square};
use reactant_testing::Display;

pub fn center(square: Square) -> Vector3 {
  Vector3::new(
    square.file() as u8 as f64 - 3.5,
    0.0,
    square.rank() as u8 as f64 - 3.5,
  )
}

pub fn grab_point(square: Square) -> Vector3 {
  // Aim above the base to avoid occlusion by the nearer rank.
  Vector3 {
    y: 0.6,
    ..self::center(square)
  }
}

pub fn pieces(display: &Display) -> Vec<(Vector3, PrefabAddress)> {
  display
    .prefabs()
    .into_iter()
    .filter(|(_, asset)| self::is_piece(asset))
    .collect()
}

pub fn at(display: &Display, square: Square) -> Vec<PrefabAddress> {
  display
    .prefabs_at(self::center(square))
    .into_iter()
    .filter(self::is_piece)
    .collect()
}

pub fn expect_piece(display: &Display, square: Square, color: Color, piece: Piece) {
  let asset = format!("{color:?}/{piece:?}").to_lowercase();
  assert_eq!(
    self::at(display, square),
    vec![PrefabAddress::from(asset)],
    "rendered piece at {square}"
  );
}

pub fn expect_empty(display: &Display, square: Square) {
  assert!(
    self::at(display, square).is_empty(),
    "unexpected rendered piece at {square}"
  );
}

fn is_piece(asset: &PrefabAddress) -> bool {
  asset.as_str().starts_with("white/") || asset.as_str().starts_with("black/")
}
