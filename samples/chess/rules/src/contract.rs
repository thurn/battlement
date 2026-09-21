//! Stable host-visible constants for black-box validation.

use battlement::{
  AudioClipAddress, MaterialAddress, ObjectId, PrefabAddress, SceneAddress, TextureAddress,
  object_id,
};

/// Stable identity of the Reactant document root.
pub const ROOT_ID: ObjectId = object_id!("43000000-0000-4000-8000-000000000002");
/// Stable identity of the title-screen Play button.
pub const PLAY_BUTTON_ID: ObjectId = object_id!("4cf7cb75-ec8f-44ec-88c9-c83ca3869f43");
/// Stable identity of the board refresh button.
pub const REFRESH_BUTTON_ID: ObjectId = object_id!("35b288b3-6d72-48af-aeb9-e8f11d63e3ea");
/// File name used for the saved chess position.
pub const SAVE_FILE_NAME: &str = "chess-game.json";
/// Complete duration of the staggered opening presentation.
pub const PIECE_SPAWN_SEQUENCE_DURATION_MS: u64 = 5_070;

/// Content scene loaded by the chess application.
pub const CONTENT: SceneAddress = crate::assets::CONTENT;
/// Material used for legal-move highlights.
pub const LEGAL_SQUARE: MaterialAddress = crate::assets::LEGAL_SQUARE;
/// Title-screen image asset.
pub const PLAY_BUTTON: TextureAddress = crate::assets::PLAY_BUTTON;
/// Refresh image asset.
pub const REFRESH_BUTTON: TextureAddress = crate::assets::REFRESH_BUTTON;
/// Piece prefab addresses prepared by the Unity host.
pub const PIECE_PREFABS: [PrefabAddress; 12] = [
  crate::assets::white::PAWN,
  crate::assets::white::ROOK,
  crate::assets::white::KNIGHT,
  crate::assets::white::BISHOP,
  crate::assets::white::QUEEN,
  crate::assets::white::KING,
  crate::assets::black::PAWN,
  crate::assets::black::ROOK,
  crate::assets::black::KNIGHT,
  crate::assets::black::BISHOP,
  crate::assets::black::QUEEN,
  crate::assets::black::KING,
];
/// White knight prefab used to validate promotion.
pub const WHITE_KNIGHT: PrefabAddress = crate::assets::white::KNIGHT;
/// Selection effect prefab.
pub const PIECE_SELECTED_EFFECT: PrefabAddress = crate::assets::effects::PIECE_SELECTED;
/// Opening particle effect prefab.
pub const PIECE_SPAWN_EFFECT: PrefabAddress = crate::assets::effects::PIECE_SPAWN;
/// Capture particle effect prefab.
pub const CAPTURE_EFFECT: PrefabAddress = crate::assets::effects::CAPTURE;
/// Playlist in playback order.
pub const MUSIC_TRACKS: [AudioClipAddress; 4] = crate::reactant_effects::MUSIC_TRACKS;
/// Sound effects prepared by the Unity host.
pub const SOUND_EFFECTS: [AudioClipAddress; 41] = crate::audio::SOUND_EFFECTS;
/// Capture sounds selected by the seeded presentation RNG.
pub const CAPTURE_SOUNDS: [AudioClipAddress; 4] = crate::audio::CAPTURE_SOUNDS;
/// Invalid-drop feedback.
pub const INVALID_DROP_SOUND: AudioClipAddress = crate::audio::INVALID_DROP_SOUND;
/// Castling feedback.
pub const CASTLE_SOUND: AudioClipAddress = crate::audio::CASTLE_SOUND;
/// Board-start feedback.
pub const START_SOUND: AudioClipAddress = crate::audio::START_SOUND;
/// Board-reset feedback.
pub const RESET_SOUND: AudioClipAddress = crate::audio::RESET_SOUND;

/// Host names of semantically observable UI states.
pub mod marker {
  /// Title screen.
  pub const TITLE: &str = "screen.title";
  /// Fresh board.
  pub const INITIAL: &str = "board.initial";
  /// Selected piece and legal targets.
  pub const SELECTED: &str = "selection.legal-targets";
  /// Accepted player move.
  pub const PLAYER_MOVE: &str = "move.committed";
  /// Accepted computer reply.
  pub const AI_RESPONSE: &str = "turn.ai-response";
  /// Capture.
  pub const CAPTURE: &str = "move.capture";
  /// Castling.
  pub const CASTLE: &str = "special.castle";
  /// En passant.
  pub const EN_PASSANT: &str = "special.en-passant";
  /// Promotion.
  pub const PROMOTION: &str = "special.promotion";
  /// Check.
  pub const CHECK: &str = "feedback.check";
  /// Human victory.
  pub const PLAYER_WIN: &str = "terminal.player-win";
  /// Computer victory.
  pub const COMPUTER_WIN: &str = "terminal.computer-win";
  /// Draw.
  pub const DRAW: &str = "terminal.draw";
  /// Pause overlay.
  pub const PAUSED: &str = "menu.paused";
  /// Menu-driven refresh.
  pub const REFRESHED: &str = "board.refreshed";
  /// Shortcut-driven restart.
  pub const RESTARTED: &str = "board.restarted";
  /// Restored saved game.
  pub const RESUMED: &str = "board.resumed";
}
