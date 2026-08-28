use battlement_reactant::prelude::*;

use crate::{Control, Game, design_system};

pub(crate) struct EffectsStores {
  pub(crate) enabled: bool,
  pub(crate) interaction: design_system::ControlState,
  pub(crate) compact: bool,
}

impl Component for EffectsStores {
  fn render(&self) -> impl Render {
    let (connected, set_connected) = use_state(false);
    let enabled = self.enabled;
    use_effect(
      move || {
        set_connected.set(enabled);
        let cleanup = set_connected.clone();
        move || cleanup.set(false)
      },
      enabled,
    );
    VisualElement::new()
      .name("effects-canvas")
      .style(design_system::canvas(self.compact))
      .child(Label::new("EFFECTS & STORES").style(design_system::eyebrow()))
      .child(
        Label::new("Synchronize after commit")
          .name("effects-title")
          .style(design_system::title()),
      )
      .child(
        VisualElement::new()
          .name("effects-specimen")
          .style(design_system::effects_specimen())
          .child(
            VisualElement::new()
              .name("effect-card")
              .style(design_system::effect_card())
              .child(Label::new("EFFECT  Connection").style(design_system::experiment_title()))
              .child(
                Label::new(if connected {
                  "CONNECTED"
                } else {
                  "DISCONNECTED"
                })
                .name("effect-status")
                .style(design_system::effect_status(connected)),
              )
              .child(crate::interactive_button(
                if self.enabled { "RESTORE" } else { "CONNECT" },
                "effects-action",
                design_system::primary_action(self.interaction),
                Control::EffectsAction,
                |game: &mut Game| game.effects_enabled = !game.effects_enabled,
              )),
          )
          .child(
            VisualElement::new()
              .name("store-card")
              .style(design_system::effect_card())
              .child(
                Label::new("STORE  External snapshot").style(design_system::experiment_title()),
              )
              .child(
                Label::new("IDLE")
                  .name("store-status")
                  .style(design_system::effect_status(false)),
              ),
          ),
      )
  }
}
