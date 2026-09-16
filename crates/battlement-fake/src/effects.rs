//! Observable one-shot effects executed by the fake host.

/// One audio playback occurrence.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioOccurrence {
  /// Command identity used for duplicate suppression.
  pub command_id: battlement::CommandId,
  /// Prepared audio clip that was played.
  pub address: battlement::AudioClipAddress,
  /// Requested initial volume.
  pub volume: f64,
  /// Requested playback pitch.
  pub pitch: f64,
  /// Whether the playback loops.
  pub looping: bool,
}

/// One temporary particle-effect occurrence.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleOccurrence {
  /// Command identity used for duplicate suppression.
  pub command_id: battlement::CommandId,
  /// Prepared particle-effect prefab that was spawned.
  pub address: battlement::PrefabAddress,
  /// Authored spawn location.
  pub location: battlement::ParticleSpawnLocation,
  /// Requested effect lifetime in milliseconds.
  pub lifetime_ms: u64,
}
