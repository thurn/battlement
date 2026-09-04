use trox::{assert_localized, tx};

use crate::{Game, design_system};
use battlement::{Align, Color, FlexDirection, FlexWrap, LengthUnits, Style};
use battlement_reactant::prelude::*;

const CHECKPOINTS: [u64; 4] = [0, 120_000, 320_000, 900_000];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalState {
  Active,
  Stopped,
  Cancelled,
  Completed,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PhysicalMotionState {
  checkpoint: usize,
  interrupted: bool,
  playing: bool,
  reversed: bool,
  terminal: TerminalState,
  trace: Vec<&'static str>,
}

impl Default for PhysicalMotionState {
  fn default() -> Self {
    Self {
      checkpoint: 0,
      interrupted: false,
      playing: false,
      reversed: false,
      terminal: TerminalState::Active,
      trace: vec!["activated"],
    }
  }
}

impl PhysicalMotionState {
  fn elapsed_micros(&self) -> u64 {
    CHECKPOINTS[self.checkpoint]
  }

  fn checkpoint(&mut self) {
    if self.terminal != TerminalState::Active {
      return;
    }
    self.checkpoint = (self.checkpoint + 1) % CHECKPOINTS.len();
    self.trace.push(match self.checkpoint {
      0 => "seek 0ms",
      1 => "seek 120ms",
      2 => "seek 320ms",
      _ => "seek 900ms",
    });
  }

  fn interrupt(&mut self) {
    if self.terminal != TerminalState::Active {
      return;
    }
    self.interrupted = !self.interrupted;
    self.trace.push(if self.interrupted {
      "retarget · velocity +"
    } else {
      "retarget · velocity −"
    });
  }

  fn play(&mut self) {
    if self.terminal == TerminalState::Active {
      self.playing = true;
      self.trace.push("play");
    }
  }

  fn pause(&mut self) {
    if self.terminal == TerminalState::Active {
      self.playing = false;
      self.trace.push("pause");
    }
  }

  fn reverse(&mut self) {
    if self.terminal == TerminalState::Active {
      self.reversed = !self.reversed;
      self.trace.push("reverse");
    }
  }

  fn terminate(&mut self, terminal: TerminalState) {
    if self.terminal != TerminalState::Active {
      return;
    }
    self.playing = false;
    self.terminal = terminal;
    self.trace.push(match terminal {
      TerminalState::Stopped => "stopped · value frozen",
      TerminalState::Cancelled => "cancelled · layer revealed",
      TerminalState::Completed => "completed · target applied",
      TerminalState::Active => unreachable!(),
    });
  }

  fn reset(&mut self) {
    *self = Self::default();
  }
}

#[builder]
pub(crate) struct PhysicalMotion {
  pub(crate) state: PhysicalMotionState,
  pub(crate) compact: bool,
}

impl Component for PhysicalMotion {
  fn render(&self) -> impl Render {
    let elapsed = self.state.elapsed_micros() as f64 / 1_000_000.0;
    let trace = self
      .state
      .trace
      .iter()
      .rev()
      .take(5)
      .copied()
      .collect::<Vec<_>>()
      .join("  ›  ");
    battlement_reactant::host::ScrollView::new()
      .name("physical-motion-canvas")
      .style(design_system::canvas(self.compact).padding(0.0))
      .content_container_style(content())
      .child(
        Label::new(tx(
          "PHYSICAL GENERATORS",
          "User-facing product copy in the Reactant sample.",
        ))
        .style(eyebrow()),
      )
      .child(
        Label::new(tx(
          "Physical Motion",
          "User-facing product copy in the Reactant sample.",
        ))
        .name("page-title")
        .style(title()),
      )
      .child(
        Label::new(assert_localized(format!(
          "CONTROLLED CLOCK  ·  {} ms  ·  {}  ·  {}",
          self.state.elapsed_micros() / 1_000,
          if self.state.playing {
            "playing"
          } else {
            "paused"
          },
          if self.state.reversed {
            "reverse"
          } else {
            "forward"
          },
        )))
        .name("physical-clock")
        .style(status()),
      )
      .child(controls())
      .child(
        Label::new(assert_localized(trace))
          .name("physical-event-trace")
          .style(event_trace()),
      )
      .child(
        Label::new(tx(
          "CHECKPOINTS  0 ms  ━  120 ms  ━  320 ms  ━  900 ms",
          "User-facing product copy in the Reactant sample.",
        ))
        .name("physical-checkpoint-markers")
        .style(markers()),
      )
      .child(gallery(elapsed, self.state.interrupted))
  }
}

fn controls() -> View {
  View::new()
    .style(control_row())
    .child(action("CHECKPOINT", "physical-checkpoint", |game| {
      game.physical_motion.checkpoint();
    }))
    .child(action("INTERRUPT", "physical-interrupt", |game| {
      game.physical_motion.interrupt();
    }))
    .child(action("PLAY", "physical-play", |game| {
      game.physical_motion.play();
    }))
    .child(action("PAUSE", "physical-pause", |game| {
      game.physical_motion.pause();
    }))
    .child(action("REVERSE", "physical-reverse", |game| {
      game.physical_motion.reverse();
    }))
    .child(action("STOP", "physical-stop", |game| {
      game.physical_motion.terminate(TerminalState::Stopped);
    }))
    .child(action("CANCEL", "physical-cancel", |game| {
      game.physical_motion.terminate(TerminalState::Cancelled);
    }))
    .child(action("COMPLETE", "physical-complete", |game| {
      game.physical_motion.terminate(TerminalState::Completed);
    }))
    .child(action("RESET", "physical-reset", |game| {
      game.physical_motion.reset();
    }))
    .child(action("STYLES", "physical-styles", |game| {
      game.screen = crate::Screen::StylesDecorations;
    }))
}

fn gallery(elapsed: f64, interrupted: bool) -> View {
  let cards = [
    specimen(
      "physical-duration-spring",
      "DURATION + BOUNCE",
      "0.80 s · bounce 0.30 · derived coefficients",
      spring_probe(Transition::spring().duration_secs(0.8).bounce(0.3), elapsed),
    ),
    specimen(
      "physical-underdamped",
      "UNDERDAMPED",
      "k 100 · c 10 · visible overshoot",
      spring_probe(Transition::spring().stiffness(100.0).damping(10.0), elapsed),
    ),
    specimen(
      "physical-critical",
      "CRITICAL",
      "k 100 · c 20 · fastest no-overshoot path",
      spring_probe(Transition::spring().stiffness(100.0).damping(20.0), elapsed),
    ),
    specimen(
      "physical-overdamped",
      "OVERDAMPED",
      "k 100 · c 30 · restrained convergence",
      spring_probe(Transition::spring().stiffness(100.0).damping(30.0), elapsed),
    ),
    specimen(
      "physical-interruption",
      "VELOCITY HANDOFF",
      "retarget keeps rendered position + signed velocity",
      View::new()
        .style(probe())
        .initial(StyleTarget::new().x(0.0))
        .animate(StyleTarget::new().x(if interrupted { -44.0 } else { 64.0 }))
        .transition(
          Transition::spring()
            .stiffness(140.0)
            .damping(12.0)
            .delay_secs(-elapsed.min(0.32)),
        ),
    ),
    specimen(
      "physical-inertia",
      "INERTIA · FREE / BOUNDED",
      "analytic decay · scale boundary 1.35 · snap 0.05",
      inertia_probes(elapsed),
    ),
  ];
  View::new()
    .name("physical-motion-gallery")
    .style(gallery_style())
    .child(Fragment::new(cards))
}

fn inertia_probes(elapsed: f64) -> View {
  View::new()
    .style(inertia_row())
    .child(
      View::new()
        .style(probe().width(74.0))
        .initial(StyleTarget::new().scale_x(0.7))
        .animate(StyleTarget::new().scale_x(1.0))
        .transition(
          Transition::inertia()
            .initial_velocity(0.9)
            .power(0.55)
            .delay_secs(-elapsed),
        ),
    )
    .child(
      View::new()
        .style(probe().width(74.0))
        .initial(StyleTarget::new().scale_x(0.7))
        .animate(StyleTarget::new().scale_x(1.0))
        .transition(
          Transition::inertia()
            .initial_velocity(0.9)
            .power(0.55)
            .minimum(0.65)
            .maximum(1.35)
            .bounce_stiffness(500.0)
            .bounce_damping(18.0)
            .target(InertiaTarget::nearest_multiple(0.05))
            .delay_secs(-elapsed),
        ),
    )
}

fn spring_probe(transition: Transition, elapsed: f64) -> View {
  View::new()
    .style(probe())
    .initial(StyleTarget::new().x(0.0))
    .animate(StyleTarget::new().x(58.0))
    .transition(transition.delay_secs(-elapsed))
}

fn specimen(name: &'static str, heading: &'static str, detail: &'static str, probe: View) -> Node {
  Node::new(
    View::new()
      .name(name)
      .style(specimen_style())
      .child(Label::new(assert_localized(heading)).style(specimen_title()))
      .child(Label::new(assert_localized(detail)).style(specimen_detail()))
      .child(probe),
  )
}

fn action(
  text: &'static str,
  name: &'static str,
  callback: impl Fn(&mut Game) + 'static,
) -> Button {
  Button::new(assert_localized(text))
    .name(name)
    .style(action_style())
    .on_click(callback)
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
    .font_size(18.0)
    .color(Color::rgb(0.68, 0.76, 0.78))
}

fn control_row() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin((10, 0))
}

