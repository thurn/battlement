use crate::animation_validation::{
  CaseId, FixtureAction, FixtureSession, ReducedMotionOverride, ValidationReport, fixture_registry,
  run_fixture_case,
};
use crate::{Game, animation_validation::runner, design_system};
use battlement::{Align, Color, FlexDirection, FlexWrap, LengthUnits, Style};
use battlement_reactant::prelude::*;

const TWEEN_CASE: CaseId = CaseId("public-tween");

const KEYFRAME_CASE: CaseId = CaseId("keyframe-boundary");

const RETARGET_CASE: CaseId = CaseId("retarget-presentation");

/// Interactive sample state for the shared animation validation strip.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ValidationUiState {
  selected_case: CaseId,
  session: FixtureSession,
  report: Option<ValidationReport>,
  show_json: bool,
  checkpoint_index: usize,
}

impl Default for ValidationUiState {
  fn default() -> Self {
    Self {
      selected_case: TWEEN_CASE,
      session: FixtureSession::default(),
      report: None,
      show_json: false,
      checkpoint_index: 0,
    }
  }
}

impl ValidationUiState {
  pub(crate) fn reset(&mut self) {
    self.session.reset();
    self.report = None;
    self.show_json = false;
    self.checkpoint_index = 0;
  }

  pub(crate) fn select_next(&mut self) {
    self.selected_case = match self.selected_case {
      TWEEN_CASE => KEYFRAME_CASE,
      KEYFRAME_CASE => RETARGET_CASE,
      _ => TWEEN_CASE,
    };
    self.reset();
  }

  pub(crate) fn seek_checkpoint(&mut self) {
    let registry = fixture_registry();
    let case = registry
      .select(
        crate::animation_validation::ScreenId("targets-timelines"),
        self.selected_case,
      )
      .expect("sample validation case should exist");
    let checkpoint = &case.checkpoints[self.checkpoint_index % case.checkpoints.len()];
    self.session.seek(checkpoint.elapsed_micros);
    self.checkpoint_index = (self.checkpoint_index + 1) % case.checkpoints.len();
  }

