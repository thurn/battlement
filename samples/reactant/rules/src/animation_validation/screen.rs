use battlement::{Align, Color, FlexDirection, FlexWrap, LengthUnits, Style};
use battlement_reactant::prelude::*;

use crate::animation_validation::{
  CaseId, FixtureAction, FixtureSession, ReducedMotionOverride, ValidationReport, fixture_registry,
  run_fixture_case,
};
use crate::{Game, animation_validation::runner, design_system};

const PASSING_CASE: CaseId = CaseId("static-presentation");
const FAILING_CASE: CaseId = CaseId("wrong-expectation");

/// Interactive sample state for the shared animation validation strip.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ValidationUiState {
  selected_case: CaseId,
  session: FixtureSession,
  report: Option<ValidationReport>,
  show_json: bool,
}

impl Default for ValidationUiState {
  fn default() -> Self {
    Self {
      selected_case: PASSING_CASE,
      session: FixtureSession::default(),
      report: None,
      show_json: false,
    }
  }
}

impl ValidationUiState {
  pub(crate) fn reset(&mut self) {
    self.session.reset();
    self.report = None;
    self.show_json = false;
  }

  pub(crate) fn select_next(&mut self) {
    self.selected_case = if self.selected_case == PASSING_CASE {
      FAILING_CASE
    } else {
      PASSING_CASE
    };
    self.reset();
  }

  pub(crate) fn seek_checkpoint(&mut self) {
    let registry = fixture_registry();
    let case = registry
      .select(
        crate::animation_validation::ScreenId("validation-infrastructure"),
        self.selected_case,
      )
      .expect("sample validation case should exist");
    self.session.seek(case.checkpoints[0].elapsed_micros);
  }

  pub(crate) fn capture(&mut self) {
    let registry = fixture_registry();
    let case = registry
      .select(
        crate::animation_validation::ScreenId("validation-infrastructure"),
        self.selected_case,
      )
      .expect("sample validation case should exist");
    self.report = Some(run_fixture_case(
      case,
      runner::fixture_metadata(),
      runner::fixture_observation,
    ));
  }

  pub(crate) fn toggle_export(&mut self) {
    self.show_json = !self.show_json;
  }

  pub(crate) fn dispatch(&mut self, action: FixtureAction) {
    self.session.dispatch(action);
  }

  pub(crate) fn session(&self) -> &FixtureSession {
    &self.session
  }
}

pub(crate) struct ValidationScreen {
  pub(crate) state: ValidationUiState,
  pub(crate) compact: bool,
}

