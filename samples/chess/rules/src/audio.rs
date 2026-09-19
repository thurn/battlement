use crate::assets::sfx;
use battlement::AudioClipAddress;

pub(crate) const PICKUP_SOUNDS: [AudioClipAddress; 4] =
  [sfx::CLICK, sfx::CLICK_2, sfx::CLICK_3, sfx::CLICK_4];
pub(crate) const CAPTURE_SOUNDS: [AudioClipAddress; 4] =
  [sfx::ATTACK_A, sfx::ATTACK_B, sfx::ATTACK_C, sfx::ATTACK_D];
pub(crate) const DROP_SOUNDS: [AudioClipAddress; 4] =
  [sfx::BOUNCE_0, sfx::BOUNCE_1, sfx::BOUNCE_2, sfx::BOUNCE_3];
pub(crate) const INVALID_DROP_SOUND: AudioClipAddress = sfx::ERROR;
pub(crate) const CASTLE_SOUND: AudioClipAddress = sfx::POWERUP_A;
pub(crate) const PROMOTION_SOUND: AudioClipAddress = sfx::POWERUP_B;
pub(crate) const CHECK_SOUND: AudioClipAddress = sfx::ALARM;
pub(crate) const START_SOUND: AudioClipAddress = sfx::ACCEPT;
pub(crate) const RESET_SOUND: AudioClipAddress = sfx::SCENE_TRANSITION;
pub(crate) const VOLUME_UP_SOUND: AudioClipAddress = sfx::CHIRP_A;
pub(crate) const VOLUME_DOWN_SOUND: AudioClipAddress = sfx::CHIRP_CRUNCH;
pub(crate) const PLAYER_WIN_SOUND: AudioClipAddress = sfx::LAP_COMPLETE;
pub(crate) const PLAYER_LOSS_SOUND: AudioClipAddress = sfx::FALL_AND_DIE;
pub(crate) const DRAW_SOUND: AudioClipAddress = sfx::WOBBLE_FALLING_TONE;

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
