use battlement_reactant::prelude::*;

use crate::{Game, design_system};

pub(crate) struct StateIdentity;

struct IdentityToken {
  id: u8,
  pulse: u8,
  reset: u8,
  name: &'static str,
}

impl Component for StateIdentity {
  fn render(&self) -> impl Render {
    let (value, set_value) = use_state(0_u8);
    let (reversed, set_reversed) = use_state(false);
    let (reset, set_reset) = use_state(0_u8);
    let (control, set_control) = use_state(design_system::ControlState::Resting);
    let action = match (value, reversed) {
      (0, _) => "QUEUE +3",
      (_, false) => "REORDER",
      (_, true) => "RESTORE",
    };
    let click_value = set_value.clone();
    let click_reversed = set_reversed.clone();
    let click_reset = set_reset.clone();
    let mut tokens = vec![
      IdentityToken {
        id: 1,
        pulse: value,
        reset,
        name: "ALPHA",
      }
      .key(1_u8),
      IdentityToken {
        id: 2,
        pulse: value,
        reset,
        name: "BRAVO",
      }
      .key(2_u8),
      IdentityToken {
        id: 3,
        pulse: value,
        reset,
        name: "CHARLIE",
      }
      .key(3_u8),
    ];
    if reversed {
      tokens.reverse();
    }
    VisualElement::new()
      .name("state-canvas")
      .style(design_system::canvas())
      .child(Label::new("STATE & IDENTITY").style(design_system::eyebrow()))
      .child(
        Label::new("State follows identity")
          .name("state-title")
          .style(design_system::title()),
      )
      .child(
        Button::new(action)
          .name("state-action")
          .style(design_system::primary_action(control))
          .on_pointer_enter({
            let setter = set_control.clone();
            move |_game: &mut Game| setter.set(design_system::ControlState::Hovered)
          })
          .on_pointer_leave({
            let setter = set_control.clone();
            move |_game: &mut Game| setter.set(design_system::ControlState::Resting)
          })
          .on_pointer_down({
            let setter = set_control.clone();
            move |_game: &mut Game| setter.set(design_system::ControlState::Pressed)
          })
          .on_pointer_up({
            let setter = set_control.clone();
            move |_game: &mut Game| setter.set(design_system::ControlState::Hovered)
          })
          .on_focus({
            let setter = set_control.clone();
            move |_game: &mut Game| setter.set(design_system::ControlState::Focused)
          })
          .on_blur(move |_game: &mut Game| set_control.set(design_system::ControlState::Resting))
          .on_click(move |_game: &mut Game| match (value, reversed) {
            (0, _) => {
              click_value.update(|current| current + 1);
              click_value.update(|current| current + 1);
              click_value.update(|current| current + 1);
            }
            (_, false) => click_reversed.set(true),
            (_, true) => {
              click_value.set(0);
              click_reversed.set(false);
              click_reset.update(|current| current.wrapping_add(1));
            }
          }),
      )
      .child(
        VisualElement::new()
          .name("state-specimen")
          .style(design_system::specimen())
          .child(
            Label::new(format!("BATCHED VALUE  {value}"))
              .name("state-value")
              .style(design_system::state_value()),
          )
          .child(
            VisualElement::new()
              .name("identity-tokens")
              .style(design_system::identity_row())
              .child(Fragment::new(tokens)),
          ),
      )
  }
}

impl Component for IdentityToken {
  fn render(&self) -> impl Render {
    let (seen_pulse, set_seen_pulse) = use_state(self.pulse);
    let (seen_reset, set_seen_reset) = use_state(self.reset);
    let (revision, set_revision) = use_state(0_u8);
    if seen_reset != self.reset {
      set_seen_reset.set(self.reset);
      set_seen_pulse.set(self.pulse);
      set_revision.set(0);
    } else if seen_pulse != self.pulse {
      set_seen_pulse.set(self.pulse);
      set_revision.update(|current| current + 1);
    }
    VisualElement::new()
      .name(format!("identity-token-{}", self.id))
      .style(design_system::identity_token())
      .child(Label::new(format!("0{}  {}", self.id, self.name)))
      .child(
        Label::new(format!("STATE {revision}"))
          .name("identity-state")
          .style(design_system::identity_state()),
      )
  }
}
