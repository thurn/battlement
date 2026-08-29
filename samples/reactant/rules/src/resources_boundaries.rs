use std::{error::Error, fmt};

use battlement::{Label, VisualElement};
use battlement_reactant::prelude::*;

use crate::{Control, Interaction, design_system};

pub(crate) struct ResourcesBoundaries {
  pub(crate) failed: bool,
  pub(crate) retry_revision: u32,
  pub(crate) preview_resource: Resource<u32, u32>,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
}

struct BoundaryPrimary {
  failed: bool,
  interaction: Interaction,
  compact: bool,
}

struct BoundaryFallback {
  message: String,
  interaction: Interaction,
  compact: bool,
}

struct ResourcePreview {
  resource: Resource<u32, u32>,
  interaction: Interaction,
  compact: bool,
}

#[derive(Debug)]
struct BoundaryFailure;

impl Component for ResourcesBoundaries {
  fn render(&self) -> impl Render {
    let compact = self.compact;
    VisualElement::new()
      .name("resources-canvas")
      .style(design_system::canvas(self.compact))
      .child(
        Label::new("RESOURCES & BOUNDARIES").style(design_system::resources_eyebrow(self.compact)),
      )
      .child(
        Label::new("Recover without losing control")
          .name("page-title")
          .style(design_system::effects_title(self.compact)),
      )
      .child(
        VisualElement::new()
          .name("resources-card-group")
          .style(design_system::resources_group(self.compact))
          .child(
            Suspense::new(
              VisualElement::new()
                .name("resource-pending")
                .style(design_system::boundary_card(false, self.compact))
                .child(
                  Label::new("RESOURCE PENDING")
                    .style(design_system::boundary_status(false, self.compact)),
                )
                .child(super::interactive_button(
                  "RESOLVE RESOURCE",
                  "resource-resolve",
                  design_system::boundary_action(
                    super::control_state(self.interaction, Control::ResourceAction),
                    true,
                    self.compact,
                  ),
                  Control::ResourceAction,
                  |game| game.resource_resolution_requested = true,
                )),
            )
            .child(ResourcePreview {
              resource: self.preview_resource.clone(),
              interaction: self.interaction,
              compact: self.compact,
            }),
          )
          .child(
            ErrorBoundary::new({
              let interaction = self.interaction;
              move |error: &RenderError| BoundaryFallback {
                message: error.to_string(),
                interaction,
                compact,
              }
            })
            .reset_on(self.retry_revision)
            .child(BoundaryPrimary {
              failed: self.failed,
              interaction: self.interaction,
              compact: self.compact,
            }),
          ),
      )
  }
}

impl Component for ResourcePreview {
  fn render(&self) -> impl Render {
    let compact = self.compact;
    let interaction = self.interaction;
    use_resource(&self.resource, 1).then(move |_| {
      VisualElement::new()
        .name("resource-ready")
        .style(design_system::boundary_card(false, compact))
        .child(Label::new("RESOURCE READY").style(design_system::boundary_status(false, compact)))
        .child(super::interactive_button(
          "REFETCH RESOURCE",
          "resource-refetch",
          design_system::boundary_action(
            super::control_state(interaction, Control::ResourceAction),
            false,
            compact,
          ),
          Control::ResourceAction,
          |game| game.resource_invalidation_requested = true,
        ))
    })
  }
}

impl Component for BoundaryPrimary {
  fn render(&self) -> impl Render {
    if self.failed {
      return Err(BoundaryFailure);
    }
    Ok(
      VisualElement::new()
        .name("boundary-primary")
        .style(design_system::boundary_card(false, self.compact))
        .child(
          Label::new("BOUNDARY READY").style(design_system::boundary_status(false, self.compact)),
        )
        .child(super::interactive_button(
          "TRIGGER ERROR",
          "boundary-action",
          design_system::boundary_action(
            super::control_state(self.interaction, Control::BoundaryAction),
            false,
            self.compact,
          ),
          Control::BoundaryAction,
          |game| game.boundary_failed = true,
        )),
    )
  }
}

impl Component for BoundaryFallback {
  fn render(&self) -> impl Render {
    VisualElement::new()
      .name("boundary-fallback")
      .style(design_system::boundary_card(true, self.compact))
      .child(Label::new("ERROR CAUGHT").style(design_system::boundary_status(true, self.compact)))
      .child(
        Label::new(self.message.clone())
          .name("boundary-error")
          .style(design_system::boundary_detail()),
      )
      .child(super::interactive_button(
        "RESET BOUNDARY",
        "boundary-reset",
        design_system::boundary_action(
          super::control_state(self.interaction, Control::BoundaryAction),
          true,
          self.compact,
        ),
        Control::BoundaryAction,
        |game| {
          game.boundary_failed = false;
          game.boundary_retry_revision += 1;
        },
      ))
  }
}

impl fmt::Display for BoundaryFailure {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("resource preview failed")
  }
}

impl Error for BoundaryFailure {}
