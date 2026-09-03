use crate::{Control, Game, Interaction, control_state, design_system, interactive_button};
use battlement::{LengthUnits, PickingMode, ScrollViewMode, ScrollerVisibility};
use battlement_reactant::prelude::*;

#[builder]
pub(crate) struct EventsPortals {
  pub(crate) active: bool,
  pub(crate) trace: Vec<&'static str>,
  #[builder(required)]
  pub(crate) overlay: PortalTarget,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
}

impl Component for EventsPortals {
  fn render(&self) -> impl Render {
    battlement_reactant::host::ScrollView::new()
      .name("events-canvas")
      .mode(ScrollViewMode::Vertical)
      .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
      .vertical_scroller_visibility(ScrollerVisibility::Auto)
      .vertical_scroller_style(design_system::effects_scroller())
      .vertical_low_button_style(design_system::effects_scroll_button())
      .vertical_high_button_style(design_system::effects_scroll_button())
      .vertical_track_style(design_system::effects_scroll_track())
      .vertical_dragger_style(design_system::effects_scroll_dragger())
      .vertical_dragger_border_style(design_system::effects_scroll_dragger())
      .style(design_system::canvas(self.compact))
      .child(
        battlement_reactant::host::View::new()
          .name("events-content")
          .style(design_system::effects_content())
          .child(
            battlement_reactant::host::Label::new("EVENTS & PORTALS")
              .style(design_system::eyebrow()),
          )
          .child(
            battlement_reactant::host::Label::new("Follow the logical path")
              .name("events-title")
              .style(design_system::effects_title(self.compact)),
          )
          .child(
            battlement_reactant::host::View::new()
              .name("events-specimen")
              .style(design_system::event_specimen(self.compact))
              .child(
                battlement_reactant::host::View::new()
                  .name("event-source")
                  .style(design_system::event_source_card(self.compact))
                  .child(
                    battlement_reactant::host::Label::new("Logical source")
                      .style(design_system::effect_heading()),
                  )
                  .child(
                    battlement_reactant::host::View::new()
                      .style(design_system::event_status_frame(self.compact))
                      .child(self.status()),
                  )
                  .child(
                    Overlay::layer(self.overlay.clone()).child(
                      battlement_reactant::host::Stack::new()
                        .name("portal-layer")
                        .picking_mode(PickingMode::Ignore)
                        .style(Style::new().width(100.0_f32.pct()).height(100.0_f32.pct()))
                        .child(
                          battlement_reactant::host::View::new()
                            .name("portal-overlay")
                            .style(design_system::portal_card(self.compact))
                            .stack_item(design_system::portal_layer_item(self.compact))
                            .child(
                              battlement_reactant::host::Label::new("Portaled overlay")
                                .style(design_system::effect_heading()),
                            )
                            .child(self.action()),
                        ),
                    ),
                  )
                  .on_click_capture(|game: &mut Game| {
                    game.event_trace.clear();
                    game.event_trace.push("CAPTURE");
                  })
                  .on_click(|game: &mut Game| game.event_trace.push("BUBBLE")),
              )
              .child(
                battlement_reactant::host::Label::new(if self.compact { "v" } else { ">" })
                  .style(design_system::portal_connector(self.compact)),
              ),
          ),
      )
  }
}

impl EventsPortals {
  fn action(&self) -> impl Render {
    interactive_button(
      if self.active { "RESTORE" } else { "RUN EVENT" },
      "events-action",
      design_system::event_action(
        control_state(self.interaction, Control::EventsAction),
        !self.active,
      ),
      Control::EventsAction,
      |game| {
        game.event_active = !game.event_active;
        if game.event_active {
          game.event_trace.push("TARGET");
        } else {
          game.event_trace.clear();
        }
      },
    )
  }

  fn status(&self) -> Node {
    if self.active {
      Node::new(
        battlement_reactant::host::View::new()
          .name("events-status")
          .style(design_system::event_route(self.compact))
          .child(
            battlement_reactant::host::Label::new("CAPTURE").style(design_system::event_step(
              self.compact,
              self.trace.contains(&"CAPTURE"),
              0,
            )),
          )
          .child(
            battlement_reactant::host::Label::new(if self.compact { "v" } else { ">" })
              .style(design_system::event_arrow(self.compact)),
          )
          .child(
            battlement_reactant::host::Label::new("TARGET").style(design_system::event_step(
              self.compact,
              self.trace.contains(&"TARGET"),
              1,
            )),
          )
          .child(
            battlement_reactant::host::Label::new(if self.compact { "v" } else { ">" })
              .style(design_system::event_arrow(self.compact)),
          )
          .child(
            battlement_reactant::host::Label::new("BUBBLE").style(design_system::event_step(
              self.compact,
              self.trace.contains(&"BUBBLE"),
              2,
            )),
          ),
      )
    } else {
      Node::new(
        battlement_reactant::host::Label::new("READY")
          .name("events-status")
          .style(design_system::event_ready()),
      )
    }
  }
}
