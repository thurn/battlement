use std::{error::Error, fmt};

use battlement::{Label, VisualElement};
use battlement_reactant::prelude::*;

use crate::{Control, Game, Interaction, design_system};

pub(crate) struct ResourcesBoundaries {
  pub(crate) failed: bool,
  pub(crate) retry_revision: u32,
  pub(crate) reports: u32,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
}

struct BoundaryPrimary {
  failed: bool,
  reports: u32,
  interaction: Interaction,
  compact: bool,
}

struct BoundaryFallback {
  message: String,
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
      .child(Label::new("RESOURCES & BOUNDARIES").style(design_system::eyebrow()))
      .child(
        Label::new("Recover without losing control")
          .name("page-title")
          .style(design_system::effects_title(self.compact)),
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
        .on_error(|game: &mut Game, _| game.boundary_reports += 1)
        .child(BoundaryPrimary {
          failed: self.failed,
          reports: self.reports,
          interaction: self.interaction,
          compact: self.compact,
        }),
      )
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
        .child(Label::new("PRIMARY READY").style(design_system::boundary_status(false)))
        .child(
          Label::new(format!("REPORTS  {}", self.reports))
            .name("boundary-reports")
            .style(design_system::boundary_detail()),
        )
        .child(super::interactive_button(
          "TRIGGER ERROR",
          "boundary-action",
          design_system::boundary_action(super::control_state(
            self.interaction,
            Control::BoundaryAction,
          )),
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
      .child(Label::new("BOUNDARY CAUGHT").style(design_system::boundary_status(true)))
      .child(
        Label::new(self.message.clone())
          .name("boundary-error")
          .style(design_system::boundary_detail()),
      )
      .child(super::interactive_button(
        "RESET BOUNDARY",
        "boundary-reset",
        design_system::boundary_action(super::control_state(
          self.interaction,
          Control::BoundaryAction,
        )),
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