fn action_style() -> Style {
  Style::new()
    .height(40.0)
    .min_width(94.0)
    .background_color(Color::rgb(0.035, 0.09, 0.115))
    .color(Color::rgb(0.94, 0.98, 0.99))
    .border_color(Color::rgb(0.32, 0.92, 0.96))
    .border_width(1.0)
    .font_size(14.0)
    .margin((0, 7, 7, 0))
}

fn event_trace() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .min_height(48.0)
    .padding(12.0)
    .background_color(Color::rgb(0.025, 0.06, 0.08))
    .color(Color::rgb(0.32, 0.92, 0.96))
    .font_size(14.0)
}

fn markers() -> Style {
  Style::new()
    .margin((12, 0, 6, 0))
    .color(Color::rgb(0.98, 0.64, 0.28))
    .font_size(14.0)
}

fn gallery_style() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}

fn specimen_style() -> Style {
  Style::new()
    .width(270.0)
    .min_height(148.0)
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

fn specimen_detail() -> Style {
  Style::new()
    .font_size(13.0)
    .white_space(battlement::WhiteSpace::Normal)
    .color(Color::rgb(0.68, 0.76, 0.78))
    .margin((6, 0))
}

fn inertia_row() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
}

fn probe() -> Style {
  Style::new()
    .width(176.0)
    .height(42.0)
    .background_color(Color::rgb(0.13, 0.78, 0.88))
    .border_radius(21.0)
    .margin((8, 0))
}
