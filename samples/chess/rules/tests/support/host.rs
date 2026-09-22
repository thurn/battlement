//! Queries against decoded host output. This is the only board-geometry adapter.
use crate::support::catalog;
use battlement::{AudioClipAddress, PhysicalKey, TextureAddress};
use battlement::{GameObjectKind, Vector3};
use cozy_chess::{Color, Piece, Square};
use reactant_testing::Display;

pub fn center(square: Square) -> Vector3 {
  Vector3::new(
    square.file() as u8 as f64 - 3.5,
    0.0,
    square.rank() as u8 as f64 - 3.5,
  )
}

pub fn pieces(display: &Display) -> Vec<(Vector3, Color, Piece)> {
  display
    .objects()
    .filter(|o| o.active_in_hierarchy())
    .filter_map(|o| {
      let GameObjectKind::Prefab { address, .. } = o.kind() else {
        return None;
      };
      let (_, color, piece) = catalog::PIECES.into_iter().find(|(a, _, _)| a == address)?;
      let pose = display.world().world_transform(o.id());
      (pose.scale.x.abs() > 0.001 && pose.scale.y.abs() > 0.001 && pose.scale.z.abs() > 0.001)
        .then_some((pose.position, color, piece))
    })
    .collect()
}

pub fn at(display: &Display, square: Square) -> Vec<(Color, Piece)> {
  let p = center(square);
  pieces(display)
    .into_iter()
    .filter(|(v, _, _)| {
      (v.x - p.x).abs() < 0.001 && (v.y - p.y).abs() < 0.001 && (v.z - p.z).abs() < 0.001
    })
    .map(|(_, c, p)| (c, p))
    .collect()
}

pub fn expect_piece(display: &Display, square: Square, color: Color, piece: Piece) {
  assert_eq!(
    at(display, square),
    vec![(color, piece)],
    "rendered piece at {square}"
  );
}

pub fn expect_empty(display: &Display, square: Square) {
  assert!(
    at(display, square).is_empty(),
    "unexpected rendered piece at {square}"
  );
}

pub fn sound(display: &Display, address: AudioClipAddress) {
  assert!(
    display
      .audio_occurrences()
      .iter()
      .any(|o| o.address == address),
    "missing sound {address:?}"
  );
}

// Screen coordinates come from the live camera. Actions cross geometric picking,
// input policy, synchronous rules, reconciliation, wire encoding, and the fake host.
// The helper never finds a private piece ID and never invokes a chess action directly.
pub fn move_piece(display: &mut Display, from: Square, to: Square) {
  assert_eq!(
    at(display, from).len(),
    1,
    "source must have exactly one visible piece; pieces: {:?}",
    pieces(display)
  );
  let mut source = center(from);
  source.y = 0.6; // Aim above the base so a nearer rank does not occlude the piece.
  let start = display.project_world(source).expect("source inside camera");
  let end = display
    .project_world(center(to))
    .expect("destination inside camera");
  display.pointer_down(0, start);
  display.pointer_move(0, end, true);
  display.pointer_up(0, end);
}

pub fn key(display: &mut Display, key: PhysicalKey) {
  display.key_down(key);
  display.key_up(key);
}

pub fn image_click(display: &mut Display, address: TextureAddress) {
  let positions = display
    .images()
    .filter(|(o, image)| o.active_in_hierarchy() && image.texture == address)
    .map(|(o, _)| display.world_point(o.id(), Vector3::ZERO))
    .collect::<Vec<_>>();
  assert_eq!(positions.len(), 1, "visible control must be unique");
  display.click_at(
    display
      .project_world(positions[0])
      .expect("control inside camera"),
  );
}
