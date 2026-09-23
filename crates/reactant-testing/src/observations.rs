//! Queries inspect decoded host state, independently of the application model.
use crate::Display;
use battlement::{
  AccessibilityNodeSnapshot, AudioClipAddress, CommandBody, GameObjectKind, PhysicalKey,
  PrefabAddress, SemanticRole, Vector3,
};
use battlement_native::Engine;
use std::time::Duration;

impl<E: Engine> Display<E> {
  /// Returns visible prefab identities and composed world positions, ignoring IDs
  /// and wrapper structure. Inactive and zero-scale objects are not displayed.
  pub fn prefabs(&self) -> Vec<(Vector3, PrefabAddress)> {
    self
      .objects()
      .filter(|object| object.active_in_hierarchy())
      .filter_map(|object| {
        let GameObjectKind::Prefab { address, .. } = object.kind() else {
          return None;
        };
        let pose = self.world().world_transform(object.id());
        [pose.scale.x, pose.scale.y, pose.scale.z]
          .iter()
          .all(|v| v.abs() > 0.001)
          .then(|| (pose.position, address.clone()))
      })
      .collect()
  }

  /// Requires an exact number of visible instances of an asset.
  pub fn expect_prefab_count(&self, address: PrefabAddress, count: usize) {
    assert_eq!(
      self
        .prefabs()
        .iter()
        .filter(|(_, asset)| *asset == address)
        .count(),
      count,
      "visible instances of {address:?}"
    );
  }

  /// Finds visible prefab assets within 0.001 world units of a position.
  /// The tolerance accommodates floating-point camera projection and transform composition.
  pub fn prefabs_at(&self, position: Vector3) -> Vec<PrefabAddress> {
    self
      .prefabs()
      .into_iter()
      .filter(|(point, _)| {
        [
          point.x - position.x,
          point.y - position.y,
          point.z - position.z,
        ]
        .iter()
        .all(|delta| delta.abs() < 0.001)
      })
      .map(|(_, asset)| asset)
      .collect()
  }

  /// Finds exactly one currently presented node by its accessible label.
  pub fn semantic_node(&self, label: &str) -> &AccessibilityNodeSnapshot {
    let mut matches = self
      .accessibility()
      .nodes
      .iter()
      .filter(|node| node.label.as_deref() == Some(label));
    let node = matches
      .next()
      .unwrap_or_else(|| panic!("missing accessible control: {label}"));
    assert!(
      matches.next().is_none(),
      "ambiguous accessible control: {label}"
    );
    node
  }

  /// Requires an accessible, activatable button.
  pub fn expect_button(&self, label: &str) {
    let node = self.semantic_node(label);
    assert_eq!(node.role, SemanticRole::Button);
    assert!(node.actions.activate, "button cannot be activated: {label}");
  }

  /// Requires a globally enabled physical key.
  pub fn expect_key_enabled(&self, key: PhysicalKey) {
    assert!(
      self.global_keys().contains(&key),
      "key is disabled: {key:?}"
    );
  }

  /// Requires an enabled primary-controller button.
  pub fn expect_controller_button_enabled(&self, button: battlement::ControllerButton) {
    assert!(
      self
        .controller_input()
        .expect("controller input enabled")
        .buttons
        .contains(&button)
    );
  }

  /// Returns played clips in execution order, restricted to an address prefix.
  pub fn sounds(&self, prefix: &str) -> Vec<AudioClipAddress> {
    self
      .audio_occurrences()
      .iter()
      .filter(|sound| sound.address.as_str().starts_with(prefix))
      .map(|sound| sound.address.clone())
      .collect()
  }

  /// Requires an executed occurrence of this audio clip.
  pub fn expect_sound(&self, address: AudioClipAddress) {
    self.expect_any_sound(&[address]);
  }

  /// Requires an executed occurrence of one of these clips.
  pub fn expect_any_sound(&self, addresses: &[AudioClipAddress]) {
    assert!(
      self
        .audio_occurrences()
        .iter()
        .any(|sound| addresses.contains(&sound.address)),
      "missing sound from {addresses:?}"
    );
  }

  /// Requires an exact number of occurrences of this temporary effect.
  pub fn expect_particles(&self, address: PrefabAddress, count: usize) {
    assert_eq!(
      self
        .particle_occurrences()
        .iter()
        .filter(|effect| effect.address == address)
        .count(),
      count,
      "particle occurrences for {address:?}"
    );
  }

  /// Checks both halves of an audible transition in the executed command journal.
  pub fn expect_crossfade(&self, address: AudioClipAddress, duration: Duration) {
    assert!(self.commands().iter().any(|entry| matches!(&entry.command.body, CommandBody::AudioPlay(play) if play.address == address && u128::from(play.fade_in_ms) == duration.as_millis())), "missing fade-in for {address:?}");
    assert!(self.commands().iter().any(|entry| matches!(&entry.command.body, CommandBody::AudioStop(stop) if u128::from(stop.fade_out_ms) == duration.as_millis())), "missing fade-out");
  }
}
