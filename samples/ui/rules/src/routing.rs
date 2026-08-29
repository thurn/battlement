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

#[cfg(test)]
impl Page {
  pub(crate) const ALL: [Self; 28] = [
    Self::Components,
    Self::Interactions,
    Self::Hierarchy,
    Self::Assets,
    Self::Layout,
    Self::Appearance,
    Self::Backgrounds,
    Self::Transforms,
    Self::Typography,
    Self::Buttons,
    Self::Containers,
    Self::Scroll,
    Self::Tabs,
    Self::TextFields,
    Self::BooleanControls,
    Self::ChoiceGroups,
    Self::Dropdowns,
    Self::Sliders,
    Self::Ranges,
    Self::Parts,
    Self::ComplexParts,
    Self::PointerRouting,
    Self::KeyboardNavigation,
    Self::RemainingEvents,
    Self::Actions,
    Self::RenderModes,
    Self::WorldSpace,
    Self::Coverage,
  ];

  pub(crate) const fn registry_key(self) -> &'static str {
    match self {
      Self::Components => "components",
      Self::Interactions => "interactions",
      Self::Hierarchy => "hierarchy",
      Self::Assets => "assets",
      Self::Layout => "layout",
      Self::Appearance => "appearance",
      Self::Backgrounds => "backgrounds",
      Self::Transforms => "transforms",
      Self::Typography => "typography",
      Self::Buttons => "buttons",
      Self::Containers => "containers",
      Self::Scroll => "scroll",
      Self::Tabs => "tabs",
      Self::TextFields => "text-fields",
      Self::BooleanControls => "boolean-controls",
      Self::ChoiceGroups => "choice-groups",
      Self::Dropdowns => "dropdowns",
      Self::Sliders => "sliders",
      Self::Ranges => "ranges",
      Self::Parts => "parts",
      Self::ComplexParts => "complex-parts",
      Self::PointerRouting => "pointer-routing",
      Self::KeyboardNavigation => "keyboard-navigation",
      Self::RemainingEvents => "remaining-events",
      Self::Actions => "actions",
      Self::RenderModes => "render-modes",
      Self::WorldSpace => "world-space",
      Self::Coverage => "coverage",
    }
  }
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
  use crate::{DITTO_VISUAL_STATE_REGISTRY, routing::Page};

  #[test]
  fn page_inventory_matches_the_ditto_registry() {
    for page in Page::ALL {
      assert!(
        DITTO_VISUAL_STATE_REGISTRY.contains(&format!("screen = \"{}\"", page.registry_key()))
      );
    }
    assert_eq!(
      DITTO_VISUAL_STATE_REGISTRY.matches("[[states]]").count(),
      65
    );
    for screen in [
      "interactions",
      "hierarchy",
      "assets",
      "layout",
      "appearance",
      "backgrounds",
      "transforms",
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
      (
        "transform round trip",
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
      (
        "buttons controls",
        &["initial", "pointer", "keyboard", "repeat"][..],
      ),
      (
        "containers controls",
        &["initial", "changed", "restored"][..],
      ),
      ("scroll controls", &["initial", "changed"][..]),
      ("tabs controls", &["initial", "selected"][..]),
      (
        "text field controls",
        &[
          "initial",
          "accepted-focus",
          "normalized-focus",
          "rejected-focus",
        ][..],
      ),
      (
        "boolean controls",
        &[
          "initial",
          "accepted",
          "restored",
          "toggle-rejected",
          "radios",
        ][..],
      ),
      ("choice group controls", &["initial", "changed"][..]),
      (
        "dropdown controls",
        &["initial", "accepted", "rejected", "cleared"][..],
      ),
      ("slider controls", &["initial", "continuous", "stepped"][..]),
      ("range controls", &["initial", "changed"][..]),
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
