use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{AudioClipAddress, CommandId, ObjectId, ParticleEffectAddress, Tween, Vector3};

/// Recursively plays particle systems rooted at an object.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ParticlePlayPayload {
    /// Target game object whose hierarchy contains particle systems.
    pub object_id: ObjectId,
    /// Whether to restart systems that are already playing.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub restart: bool,
}

/// Recursively stops particle systems rooted at an object.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ParticleStopPayload {
    /// Target game object whose hierarchy contains particle systems.
    pub object_id: ObjectId,
    /// Whether to clear live particles after stopping.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub clear: bool,
}

/// Spawns a prepared temporary particle-effect prefab.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ParticleSpawnPayload {
    /// Prepared particle-effect-prefab address.
    pub address: ParticleEffectAddress,
    /// Source of the effect's initial world position.
    pub location: ParticleSpawnLocation,
    /// Positive effect lifetime in milliseconds.
    #[schemars(range(min = 1, max = 86_400_000))]
    pub lifetime_ms: u64,
}

/// Source of a temporary particle effect's initial world position.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub enum ParticleSpawnLocation {
    /// Use a game object's current world position.
    GameObject(ObjectId),
    /// Use an explicit world-space position.
    WorldPosition(Vector3),
}

/// Plays a prepared audio clip through a Masonry-owned 2D audio source.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AudioPlayPayload {
    /// Prepared audio-clip address.
    pub address: AudioClipAddress,
    /// Initial volume in the inclusive range `[0, 1]`.
    #[serde(
        default = "crate::serialization::default_one",
        skip_serializing_if = "crate::serialization::is_one"
    )]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub volume: f64,
    /// Playback pitch in the range `(0, 3]`.
    #[serde(
        default = "crate::serialization::default_one",
        skip_serializing_if = "crate::serialization::is_one"
    )]
    #[schemars(range(min = 0.0, max = 3.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub pitch: f64,
    /// Whether playback loops until explicitly stopped.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub r#loop: bool,
    /// Fade-in duration in milliseconds.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    #[schemars(range(max = 86_400_000))]
    pub fade_in_ms: u64,
}

/// Stops audio started by an earlier audio-play command.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AudioStopPayload {
    /// Command and operation identity of the audio playback.
    pub audio_command_id: CommandId,
    /// Fade-out duration in milliseconds.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    #[schemars(range(max = 86_400_000))]
    pub fade_out_ms: u64,
}

/// Sets a playing audio operation's volume.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AudioVolumePayload {
    /// Command and operation identity of the audio playback.
    pub audio_command_id: CommandId,
    /// Requested volume in the inclusive range `[0, 1]`.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub volume: f64,
}

/// Tweens a playing audio operation's volume.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenAudioVolumePayload {
    /// Command and operation identity of the audio playback.
    pub audio_command_id: CommandId,
    /// Requested final volume in the inclusive range `[0, 1]`.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub volume: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}
