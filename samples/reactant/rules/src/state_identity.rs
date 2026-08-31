use battlement_reactant::{hooks, prelude::*};

use crate::{Game, design_system};

pub(crate) struct StateIdentity {
  pub(crate) compact: bool,
}

struct IdentityToken {
  id: u8,
  position: f32,
  pulse: u8,
  name: &'static str,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct IdentityState {
  pulse: u8,
  revision: u8,
}

#[derive(Clone, Copy)]
enum IdentityAction {
  Observe(u8),
}

impl Component for StateIdentity {
  fn render(&self) -> impl Render {
    let (value, set_value) = hooks::use_state(0_u8);
    let (reversed, set_reversed) = hooks::use_state(false);
    let (reset, set_reset) = hooks::use_state(0_u8);
    let (control, set_control) = hooks::use_state(design_system::ControlState::Resting);
    let action = match (value, reversed) {
      (0, _) => "QUEUE +3",
      (_, false) => "REORDER",
      (_, true) => "RESTORE",
    };
    let click_value = set_value.clone();
    let click_reversed = set_reversed.clone();
    let click_reset = set_reset.clone();
    let click_control = set_control.clone();
    let mut identities = [(1_u8, "ALPHA"), (2_u8, "BRAVO"), (3_u8, "CHARLIE")];
    if reversed {
      identities.reverse();
    }
    let tokens = identities
      .into_iter()
      .enumerate()
      .map(|(position, (id, name))| {
        IdentityToken {
          id,
          position: position as f32,
          pulse: value,
          name,
        }
        .key((reset, id))
      })
      .collect::<Vec<_>>();
    battlement_reactant::host::View::new()
      .name("state-canvas")
      .style(design_system::canvas(self.compact))
      .child(
        battlement_reactant::host::Label::new("State follows identity")
          .name("state-title")
          .style(design_system::title()),
      )
      .child(
        battlement_reactant::host::Button::new(action)
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
          .on_pointer_cancel({
            let setter = set_control.clone();
            move |_game: &mut Game| setter.set(design_system::ControlState::Resting)
          })
          .on_pointer_capture_out({
            let setter = set_control.clone();
            move |_game: &mut Game| setter.set(design_system::ControlState::Resting)
          })
          .on_focus({
            let setter = set_control.clone();
            move |_game: &mut Game| setter.set(design_system::ControlState::Focused)
          })
          .on_blur(move |_game: &mut Game| set_control.set(design_system::ControlState::Resting))
          .on_click(move |_game: &mut Game| {
            click_control.set(design_system::ControlState::Resting);
            match (value, reversed) {
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
            }
          }),
      )
      .child(
        battlement_reactant::host::View::new()
          .name("state-specimen")
          .style(design_system::state_specimen())
          .child(
            battlement_reactant::host::Label::new(format!("BATCHED VALUE  {value}"))
              .name("state-value")
              .style(design_system::state_value()),
          )
          .child(
            battlement_reactant::host::View::new()
              .name("identity-tokens")
              .style(design_system::identity_row())
              .child(Fragment::new(tokens)),
          ),
      )
  }
}

impl Component for IdentityToken {
  fn render(&self) -> impl Render {
    let (state, dispatch) = hooks::use_reducer(
      self::reduce_identity,
      IdentityState {
        pulse: self.pulse,
        revision: 0,
      },
    );
    if state.pulse != self.pulse {
      dispatch.send(IdentityAction::Observe(self.pulse));
    }
    battlement_reactant::host::View::new()
      .name(format!("identity-token-{}", self.id))
      .style(design_system::identity_token(
        self.position,
        state.revision > 0,
      ))
      .child(battlement_reactant::host::Label::new(format!(
        "0{}  {}",
        self.id, self.name
      )))
      .child(
        battlement_reactant::host::Label::new(format!("REDUCER {}", state.revision))
          .name("identity-state")
          .style(design_system::identity_state(state.revision > 0)),
      )
  }
}

fn reduce_identity(state: &IdentityState, action: IdentityAction) -> IdentityState {
  match action {
    IdentityAction::Observe(pulse) => IdentityState {
      pulse,
      revision: state.revision + 1,
    },
  }
}
