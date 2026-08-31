use battlement::{Align, Color, FlexDirection, FlexWrap, LengthUnits, Style};
use battlement_reactant::prelude::*;

use crate::{Game, design_system};

const CHECKPOINTS: [u64; 5] = [0, 320, 410, 500, 720];
const CHILD_NAMES: [&str; 4] = ["ALPHA", "BRAVO", "CHARLIE", "DELTA"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RouteDirection {
  East,
  West,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum RouteVariant {
  East,
  West,
  Custom,
  Forward,
  Reverse,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VariantsOrchestrationState {
  direction: RouteDirection,
  reverse_stagger: bool,
  custom_offset: i32,
  checkpoint: usize,
  generation: u32,
}

impl Default for VariantsOrchestrationState {
  fn default() -> Self {
    Self {
      direction: RouteDirection::East,
      reverse_stagger: false,
      custom_offset: 12,
      checkpoint: 0,
      generation: 0,
    }
  }
}

impl VariantsOrchestrationState {
  fn route(&mut self) {
    self.direction = match self.direction {
      RouteDirection::East => RouteDirection::West,
      RouteDirection::West => RouteDirection::East,
    };
    self.checkpoint = 0;
    self.generation = self.generation.wrapping_add(1);
  }

  fn checkpoint(&mut self) {
    self.checkpoint = (self.checkpoint + 1) % CHECKPOINTS.len();
  }

  fn reverse_midway(&mut self) {
    self.direction = match self.direction {
      RouteDirection::East => RouteDirection::West,
      RouteDirection::West => RouteDirection::East,
    };
    self.checkpoint = 2;
    self.generation = self.generation.wrapping_add(1);
  }

  fn change_custom_data(&mut self) {
    self.custom_offset += 7;
  }

  fn toggle_stagger(&mut self) {
    self.reverse_stagger = !self.reverse_stagger;
    self.checkpoint = 0;
    self.generation = self.generation.wrapping_add(1);
  }

  fn reset(&mut self) {
    *self = Self::default();
  }
}

pub(crate) struct VariantsOrchestration {
  pub(crate) state: VariantsOrchestrationState,
  pub(crate) compact: bool,
}

#[derive(Clone)]
struct RouteChild {
  index: u32,
  direction: RouteDirection,
  reverse_stagger: bool,
  checkpoint_millis: u64,
}

impl Component for VariantsOrchestration {
  fn render(&self) -> impl Render {
    let checkpoint = CHECKPOINTS[self.state.checkpoint];
    let names = selected_names(self.state.direction, self.state.reverse_stagger);
    ScrollView::new()
      .name("variants-orchestration-canvas")
      .style(design_system::canvas(self.compact).padding(0.0))
      .content_container_style(content())
      .child(Label::new("LOGICAL VARIANT PROPAGATION").style(eyebrow()))
      .child(
        Label::new("Variants & Orchestration")
          .name("page-title")
          .style(title()),
      )
      .child(
        Label::new(format!(
          "ROUTE {}  ·  {} STAGGER  ·  CUSTOM +{}  ·  {} ms  ·  GENERATION {}",
          direction_name(self.state.direction),
          if self.state.reverse_stagger {
            "REVERSE"
          } else {
            "FORWARD"
          },
          self.state.custom_offset,
          checkpoint,
          self.state.generation,
        ))
        .name("variants-status")
        .style(status()),
      )
      .child(controls())
      .child(
        View::new()
          .name("variant-parent")
          .style(parent())
          .initial(MotionStyle::new().opacity(0.4).scale(0.96))
          .variants(parent_variants())
          .custom(self.state.custom_offset)
          .animate_variants(names)
          .child(
            Label::new("PARENT  ·  ordered [route, custom, orchestration]")
              .name("variant-ordered-list")
              .inherit_variants(false)
              .style(parent_label()),
          )
          .child(Fragment::new(
            (0..4)
              .map(|index| RouteChild {
                index,
                direction: self.state.direction,
                reverse_stagger: self.state.reverse_stagger,
                checkpoint_millis: checkpoint,
              })
              .collect::<Vec<_>>(),
          )),
      )
      .child(
        Label::new(orchestration_record(checkpoint, self.state.reverse_stagger))
          .name("variants-orchestration-record")
          .style(record()),
      )
  }
}

impl Component for RouteChild {
  fn render(&self) -> impl Render {
    let opted_out = self.index == 2;
    let (start, complete) = child_boundaries(self.index, self.reverse_stagger);
    let state = if opted_out {
      "OPTED OUT · STATIC".to_owned()
    } else if self.checkpoint_millis < start {
      format!("WAITING · starts {start} ms")
    } else if self.checkpoint_millis < complete {
      format!("ACTIVE · started {start} ms")
    } else {
      format!("COMPLETE · {complete} ms")
    };
    let names = selected_names(self.direction, self.reverse_stagger);
    View::new()
      .name(format!("variant-child-{}", self.index))
      .style(child())
      .initial(MotionStyle::new().opacity(0.25).scale(0.88))
      .variants(child_variants(self.index))
      .inherit_variants(!opted_out)
      .child(Label::new(CHILD_NAMES[self.index as usize]).style(child_name()))
      .child(
        Label::new(state)
          .name(format!("variant-child-{}-state", self.index))
          .style(child_state(opted_out)),
      )
      .child(
        View::new()
          .name(format!("variant-child-{}-nested", self.index))
          .style(nested())
          .variants(child_variants(self.index + 4))
          .animate_variants(if opted_out { Vec::new() } else { names }),
      )
  }
}

fn parent_variants() -> Variants<RouteVariant, i32> {
  Variants::new()
    .target(RouteVariant::East, MotionStyle::new().x(18.0).opacity(0.72))
    .target(
      RouteVariant::West,
      MotionStyle::new().x(-18.0).opacity(0.72),
    )
    .resolver(RouteVariant::Custom, |value| {
      VariantTarget::new(MotionStyle::new().y(*value as f32).opacity(0.88))
    })
    .target(
      RouteVariant::Forward,
      orchestrated_target(StaggerDirection::Forward),
    )
    .target(
      RouteVariant::Reverse,
      orchestrated_target(StaggerDirection::Reverse),
    )
}

fn child_variants(index: u32) -> Variants<RouteVariant, i32> {
  let distance = 26.0 + index as f32 * 6.0;
  Variants::new()
    .target(
      RouteVariant::East,
      MotionStyle::new().x(distance).opacity(0.66),
    )
    .target(
      RouteVariant::West,
      MotionStyle::new().x(-distance).opacity(0.66),
    )
    .resolver(RouteVariant::Custom, move |value| {
      VariantTarget::new(
        MotionStyle::new()
          .y((*value as f32 + index as f32 * 2.0) * 0.25)
          .opacity(0.84),
      )
    })
    .target(
      RouteVariant::Forward,
      MotionTarget::new(MotionStyle::new().scale(1.0).opacity(1.0))
        .transition(Transition::tween().duration_secs(0.22)),
    )
    .target(
      RouteVariant::Reverse,
      MotionTarget::new(MotionStyle::new().scale(0.96).opacity(1.0))
        .transition(Transition::tween().duration_secs(0.22)),
    )
}

fn orchestrated_target(direction: StaggerDirection) -> VariantTarget {
  VariantTarget::new(
    MotionTarget::new(MotionStyle::new().scale(1.0).opacity(1.0)).transition(
      Transition::tween()
        .duration_secs(0.24)
        .delay_children_secs(0.08)
        .stagger_children_secs(0.09)
        .stagger_direction(direction)
        .stagger_child_count(3)
        .when(VariantWhen::BeforeChildren),
    ),
  )
}

fn selected_names(direction: RouteDirection, reverse_stagger: bool) -> Vec<RouteVariant> {
  vec![
    match direction {
      RouteDirection::East => RouteVariant::East,
      RouteDirection::West => RouteVariant::West,
    },
    RouteVariant::Custom,
    if reverse_stagger {
      RouteVariant::Reverse
    } else {
      RouteVariant::Forward
    },
  ]
}

fn child_boundaries(index: u32, reverse: bool) -> (u64, u64) {
  if index == 2 {
    return (0, 0);
  }
  let slot = if index > 2 { index - 1 } else { index };
  let stagger_index = if reverse { 2 - slot } else { slot };
  let start = 320 + u64::from(stagger_index) * 90;
  (start, start + 220)
}

fn orchestration_record(checkpoint: u64, reverse: bool) -> String {
  let entries = (0..4)
    .map(|index| {
      if index == 2 {
        return "CHARLIE opted-out".to_owned();
      }
      let (start, complete) = child_boundaries(index, reverse);
      let state = if checkpoint < start {
        "waiting"
      } else if checkpoint < complete {
        "active"
      } else {
        "complete"
      };
      format!("{} {start}→{complete} {state}", CHILD_NAMES[index as usize])
    })
    .collect::<Vec<_>>()
    .join("  ›  ");
  format!("ORDER RECORD  ·  parent 0→240  ›  children released 320  ·  {entries}")
}

fn direction_name(value: RouteDirection) -> &'static str {
  match value {
    RouteDirection::East => "EAST",
    RouteDirection::West => "WEST",
  }
}

fn controls() -> View {
  View::new()
    .style(control_row())
    .child(action("ROUTE", "variants-route", |game| {
      game.variants_orchestration.route();
    }))
    .child(action("CHECKPOINT", "variants-checkpoint", |game| {
      game.variants_orchestration.checkpoint();
    }))
    .child(action(
      "REVERSE MIDWAY",
      "variants-reverse-midway",
      |game| {
        game.variants_orchestration.reverse_midway();
      },
    ))
    .child(action("CUSTOM +7", "variants-custom", |game| {
      game.variants_orchestration.change_custom_data();
    }))
    .child(action("STAGGER", "variants-stagger", |game| {
      game.variants_orchestration.toggle_stagger();
    }))
    .child(action("RESET", "variants-reset", |game| {
      game.variants_orchestration.reset();
    }))
    .child(action("STYLES", "variants-styles", |game| {
      game.screen = crate::Screen::StylesDecorations;
    }))
}

fn action(
  text: &'static str,
  name: &'static str,
  callback: impl Fn(&mut Game) + 'static,
) -> Button {
  Button::new(text)
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
    .font_size(17.0)
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
fn parent() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .padding(14.0)
    .background_color(Color::rgb(0.025, 0.06, 0.08))
    .border_color(Color::rgb(0.15, 0.28, 0.32))
    .border_width(1.0)
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}
fn parent_label() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .font_size(14.0)
    .color(Color::rgb(0.32, 0.92, 0.96))
    .margin((0, 0, 10, 0))
}
fn child() -> Style {
  Style::new()
    .width(204.0)
    .min_height(112.0)
    .padding(12.0)
    .margin((4, 8, 4, 0))
    .background_color(Color::rgb(0.045, 0.11, 0.14))
    .border_color(Color::rgb(0.22, 0.5, 0.54))
    .border_width(1.0)
}
fn child_name() -> Style {
  Style::new()
    .font_size(18.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
}
fn child_state(opted_out: bool) -> Style {
  Style::new()
    .font_size(12.0)
    .color(if opted_out {
      Color::rgb(0.98, 0.58, 0.18)
    } else {
      Color::rgb(0.48, 0.9, 0.7)
    })
    .margin((8, 0))
}
fn nested() -> Style {
  Style::new()
    .width(68.0)
    .height(8.0)
    .border_radius(4.0)
    .background_color(Color::rgb(0.65, 0.28, 0.95))
}
fn record() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .font_size(14.0)
    .white_space(battlement::WhiteSpace::Normal)
    .color(Color::rgb(0.68, 0.76, 0.78))
    .margin((14, 0))
}
