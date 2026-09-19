use trox::ls;

use crate::{Control, Game, Interaction, design_system, model};
use battlement::Style;
pub(crate) fn interactive_button(
  dispatch: &model::GameDispatch,
  text: &'static str,
  name: &'static str,
  style: Style,
  control: Control,
  click: impl Fn(&mut Game) + 'static,
) -> reactant::host::ButtonHost {
  let enter = dispatch.action(move |game| game.interaction.hovered = Some(control));
  let leave = dispatch.action(move |game| {
    if game.interaction.hovered == Some(control) {
      game.interaction.hovered = None;
    }
    if game.interaction.pressed == Some(control) {
      game.interaction.pressed = None;
    }
  });
  let down = dispatch.action(move |game| game.interaction.pressed = Some(control));
  let release = dispatch.action(move |game| {
    if game.interaction.pressed == Some(control) {
      game.interaction.pressed = None;
    }
  });
  let focus = dispatch.action(move |game| game.interaction.focused = Some(control));
  let blur = dispatch.action(move |game| {
    if game.interaction.focused == Some(control) {
      game.interaction.focused = None;
    }
  });
  let activate = dispatch.action(move |game| {
    game.interaction.hovered = None;
    game.interaction.pressed = None;
    game.interaction.focused = None;
    click(game);
  });
  reactant::host::ButtonHost::new(ls(text))
    .name(name)
    .style(style)
    .on_pointer_enter(enter)
    .on_pointer_leave(leave)
    .on_pointer_down(down)
    .on_pointer_up(dispatch.action(move |game| {
      if game.interaction.pressed == Some(control) {
        game.interaction.pressed = None;
      }
    }))
    .on_pointer_cancel(dispatch.action(move |game| {
      if game.interaction.pressed == Some(control) {
        game.interaction.pressed = None;
      }
    }))
    .on_pointer_capture_out(release)
    .on_focus(focus)
    .on_blur(blur)
    .on_click(activate)
}

pub(crate) fn control_state(
  interaction: Interaction,
  control: Control,
) -> design_system::ControlState {
  if interaction.pressed == Some(control) {
    return design_system::ControlState::Pressed;
  }
  if interaction.focused == Some(control) {
    return design_system::ControlState::Focused;
  }
  if interaction.hovered == Some(control) {
    return design_system::ControlState::Hovered;
  }
  design_system::ControlState::Resting
}
