//! Prepared assets are immutable and shared; mutable hosts remain fresh per test.
use battlement::AudioClipAddress;
use battlement::PrefabAddress;
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use chess_rules::{assets, audio};
use cozy_chess::{Color, Piece};
use std::sync::{Arc, OnceLock};

// This mapping intentionally does not call the renderer's piece-to-asset function.
// If the renderer draws a bishop using a pawn prefab, the assertion must notice.
pub const PIECES: [(PrefabAddress, Color, Piece); 12] = [
  (assets::white::PAWN, Color::White, Piece::Pawn),
  (assets::white::ROOK, Color::White, Piece::Rook),
  (assets::white::KNIGHT, Color::White, Piece::Knight),
  (assets::white::BISHOP, Color::White, Piece::Bishop),
  (assets::white::QUEEN, Color::White, Piece::Queen),
  (assets::white::KING, Color::White, Piece::King),
  (assets::black::PAWN, Color::Black, Piece::Pawn),
  (assets::black::ROOK, Color::Black, Piece::Rook),
  (assets::black::KNIGHT, Color::Black, Piece::Knight),
  (assets::black::BISHOP, Color::Black, Piece::Bishop),
  (assets::black::QUEEN, Color::Black, Piece::Queen),
  (assets::black::KING, Color::Black, Piece::King),
];
pub const MUSIC: [AudioClipAddress; 4] = [
  assets::music::CRITICAL,
  assets::music::SWITCH_WITH_ME,
  assets::music::BREAKBEAT_CHIPS,
  assets::music::DRAG_AND_DREAD,
];

pub fn assets() -> Arc<FakeAssetCatalog> {
  static CATALOG: OnceLock<Arc<FakeAssetCatalog>> = OnceLock::new();
  CATALOG
    .get_or_init(|| {
      let mut c = FakeAssetCatalog::new();
      c.add_scene(assets::CONTENT);
      for (address, _, _) in PIECES {
        c.add_prefab(
          address,
          FakePrefab::new()
            .with_material_slots(1)
            .with_pointer_collider(),
        );
      }
      for address in MUSIC.into_iter().chain(audio::SOUND_EFFECTS) {
        c.add_audio_clip(address);
      }
      c.add_texture(assets::PLAY_BUTTON);
      c.add_texture(assets::REFRESH_BUTTON);
      c.add_material(assets::LEGAL_SQUARE);
      c.add_prefab(
        assets::effects::PIECE_SELECTED,
        FakePrefab::new().with_particle_systems(),
      );
      c.add_particle_effect(assets::effects::PIECE_SPAWN);
      c.add_particle_effect(assets::effects::CAPTURE);
      Arc::new(c)
    })
    .clone()
}
