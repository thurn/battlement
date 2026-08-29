use battlement::{Label, TextField, VisualElement};
use battlement_reactant::prelude::*;

use crate::{Control, Game, Interaction, design_system};

pub(crate) struct RefsGeometry {
  pub(crate) active: bool,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
}

impl Component for RefsGeometry {
  fn render(&self) -> impl Render {
    let field_ref = use_element_ref();
    let action_ref = field_ref.clone();
    let action_button_ref = use_element_ref();
    let restore_ref = action_button_ref.clone();
    let active = self.active;
    VisualElement::new()
      .name("refs-canvas")
      .style(design_system::canvas(self.compact))
      .child(Label::new("REFS & GEOMETRY").style(design_system::resources_eyebrow(self.compact)))
      .child(
        Label::new("Act on committed hosts")
          .name("refs-title")
          .style(design_system::effects_title(self.compact)),
      )
      .child(
        VisualElement::new()
          .name("refs-card")
          .style(design_system::refs_card(self.compact))
          .child(
            Label::new(if active {
              "FOCUS & SELECTION ACTIVE"
            } else {
              "HOST READY"
            })
            .name("refs-status")
            .style(design_system::refs_status(active, self.compact)),
          )
          .child(
            TextField::new()
              .name("refs-field")
              .value("Stable reference")
              .style(design_system::refs_field(active, self.compact))
              .element_ref(field_ref),
          )
          .child(
            super::interactive_button(
              if active { "RESTORE" } else { "FOCUS & SELECT" },
              "refs-action",
              design_system::boundary_action(
                super::control_state(self.interaction, Control::RefsAction),
                !active,
                self.compact,
              ),
              Control::RefsAction,
              move |game: &mut Game| {
                if active {
                  restore_ref.focus();
                  action_ref.select_text(0, 0);
                } else {
                  action_ref.focus();
                  action_ref.select_text(16, 0);
                }
                game.refs_active = !active;
              },
            )
            .element_ref(action_button_ref),
          ),
      )
  }
}