  pub(crate) fn capture(&mut self) {
    let registry = fixture_registry();
    let case = registry
      .select(
        crate::animation_validation::ScreenId("targets-timelines"),
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

#[builder]
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
            .name("targets-timelines-canvas")
            .style(canvas(self.compact))
            .content_container_style(content())
            .child(
                battlement_reactant::host::Label::new("MOTION AUTHORING")
                    .style(eyebrow()),
            )
            .child(
                battlement_reactant::host::Label::new("Targets & Timelines")
                    .name("page-title")
                    .style(title()),
            )
            .child(
                battlement_reactant::host::Label::new(
                        format!(
                            "Case: validation-infrastructure/{} · t={}µs · generation={} · reconnects={} · actions={}",
                            self.state.selected_case.0, self.state.session
                            .elapsed_micros(), self.state.session.generation(), self
                            .state.session.reconnects(), self.state.session.actions()
                            .len(),
                        ),
                    )
                    .name("validation-selection")
                    .style(status()),
            )
            .child(
                battlement_reactant::host::View::new()
                    .style(control_row())
                    .child(
                        action(
                            "SELECT CASE",
                            "validation-select",
                            |game| {
                                game.animation_validation.select_next();
                            },
                        ),
                    )
                    .child(
                        action(
                            "RESET",
                            "validation-reset",
                            |game| {
                                game.animation_validation.reset();
                            },
                        ),
                    )
                    .child(
                        action(
                            "SEEK",
                            "validation-seek",
                            |game| {
                                game.animation_validation.seek_checkpoint();
                            },
                        ),
                    )
                    .child(
                        action(
                            "CAPTURE",
                            "validation-capture",
                            |game| {
                                game.animation_validation.capture();
                            },
                        ),
                    )
                    .child(
                        action(
                            "EXPORT",
                            "validation-export",
                            |game| {
                                game.animation_validation.toggle_export();
                            },
                        ),
                    )
                    .child(
                        action(
                            "PHYSICAL MOTION",
                            "validation-physical",
                            |game| {
                                game.screen = crate::Screen::PhysicalMotion;
                            },
                        ),
                    ),
            )
            .child(
                battlement_reactant::host::View::new()
                    .style(control_row())
                    .child(
                        action(
                            "TRIGGER",
                            "validation-trigger",
                            |game| {
                                game.animation_validation.dispatch(FixtureAction::Trigger);
                            },
                        ),
                    )
                    .child(
                        action(
                            "PLAY",
                            "validation-play",
                            |game| {
                                game.animation_validation.dispatch(FixtureAction::Play);
                            },
                        ),
                    )
                    .child(
                        action(
                            "PAUSE",
                            "validation-pause",
                            |game| {
                                game.animation_validation.dispatch(FixtureAction::Pause);
                            },
                        ),
                    )
                    .child(
                        action(
                            "REPLAY",
                            "validation-replay",
                            |game| {
                                game.animation_validation.dispatch(FixtureAction::Replay);
                            },
                        ),
                    )
                    .child(
                        action(
                            "SPEED",
                            "validation-speed",
                            |game| {
                                let speed = match game
                                    .animation_validation
                                    .session()
                                    .speed()
                                {
                                    1.0 => 0.1,
                                    0.1 => 0.25,
                                    0.25 => 4.0,
                                    _ => 1.0,
                                };
                                game.animation_validation
                                    .dispatch(FixtureAction::Speed(speed));
                            },
                        ),
                    )
                    .child(
                        action(
                            "REDUCE",
                            "validation-reduced",
                            |game| {
                                let value = match game
                                    .animation_validation
                                    .session()
                                    .reduced_motion()
                                {
                                    ReducedMotionOverride::System => {
                                        ReducedMotionOverride::Always
                                    }
                                    ReducedMotionOverride::Always => {
                                        ReducedMotionOverride::Never
                                    }
                                    ReducedMotionOverride::Never => {
                                        ReducedMotionOverride::System
                                    }
                                };
                                game.animation_validation
                                    .dispatch(FixtureAction::ReducedMotion(value));
                            },
                        ),
                    )
                    .child(
                        action(
                            "RECONNECT",
                            "validation-reconnect",
                            |game| {
                                game.animation_validation
                                    .dispatch(FixtureAction::Reconnect);
                            },
                        ),
                    ),
            )
            .child(
                battlement_reactant::host::Label::new(
                        format!(
                            "{} · {} · {:.1}x · {:?}", report_text, if self.state
                            .session.playing() { "playing" } else { "paused" }, self
                            .state.session.speed(), self.state.session.reduced_motion(),
                        ),
                    )
                    .name("validation-result")
                    .style(
                        result(
                            self
                                .state
                                .report
                                .as_ref()
                                .is_some_and(ValidationReport::passed),
                        ),
                    ),
            )
            .child(
                timeline_gallery(
                    self.state.session.elapsed_micros(),
                    self.state.session.retargeted(),
                ),
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

fn protocol_probe() -> Style {
  Style::new()
    .width(238.0)
    .height(54.0)
    .background_color(Color::rgb(0.13, 0.78, 0.88))
    .border_radius(8.0)
    .padding(10.0)
    .margin((6, 0))
}

fn timeline_gallery(elapsed_micros: u64, retargeted: bool) -> View {
  let elapsed = elapsed_micros as f64 / 1_000_000.0;
  let linear = specimen(
    "timeline-linear",
    "LINEAR",
    "expected · start 0.00 · midpoint 0.50 · end 1.00",
    View::new()
      .name("timeline-linear-probe")
      .style(protocol_probe())
      .initial(MotionStyle::new().opacity(0.0))
      .animate(MotionStyle::new().opacity(1.0))
      .transition(
        Transition::tween()
          .duration_secs(1.0)
          .delay_secs(-elapsed)
          .ease(Easing::Linear),
      ),
  );
  let eased = specimen(
    "timeline-eased",
    "EASED",
    "expected · start 0.00 · midpoint 0.50 · end 1.00",
    View::new()
      .style(protocol_probe())
      .initial(MotionStyle::new().opacity(0.0))
      .animate(MotionStyle::new().opacity(1.0))
      .transition(
        Transition::tween()
          .duration_secs(1.0)
          .delay_secs(-elapsed)
          .ease(Easing::EaseInOut),
      ),
  );
  let keyframes = specimen(
    "timeline-keyframes",
    "KEYFRAMES + OVERRIDE",
    "expected · 0%=0.00 · 25%=0.80 · 75%=0.20 · 100%=1.00",
    View::new()
      .style(protocol_probe())
      .initial(MotionStyle::new().opacity(0.0).x(0.0))
      .animate(
        MotionStyle::new()
          .opacity_keyframes(Keyframes::new([0.0, 0.8, 0.2, 1.0]).times([0.0, 0.25, 0.75, 1.0]))
          .x(48.0),
      )
      .transition(
        Transition::tween()
          .duration_secs(1.0)
          .delay_secs(-elapsed)
          .ease(Easing::Linear)
          .property(
            MotionProperty::X,
            Transition::tween()
              .duration_secs(0.5)
              .delay_secs(-elapsed)
              .ease(Easing::EaseOut),
          ),
      ),
  );
  let repeats = specimen(
    "timeline-repeats",
    "DELAY + REVERSE + MIRROR",
    "expected · delay=0 · reverse midpoint=1 · finite end=0",
    View::new()
      .style(protocol_probe())
      .initial(MotionStyle::new().x(0.0).scale(0.75))
      .animate(MotionStyle::new().x(64.0).scale(1.2))
      .transition(
        Transition::tween()
          .duration_secs(0.5)
          .delay_secs(0.1 - elapsed)
          .ease(Easing::Linear)
          .repeat(Repeat::Count(1))
          .repeat_type(RepeatType::Reverse)
          .property(
            MotionProperty::Scale,
            Transition::tween()
              .duration_secs(0.5)
              .delay_secs(0.1 - elapsed)
              .ease(Easing::Linear)
              .repeat(Repeat::Count(1))
              .repeat_type(RepeatType::Mirror),
          ),
      ),
  );
  let shapes = specimen(
    "timeline-shapes",
    "DISCRETE + STRUCTURED",
    "expected · midpoint hidden · scale [1.25, 0.75] · end visible",
    View::new()
      .style(protocol_probe())
      .initial(
        MotionStyle::new()
          .scale(0.75)
          .visibility(Visibility::Visible),
      )
      .animate(
        MotionStyle::new()
          .scale_keyframes(Keyframes::new([0.75, 1.25, 1.0]))
          .visibility_keyframes(Keyframes::new([
            Visibility::Visible,
            Visibility::Hidden,
            Visibility::Visible,
          ])),
      )
      .transition(
        Transition::tween()
          .duration_secs(1.0)
          .delay_secs(-elapsed)
          .ease(Easing::Linear),
      ),
  );
  let target_scale = if retargeted { 0.72 } else { 1.28 };
  let target_color = if retargeted {
    MotionColor::new(0.98, 0.4, 0.16, 1.0)
  } else {
    MotionColor::new(0.13, 0.78, 0.88, 1.0)
  };
  let retarget = specimen(
    "timeline-retarget",
    "VISIBLE RETARGET",
    "expected · first post-retarget equals last pre-retarget · terminal target applied",
    View::new()
      .style(protocol_probe())
      .initial(
        MotionStyle::new()
          .scale(1.0)
          .background_color(MotionColor::new(0.12, 0.18, 0.22, 1.0)),
      )
      .animate(
        MotionTarget::new(
          MotionStyle::new()
            .scale(target_scale)
            .background_color(target_color),
        )
        .transition_end(MotionStyle::new().opacity(0.96)),
      )
      .transition(
        Transition::tween()
          .duration_secs(1.0)
          .delay_secs(-elapsed.min(0.5))
          .ease(Easing::EaseInOut),
      ),
  );
  View::new()
    .name("targets-timelines-gallery")
    .style(gallery())
    .child(Fragment::new([
      Node::new(linear),
      Node::new(eased),
      Node::new(keyframes),
      Node::new(repeats),
      Node::new(shapes),
      Node::new(retarget),
    ]))
}

fn specimen(name: &'static str, title: &'static str, expected: &'static str, probe: View) -> View {
  View::new()
    .name(name)
    .style(specimen_style())
    .child(Label::new(title).style(specimen_title()))
    .child(Label::new(expected).style(specimen_expected()))
    .child(probe)
}

fn gallery() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}

fn specimen_style() -> Style {
  Style::new()
    .width(270.0)
    .min_height(140.0)
    .padding(12.0)
    .margin((6, 8, 6, 0))
    .background_color(Color::rgb(0.025, 0.06, 0.08))
    .border_color(Color::rgb(0.15, 0.28, 0.32))
    .border_width(1.0)
}

fn specimen_title() -> Style {
  Style::new()
    .font_size(18.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
}

fn specimen_expected() -> Style {
  Style::new()
    .font_size(13.0)
    .white_space(battlement::WhiteSpace::Normal)
    .color(Color::rgb(0.68, 0.76, 0.78))
    .margin((6, 0))
}

fn details_style(compact: bool) -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .max_width(if compact { 760.0 } else { 980.0 })
    .color(Color::rgb(0.68, 0.76, 0.78))
    .font_size(if compact { 13.0 } else { 15.0 })
    .white_space(battlement::WhiteSpace::Normal)
}
