use serde::{Deserialize, Serialize};

use crate::{AudioClipAddress, CommandId, ObjectId, PrefabAddress, Tween, Vector3};

/// Recursively plays particle systems rooted at an object.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParticlePlayPayload {
  /// Target game object whose hierarchy contains particle systems.
  pub object_id: ObjectId,
  /// Whether to restart systems that are already playing.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub restart: bool,
}

/// Recursively stops particle systems rooted at an object.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParticleStopPayload {
  /// Target game object whose hierarchy contains particle systems.
  pub object_id: ObjectId,
  /// Whether to clear live particles after stopping.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub clear: bool,
}

/// Spawns a prepared temporary particle-effect prefab.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParticleSpawnPayload {
  /// Prepared prefab address whose hierarchy contains particle systems.
  pub address: PrefabAddress,
  /// Source of the effect's initial world position.
  pub location: ParticleSpawnLocation,
  /// Positive effect lifetime in milliseconds.
  pub lifetime_ms: u64,
}

/// Source of a temporary particle effect's initial world position.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum ParticleSpawnLocation {
  /// Use a game object's current world position.
  GameObject(ObjectId),
  /// Use an explicit world-space position.
  WorldPosition(Vector3),
}

/// Plays a prepared audio clip through a Battlement-owned 2D audio source.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AudioPlayPayload {
  /// Prepared audio-clip address.
  pub address: AudioClipAddress,
  /// Initial volume in the inclusive range `[0, 1]`.
  #[serde(default = "crate::default_one", skip_serializing_if = "crate::is_one")]
  pub volume: f64,
  /// Playback pitch in the range `(0, 3]`.
  #[serde(default = "crate::default_one", skip_serializing_if = "crate::is_one")]
  pub pitch: f64,
  /// Whether playback loops until explicitly stopped.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub r#loop: bool,
  /// Fade-in duration in milliseconds.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub fade_in_ms: u64,
}

/// Stops audio started by an earlier audio-play command.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct AudioStopPayload {
  /// Command and operation identity of the audio playback.
  pub audio_command_id: CommandId,
  /// Fade-out duration in milliseconds.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub fade_out_ms: u64,
}

/// Sets a playing audio operation's volume.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct AudioVolumePayload {
  /// Command and operation identity of the audio playback.
  pub audio_command_id: CommandId,
  /// Requested volume in the inclusive range `[0, 1]`.
  pub volume: f64,
}

/// Tweens a playing audio operation's volume.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TweenAudioVolumePayload {
  /// Command and operation identity of the audio playback.
  pub audio_command_id: CommandId,
  /// Requested final volume in the inclusive range `[0, 1]`.
  pub volume: f64,
  /// Tween timing and repetition.
  pub tween: Tween,
}
