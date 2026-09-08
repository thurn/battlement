use battlement::{ActionId, Batch, BatchId, Command, ParallelCommandGroup, Response, SessionId};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Page {
  Components,
  Interactions,
  Hierarchy,
  Assets,
  Layout,
  Appearance,
  Backgrounds,
  Transforms,
  Typography,
  Buttons,
  Containers,
  Scroll,
  Tabs,
  TextFields,
  BooleanControls,
  ChoiceGroups,
  Dropdowns,
  Sliders,
  Ranges,
  Parts,
  ComplexParts,
  PointerRouting,
  KeyboardNavigation,
  RemainingEvents,
  Actions,
  RenderModes,
  WorldSpace,
  Coverage,
}

pub(crate) fn single_ui_command_response(
  session_id: SessionId,
  action_id: ActionId,
  commands: Vec<Command>,
) -> Response<Command> {
  Response::batch(
    Batch::new(
      BatchId::new_v4(),
      session_id,
      vec![ParallelCommandGroup::new(commands)],
    )
    .caused_by_action_id(action_id),
  )
}

#[cfg(test)]
mod tests {
  use crate::DITTO_VISUAL_STATE_REGISTRY;

  #[test]
  fn deterministic_page_inventory_matches_the_ditto_registry() {
    assert_eq!(
      DITTO_VISUAL_STATE_REGISTRY.matches("[[states]]").count(),
      24
    );
    for screen in [
      "interactions",
      "hierarchy",
      "assets",
      "layout",
      "appearance",
      "backgrounds",
    ] {
      for state in ["initial", "changed", "restored"] {
        assert!(
          DITTO_VISUAL_STATE_REGISTRY.contains(&format!("key = \"{screen}.{state}\"")),
          "registry is missing {screen}.{state}"
        );
      }
    }
  }

  #[test]
  fn foundation_scenarios_cover_the_registered_stable_states() {
    let suite = include_str!("../../ditto.toml");
    for (scenario, checkpoints) in [
      ("components foundation", &["initial"][..]),
      (
        "interactions round trip",
        &["initial", "changed", "restored"][..],
      ),
      (
        "hierarchy round trip",
        &["initial", "changed", "restored"][..],
      ),
      (
        "asset source round trip",
        &["initial", "changed", "restored"][..],
      ),
      ("layout round trip", &["initial", "changed", "restored"][..]),
      (
        "appearance round trip",
        &["initial", "changed", "restored"][..],
      ),
      (
        "background round trip",
        &["initial", "changed", "restored"][..],
      ),
      ("typography foundation", &["initial"][..]),
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
  fn control_scenarios_cover_the_registered_stable_states() {
    let suite = include_str!("../../ditto.toml");
    for (scenario, checkpoints) in [
      ("native parts", &["initial"][..]),
      ("complex parts", &["initial", "changed", "restored"][..]),
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
}
