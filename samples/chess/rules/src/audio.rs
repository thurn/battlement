//! Sound roles used by the chess presentation.
//!
//! Keeping the role-to-asset mapping in Rust lets the rules and app effects talk
//! in terms of events such as “capture” or “invalid drop.” The generated
//! [`crate::assets`] module remains the lower-level catalog of Unity addresses.

use crate::assets::sfx;
use battlement::AudioClipAddress;

/// Addresses of NotJam's sound-effect collection.
pub const SOUND_EFFECTS: [AudioClipAddress; 41] = [
  sfx::ACCEPT,
  sfx::ALARM,
  sfx::ATTACK_A,
  sfx::ATTACK_B,
  sfx::ATTACK_C,
  sfx::ATTACK_D,
  sfx::BLEEP_WHITE_NOISE,
  sfx::BOOST_PAD,
  sfx::BOUNCE_0,
  sfx::BOUNCE_1,
  sfx::BOUNCE_2,
  sfx::BOUNCE_3,
  sfx::CHIRP_A,
  sfx::CHIRP_CRUNCH,
  sfx::CHIRP_WHITE_NOISE,
  sfx::CLICK,
  sfx::CLICK_2,
  sfx::CLICK_3,
  sfx::CLICK_4,
  sfx::CRUNCH_A,
  sfx::CRUNCH_B,
  sfx::DASH,
  sfx::ERROR,
  sfx::EXIT_SCENE_TRANSITION,
  sfx::FALL_AND_DIE,
  sfx::GRAPPLE,
  sfx::GREEN_LIGHT_TONE,
  sfx::LAP_COMPLETE,
  sfx::LOCKON_AVAILABLE,
  sfx::POWERUP_A,
  sfx::POWERUP_B,
  sfx::POWERUP_CURSED,
  sfx::RED_LIGHT_TONE,
  sfx::RISING_METALLIC,
  sfx::RISING_TONE_EXPLOSION,
  sfx::RISING_TONE_METALLIC,
  sfx::SCENE_TRANSITION,
  sfx::SIREN_EXPLOSION,
  sfx::SLINGSHOT,
  sfx::SWIPE_METALLIC,
  sfx::WOBBLE_FALLING_TONE,
];
/// Variants rotated through when the player selects a piece.
pub const PICKUP_SOUNDS: [AudioClipAddress; 4] =
  [sfx::CLICK, sfx::CLICK_2, sfx::CLICK_3, sfx::CLICK_4];
/// Variants selected for captures so repeated moves sound less mechanical.
pub const CAPTURE_SOUNDS: [AudioClipAddress; 4] =
  [sfx::ATTACK_A, sfx::ATTACK_B, sfx::ATTACK_C, sfx::ATTACK_D];
/// Variants selected when an ordinary move reaches its destination.
pub const DROP_SOUNDS: [AudioClipAddress; 4] =
  [sfx::BOUNCE_0, sfx::BOUNCE_1, sfx::BOUNCE_2, sfx::BOUNCE_3];
/// Feedback for an illegal move or a rejected drag.
pub const INVALID_DROP_SOUND: AudioClipAddress = sfx::ERROR;
/// Arrival sound for castling.
pub const CASTLE_SOUND: AudioClipAddress = sfx::POWERUP_A;
/// Arrival sound for promotion.
pub const PROMOTION_SOUND: AudioClipAddress = sfx::POWERUP_B;
/// Follow-up sound when a move leaves the opponent in check.
pub const CHECK_SOUND: AudioClipAddress = sfx::ALARM;
/// Sound that begins a fresh-game opening sequence.
pub const START_SOUND: AudioClipAddress = sfx::ACCEPT;
/// Sound that accompanies a board reset.
pub const RESET_SOUND: AudioClipAddress = sfx::SCENE_TRANSITION;
/// Feedback for increasing music volume.
pub const VOLUME_UP_SOUND: AudioClipAddress = sfx::CHIRP_A;
/// Feedback for decreasing music volume.
pub const VOLUME_DOWN_SOUND: AudioClipAddress = sfx::CHIRP_CRUNCH;
/// Terminal sound when the human player wins.
pub const PLAYER_WIN_SOUND: AudioClipAddress = sfx::LAP_COMPLETE;
/// Terminal sound when the computer wins.
pub const PLAYER_LOSS_SOUND: AudioClipAddress = sfx::FALL_AND_DIE;

/// Terminal sound for stalemate or another drawn position.
pub const DRAW_SOUND: AudioClipAddress = sfx::WOBBLE_FALLING_TONE;