impl Component for ValidationScreen {
  fn render(&self) -> impl Render {
    let report_text = self.state.report.as_ref().map_or_else(
      || "No checkpoint captured".to_owned(),
      ValidationReport::concise,
    );
    let details = self
      .state
      .report
      .as_ref()
      .map_or_else(String::new, |report| {
        if self.state.show_json {
          report.json()
        } else {
          report
            .checkpoints
            .iter()
            .flat_map(|checkpoint| checkpoint.failures.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
        }
      });
    battlement_reactant::host::ScrollView::new()
      .name("animation-validation-canvas")
      .style(canvas(self.compact))
      .content_container_style(content())
      .child(battlement_reactant::host::Label::new("ANIMATION VALIDATION").style(eyebrow()))
      .child(
        battlement_reactant::host::Label::new("Deterministic evidence strip")
          .name("page-title")
          .style(title()),
      )
      .child(
        battlement_reactant::host::Label::new(format!(
          "Case: validation-infrastructure/{} · t={}µs · generation={} · reconnects={} · actions={}",
          self.state.selected_case.0,
          self.state.session.elapsed_micros(),
          self.state.session.generation(),
          self.state.session.reconnects(),
          self.state.session.actions().len(),
        ))
        .name("validation-selection")
        .style(status()),
      )
      .child(
        battlement_reactant::host::View::new()
          .style(control_row())
          .child(action("SELECT CASE", "validation-select", |game| {
            game.animation_validation.select_next();
          }))
          .child(action("RESET", "validation-reset", |game| {
            game.animation_validation.reset();
          }))
          .child(action("SEEK", "validation-seek", |game| {
            game.animation_validation.seek_checkpoint();
          }))
          .child(action("CAPTURE", "validation-capture", |game| {
            game.animation_validation.capture();
          }))
          .child(action("EXPORT", "validation-export", |game| {
            game.animation_validation.toggle_export();
          })),
      )
      .child(
        battlement_reactant::host::View::new()
          .style(control_row())
          .child(action("TRIGGER", "validation-trigger", |game| {
            game
              .animation_validation
              .dispatch(FixtureAction::Trigger);
          }))
          .child(action("PLAY", "validation-play", |game| {
            game.animation_validation.dispatch(FixtureAction::Play);
          }))
          .child(action("PAUSE", "validation-pause", |game| {
            game.animation_validation.dispatch(FixtureAction::Pause);
          }))
          .child(action("REPLAY", "validation-replay", |game| {
            game.animation_validation.dispatch(FixtureAction::Replay);
          }))
          .child(action("SPEED", "validation-speed", |game| {
            let speed = if game.animation_validation.session().speed() == 1.0 {
              2.0
            } else {
              1.0
            };
            game
              .animation_validation
              .dispatch(FixtureAction::Speed(speed));
          }))
          .child(action("REDUCE", "validation-reduced", |game| {
            let value = match game.animation_validation.session().reduced_motion() {
              ReducedMotionOverride::System => ReducedMotionOverride::Always,
              ReducedMotionOverride::Always => ReducedMotionOverride::Never,
              ReducedMotionOverride::Never => ReducedMotionOverride::System,
            };
            game
              .animation_validation
              .dispatch(FixtureAction::ReducedMotion(value));
          }))
          .child(action("RECONNECT", "validation-reconnect", |game| {
            game
              .animation_validation
              .dispatch(FixtureAction::Reconnect);
          })),
      )
      .child(
        battlement_reactant::host::Label::new(format!(
          "{} · {} · {:.1}x · {:?}",
          report_text,
          if self.state.session.playing() {
            "playing"
          } else {
            "paused"
          },
          self.state.session.speed(),
          self.state.session.reduced_motion(),
        ))
        .name("validation-result")
        .style(result(self.state.report.as_ref().is_some_and(ValidationReport::passed))),
      )
      .child(
        battlement_reactant::host::Label::new(details)
          .name("validation-details")
          .style(details_style(self.compact)),
      )
  }
}

fn action(
  text: &'static str,
  name: &'static str,
  callback: impl Fn(&mut Game) + 'static,
) -> Button {
  battlement_reactant::host::Button::new(text)
    .name(name)
    .style(action_style())
    .on_click(callback)
}

fn canvas(compact: bool) -> Style {
  design_system::canvas(compact).padding(0.0)
}

fn content() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .padding(28.0)
    .align_items(Align::FlexStart)
}

fn eyebrow() -> Style {
  Style::new()
    .font_size(20.0)
    .color(Color::rgb(0.98, 0.4, 0.16))
}

fn title() -> Style {
  Style::new()
    .font_size(40.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
    .margin((6, 0, 12, 0))
}

fn status() -> Style {
  Style::new()
    .font_size(20.0)
    .color(Color::rgb(0.68, 0.76, 0.78))
    .margin((4, 0))
}

fn control_row() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin((8, 0))
}

fn action_style() -> Style {
  Style::new()
    .height(42.0)
    .min_width(108.0)
    .background_color(Color::rgb(0.035, 0.09, 0.115))
    .color(Color::rgb(0.94, 0.98, 0.99))
    .border_color(Color::rgb(0.32, 0.92, 0.96))
    .border_width(1.0)
    .border_radius(4.0)
    .font_size(16.0)
    .padding((8, 12))
    .margin((0, 8, 8, 0))
}

fn result(passed: bool) -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .background_color(Color::rgb(0.035, 0.09, 0.115))
    .border_color(if passed {
      Color::rgb(0.32, 0.92, 0.96)
    } else {
      Color::rgb(0.98, 0.4, 0.16)
    })
    .border_top_width(3.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
    .font_size(20.0)
    .padding(16.0)
    .margin((8, 0))
}

fn details_style(compact: bool) -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .max_width(if compact { 760.0 } else { 980.0 })
    .color(Color::rgb(0.68, 0.76, 0.78))
    .font_size(if compact { 13.0 } else { 15.0 })
    .white_space(battlement::WhiteSpace::Normal)
}
