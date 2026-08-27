use battlement::{Box, Button, Color, Command, ParallelCommandGroup, Style, UiNode};

use crate::{CALLBACK_BUTTON_ID, GREETING_ID, PAGE_ID, TRANSIENT_CARD_ID, components};

pub(crate) fn show() -> Vec<ParallelCommandGroup<Command>> {
  let transient = UiNode::new(
    TRANSIENT_CARD_ID,
    Box::new().style(Style::new().background_color(Color::rgb(0.08, 0.2, 0.24))),
  );
  vec![
    ParallelCommandGroup::new(vec![Command::create_visual_element(PAGE_ID, transient)]),
    ParallelCommandGroup::new(vec![Command::update_visual_element(
      TRANSIENT_CARD_ID,
      Box::default()
        .name("updated-callback-result")
        .style(Style::new().background_color(Color::rgb(0.1, 0.36, 0.4))),
    )]),
    ParallelCommandGroup::new(vec![Command::update_visual_element_parent(
      TRANSIENT_CARD_ID,
      PAGE_ID,
    )]),
    ParallelCommandGroup::new(vec![Command::update_visual_element_index(
      TRANSIENT_CARD_ID,
      0,
    )]),
    ParallelCommandGroup::new(vec![Command::destroy_visual_element(TRANSIENT_CARD_ID)]),
    ParallelCommandGroup::new(vec![
      Command::create_visual_element(PAGE_ID, components::greeting(GREETING_ID)),
      Command::update_visual_element(CALLBACK_BUTTON_ID, Button::new("Hide")),
    ]),
  ]
}

pub(crate) fn hide() -> Vec<ParallelCommandGroup<Command>> {
  vec![ParallelCommandGroup::new(vec![
    Command::destroy_visual_element(GREETING_ID),
    Command::update_visual_element(
      CALLBACK_BUTTON_ID,
      Button::new("Click to run a Rust callback"),
    ),
  ])]
}
