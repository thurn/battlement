use std::collections::BTreeSet;

use crate::{DITTO_VISUAL_STATE_REGISTRY, Screen};

#[test]
fn screen_inventory_matches_the_ditto_registry() {
  assert_eq!(
    DITTO_VISUAL_STATE_REGISTRY.matches("[[states]]").count(),
    40
  );
  let registered_screens = DITTO_VISUAL_STATE_REGISTRY
    .lines()
    .filter_map(|line| line.strip_prefix("screen = \"")?.strip_suffix('"'))
    .collect::<BTreeSet<_>>();
  assert_eq!(registered_screens.len(), Screen::ALL.len());
  for screen in Screen::ALL {
    assert!(
      registered_screens.contains(screen.registry_key()),
      "registry is missing {}",
      screen.registry_key()
    );
  }
}

#[test]
fn assets_scenario_covers_initial_resize_and_restoration() {
  let suite = include_str!("../../ditto.toml");
  let start = suite
    .find("name = \"assets\"")
    .expect("suite is missing assets");
  let following = &suite[start..];
  let block = following
    .find("\n[[scenarios]]")
    .map_or(following, |end| &following[..end]);
  for checkpoint in ["initial", "resized", "restored"] {
    assert!(
      block.contains(&format!("screenshot = {{ name = \"{checkpoint}\" }}")),
      "assets scenario is missing {checkpoint}"
    );
  }
  assert_eq!(block.matches("screenshot =").count(), 3);
}

#[test]
fn task_47_scenarios_cover_registered_stable_states() {
  let suite = include_str!("../../ditto.toml");
  for (scenario, checkpoints) in [
    ("composition", &["initial", "reordered", "restored"][..]),
    ("events and portals", &["initial", "routed", "restored"][..]),
    (
      "state and identity",
      &["initial", "changed", "reordered", "restored"][..],
    ),
  ] {
    let start = suite
      .find(&format!("name = \"{scenario}\""))
      .unwrap_or_else(|| panic!("suite is missing {scenario}"));
    let following = &suite[start..];
    let block = following
      .find("\n[[scenarios]]")
      .map_or(following, |end| &following[..end]);
    for checkpoint in checkpoints {
      assert!(
        block.contains(&format!("screenshot = {{ name = \"{checkpoint}\" }}")),
        "scenario {scenario} is missing {checkpoint}"
      );
    }
    assert_eq!(block.matches("screenshot =").count(), checkpoints.len());
  }
}

#[test]
fn task_48_scenarios_cover_registered_stable_states() {
  let suite = include_str!("../../ditto.toml");
  for (scenario, checkpoints) in [
    (
      "context and memo",
      &[
        "initial",
        "value-changed",
        "overridden",
        "memo-restored",
        "restored",
      ][..],
    ),
    (
      "effects and stores",
      &[
        "initial",
        "effect-connected",
        "store-swapped",
        "connected-store-swapped",
        "store-updated",
        "effect-disconnected",
        "restored",
      ][..],
    ),
    (
      "resources and boundaries",
      &[
        "initial",
        "boundary-error",
        "boundary-restored",
        "resource-ready",
        "resource-error",
        "resource-recovered",
        "restored",
      ][..],
    ),
    (
      "refs and geometry",
      &["initial", "unavailable", "restored"][..],
    ),
  ] {
    let start = suite
      .find(&format!("name = \"{scenario}\""))
      .unwrap_or_else(|| panic!("suite is missing {scenario}"));
    let following = &suite[start..];
    let block = following
      .find("\n[[scenarios]]")
      .map_or(following, |end| &following[..end]);
    for checkpoint in checkpoints {
      assert!(
        block.contains(&format!("screenshot = {{ name = \"{checkpoint}\" }}")),
        "scenario {scenario} is missing {checkpoint}"
      );
    }
    assert_eq!(block.matches("screenshot =").count(), checkpoints.len());
  }
}
