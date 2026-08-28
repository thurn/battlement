use battlement::{ScrollViewMode, ScrollerVisibility};
use battlement_reactant::prelude::*;

use crate::{Control, Game, Interaction, control_state, design_system, interactive_button};

pub(crate) struct EventsPortals {
  pub(crate) active: bool,
  pub(crate) trace: Vec<&'static str>,
  pub(crate) overlay: PortalTarget,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
}

impl Component for EventsPortals {
  fn render(&self) -> impl Render {
    ScrollView::new()
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
        VisualElement::new()
          .name("events-content")
          .style(design_system::effects_content())
          .child(Label::new("EVENTS & PORTALS").style(design_system::eyebrow()))
          .child(
            Label::new("Follow the logical path")
              .name("events-title")
              .style(design_system::effects_title(self.compact)),
          )
          .child(
            VisualElement::new()
              .name("events-specimen")
              .style(design_system::event_specimen(self.compact))
              .child(
                VisualElement::new()
                  .name("event-source")
                  .style(design_system::event_source_card(self.compact))
                  .child(Label::new("Logical source").style(design_system::effect_heading()))
                  .child(
                    VisualElement::new()
                      .style(design_system::event_status_frame(self.compact))
                      .child(self.status()),
                  )
                  .child(create_portal(self.action(), self.overlay.clone()))
                  .on_click_capture(|game: &mut Game| {
                    game.event_trace.clear();
                    game.event_trace.push("CAPTURE");
                  })
                  .on_click(|game: &mut Game| game.event_trace.push("BUBBLE")),
              )
              .child(
                Label::new(if self.compact { "v" } else { ">" })
                  .style(design_system::portal_connector(self.compact)),
              )
              .child(
                VisualElement::new()
                  .name("portal-overlay")
                  .style(design_system::portal_card(self.compact))
                  .child(Label::new("Portaled overlay").style(design_system::effect_heading()))
                  .portal_target(self.overlay.clone()),
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
        VisualElement::new()
          .name("events-status")
          .style(design_system::event_route(self.compact))
          .child(Label::new("CAPTURE").style(design_system::event_step(
            self.compact,
            self.trace.contains(&"CAPTURE"),
            0,
          )))
          .child(
            Label::new(if self.compact { "v" } else { ">" })
              .style(design_system::event_arrow(self.compact)),
          )
          .child(Label::new("TARGET").style(design_system::event_step(
            self.compact,
            self.trace.contains(&"TARGET"),
            1,
          )))
          .child(
            Label::new(if self.compact { "v" } else { ">" })
              .style(design_system::event_arrow(self.compact)),
          )
          .child(Label::new("BUBBLE").style(design_system::event_step(
            self.compact,
            self.trace.contains(&"BUBBLE"),
            2,
          ))),
      )
    } else {
      Node::new(
        Label::new("READY")
          .name("events-status")
          .style(design_system::event_ready()),
      )
    }
  }
}
