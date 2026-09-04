use trox::{assert_localized, tx};

use crate::{Control, Interaction, design_system, preview_resource::Preview};
use battlement_reactant::prelude::*;
use std::{error::Error, fmt};

#[builder]
pub(crate) struct ResourcesBoundaries {
  pub(crate) failed: bool,
  pub(crate) retry_revision: u32,
  #[builder(required)]
  pub(crate) preview_resource: Preview,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
}

#[builder]
struct BoundaryPrimary {
  failed: bool,
  interaction: Interaction,
  compact: bool,
}

#[builder]
struct BoundaryFallback {
  message: String,
  interaction: Interaction,
  compact: bool,
}

#[builder]
struct ResourcePreview {
  #[builder(required)]
  resource: Preview,
  interaction: Interaction,
  compact: bool,
}

#[derive(Debug)]
struct BoundaryFailure;

impl Component for ResourcesBoundaries {
  fn render(&self) -> impl Render {
    let compact = self.compact;
    let preview = self.preview_resource.clone();
    battlement_reactant::host::View::new()
      .name("resources-canvas")
      .style(design_system::canvas(self.compact))
      .child(
        battlement_reactant::host::Label::new(tx(
          "RESOURCES & BOUNDARIES",
          "User-facing product copy in the Reactant sample.",
        ))
        .style(design_system::resources_eyebrow(self.compact)),
      )
      .child(
        battlement_reactant::host::Label::new(tx(
          "Recover without losing control",
          "User-facing product copy in the Reactant sample.",
        ))
        .name("page-title")
        .style(design_system::effects_title(self.compact)),
      )
      .child(
        battlement_reactant::host::View::new()
          .name("resources-card-group")
          .style(design_system::resources_group(self.compact))
          .child(
            Suspense::new(
              battlement_reactant::host::View::new()
                .name("resource-pending")
                .style(design_system::boundary_card(false, self.compact))
                .child(
                  battlement_reactant::host::Label::new(tx(
                    "RESOURCE PENDING",
                    "User-facing product copy in the Reactant sample.",
                  ))
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
                  move |_| preview.resolve(),
                )),
            )
            .child(
              ResourcePreview::new()
                .resource(self.preview_resource.clone())
                .interaction(self.interaction)
                .compact(self.compact),
            ),
          )
          .child(
            ErrorBoundary::new({
              let interaction = self.interaction;
              move |error: &RenderError| {
                BoundaryFallback::new()
                  .message(error.to_string())
                  .interaction(interaction)
                  .compact(compact)
              }
            })
            .reset_on(self.retry_revision)
            .child(
              BoundaryPrimary::new()
                .failed(self.failed)
                .interaction(self.interaction)
                .compact(self.compact),
            ),
          ),
      )
  }
}

impl Component for ResourcePreview {
  fn render(&self) -> impl Render {
    let compact = self.compact;
    let interaction = self.interaction;
    let control = use_resource_control(&self.resource.resource);
    use_resource(&self.resource.resource, 1).then(move |_| {
      battlement_reactant::host::View::new()
        .name("resource-ready")
        .style(design_system::boundary_card(false, compact))
        .child(
          battlement_reactant::host::Label::new(tx(
            "RESOURCE READY",
            "User-facing product copy in the Reactant sample.",
          ))
          .style(design_system::boundary_status(false, compact)),
        )
        .child(super::interactive_button(
          "REFETCH RESOURCE",
          "resource-refetch",
          design_system::boundary_action(
            super::control_state(interaction, Control::ResourceAction),
            false,
            compact,
          ),
          Control::ResourceAction,
          move |_| control.invalidate(1),
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
      battlement_reactant::host::View::new()
        .name("boundary-primary")
        .style(design_system::boundary_card(false, self.compact))
        .child(
          battlement_reactant::host::Label::new(tx(
            "BOUNDARY READY",
            "User-facing product copy in the Reactant sample.",
          ))
          .style(design_system::boundary_status(false, self.compact)),
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
    battlement_reactant::host::View::new()
      .name("boundary-fallback")
      .style(design_system::boundary_card(true, self.compact))
      .child(
        battlement_reactant::host::Label::new(tx(
          "ERROR CAUGHT",
          "User-facing product copy in the Reactant sample.",
        ))
        .style(design_system::boundary_status(true, self.compact)),
      )
      .child(
        battlement_reactant::host::Label::new(assert_localized(self.message.clone()))
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
