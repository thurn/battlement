use battlement::{Command, ParallelCommandGroup, PickingMode, UiBox, UiButton, UiLabel};

use crate::{
  HIERARCHY_ACTION_ID, HIERARCHY_BRANCH_ID, HIERARCHY_DESTINATION_ID, HIERARCHY_MOVABLE_ID,
  HIERARCHY_PRIMARY_ID, HIERARCHY_SECONDARY_ID, components,
};

pub(crate) fn apply() -> Vec<ParallelCommandGroup<Command>> {
  vec![
    ParallelCommandGroup::new(vec![Command::update_visual_element_index(
      HIERARCHY_SECONDARY_ID,
      0,
    )]),
    ParallelCommandGroup::new(vec![Command::update_visual_element(
      HIERARCHY_PRIMARY_ID,
      UiLabel::default()
        .enabled(false)
        .picking_mode(PickingMode::Ignore)
        .class("changed"),
    )]),
    ParallelCommandGroup::new(vec![Command::update_visual_element_parent(
      HIERARCHY_MOVABLE_ID,
      HIERARCHY_DESTINATION_ID,
    )]),
    ParallelCommandGroup::new(vec![
      Command::update_visual_element(HIERARCHY_BRANCH_ID, UiBox::default().delegates_focus(false)),
      Command::update_visual_element(HIERARCHY_ACTION_ID, UiButton::new("Reset")),
    ]),
  ]
}

pub(crate) fn reset() -> Vec<ParallelCommandGroup<Command>> {
  vec![
    ParallelCommandGroup::new(vec![Command::update_visual_element_parent(
      HIERARCHY_MOVABLE_ID,
      HIERARCHY_BRANCH_ID,
    )]),
    ParallelCommandGroup::new(vec![Command::update_visual_element_index(
      HIERARCHY_PRIMARY_ID,
      0,
    )]),
    ParallelCommandGroup::new(vec![Command::update_visual_element(
      HIERARCHY_PRIMARY_ID,
      UiLabel::default()
        .enabled(true)
        .picking_mode(PickingMode::Position)
        .class("ready"),
    )]),
    ParallelCommandGroup::new(vec![
      Command::update_visual_element(HIERARCHY_BRANCH_ID, UiBox::default().delegates_focus(true)),
      Command::update_visual_element(HIERARCHY_ACTION_ID, UiButton::new("Reorder children")),
    ]),
  ]
}

pub(crate) fn ids() -> components::HierarchyIds {
  components::HierarchyIds {
    branch: HIERARCHY_BRANCH_ID,
    primary: HIERARCHY_PRIMARY_ID,
    secondary: HIERARCHY_SECONDARY_ID,
    movable: HIERARCHY_MOVABLE_ID,
    destination: HIERARCHY_DESTINATION_ID,
    action: HIERARCHY_ACTION_ID,
  }
}
