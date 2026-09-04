use trox::{ls, tx};

use crate::{Game, design_system};
use battlement::{Align, Color, FlexDirection, FlexWrap, Length, LengthUnits, Overflow, Style};
use battlement_reactant::prelude::*;

const HOST_COUNT: usize = 200;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum PerformanceScenario {
  Transform200,
  Mixed200,
  MixedInteraction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PerformanceStructure {
  pub(crate) hosts: usize,
  pub(crate) graph_nodes: usize,
  pub(crate) subscriptions: usize,
  pub(crate) active_timelines: usize,
  pub(crate) layout_tracks: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MotionPerformanceState {
  scenario: PerformanceScenario,
  phase: u32,
  subscribed: bool,
}

impl Default for MotionPerformanceState {
  fn default() -> Self {
    Self {
      scenario: PerformanceScenario::Transform200,
      phase: 0,
      subscribed: false,
    }
  }
}

impl MotionPerformanceState {
  pub(crate) fn profiled(value: &str) -> Option<Self> {
    let scenario = match value {
      "transform-200" => PerformanceScenario::Transform200,
      "mixed-200" => PerformanceScenario::Mixed200,
      "mixed-interaction" => PerformanceScenario::MixedInteraction,
      _ => return None,
    };
    Some(Self {
      scenario,
      phase: 0,
      subscribed: false,
    })
  }

  pub(crate) fn structure(&self) -> PerformanceStructure {
    structure(self.scenario, self.subscribed)
  }
}

#[builder]
pub(crate) struct MotionPerformance {
  pub(crate) state: MotionPerformanceState,
  pub(crate) compact: bool,
}

#[builder]
struct PerformanceWorkload {
  #[builder(required)]
  scenario: PerformanceScenario,
  phase: u32,
}

impl Component for MotionPerformance {
  fn render(&self) -> impl Render {
    let structure = self.state.structure();
    ScrollView::new()
      .name("motion-performance-canvas")
      .style(design_system::canvas(self.compact).padding(0.0))
      .content_container_style(content())
      .child(
        Label::new(tx(
          "RELEASE PROFILING",
          "Motion performance section heading.",
        ))
        .style(eyebrow()),
      )
      .child(
        Label::new(tx(
          "Motion Performance",
          "Motion performance interface label.",
        ))
        .name("page-title")
        .style(title()),
      )
      .child(controls(&self.state))
      .child(
        Label::new(ls(format!(
          "{} · PHASE {} · 5S WARM-UP + 30S SAMPLE",
          scenario_name(self.state.scenario),
          self.state.phase,
        )))
        .name("motion-performance-status")
        .style(status()),
      )
      .child(counter_strip(structure))
      .child(
        PerformanceWorkload::new()
          .scenario(self.state.scenario)
          .phase(self.state.phase)
          .key(self.state.scenario),
      )
  }
}

impl Component for PerformanceWorkload {
  fn render(&self) -> impl Render {
    workload(self.scenario, self.phase)
  }
}

pub(crate) fn structure(scenario: PerformanceScenario, subscribed: bool) -> PerformanceStructure {
  let subscriptions = usize::from(subscribed);
  match scenario {
    PerformanceScenario::Transform200 => PerformanceStructure {
      hosts: HOST_COUNT,
      graph_nodes: 120,
      subscriptions,
      active_timelines: 160,
      layout_tracks: 0,
    },
    PerformanceScenario::Mixed200 => PerformanceStructure {
      hosts: HOST_COUNT,
      graph_nodes: 75,
      subscriptions,
      active_timelines: 175,
      layout_tracks: 10,
    },
    PerformanceScenario::MixedInteraction => PerformanceStructure {
      hosts: HOST_COUNT,
      graph_nodes: 87,
      subscriptions,
      active_timelines: HOST_COUNT,
      layout_tracks: 29,
    },
  }
}

fn controls(state: &MotionPerformanceState) -> View {
  View::new()
    .style(control_row())
    .child(action("TRANSFORM-200", "performance-transform", |game| {
      game.motion_performance.scenario = PerformanceScenario::Transform200;
      game.motion_performance.phase = 0;
    }))
    .child(action("MIXED-200", "performance-mixed", |game| {
      game.motion_performance.scenario = PerformanceScenario::Mixed200;
      game.motion_performance.phase = 0;
    }))
    .child(action("INTERACTION", "performance-interaction", |game| {
      game.motion_performance.scenario = PerformanceScenario::MixedInteraction;
      game.motion_performance.phase = 0;
    }))
    .child(action("STEP", "performance-step", |game| {
      game.motion_performance.phase = game.motion_performance.phase.wrapping_add(1);
    }))
    .child(action(
      if state.subscribed {
        "UNSUBSCRIBE"
      } else {
        "SUBSCRIBE"
      },
      "performance-subscription",
      |game| {
        game.motion_performance.subscribed = !game.motion_performance.subscribed;
      },
    ))
    .child(action("RESET", "performance-reset", |game| {
      game.motion_performance = MotionPerformanceState::default();
    }))
}

fn counter_strip(structure: PerformanceStructure) -> View {
  View::new()
    .name("motion-performance-counters")
    .style(counters())
    .child(counter("HOSTS", structure.hosts))
    .child(counter("TIMELINES", structure.active_timelines))
    .child(counter("LAYOUT", structure.layout_tracks))
    .child(counter("GRAPH", structure.graph_nodes))
    .child(counter("SUBSCRIPTIONS", structure.subscriptions))
    .child(counter("CPU P95", "HOST"))
    .child(counter("FRAME PACING", "PRESENT"))
    .child(counter("ALLOC", "HOST"))
    .child(counter("PROPERTIES", "HOST"))
    .child(counter("LIFECYCLE", "HOST"))
}

fn counter(label: &'static str, value: impl ToString) -> View {
  View::new()
    .style(counter_style())
    .child(Label::new(ls(label)).style(counter_label()))
    .child(Label::new(ls(value.to_string())).style(counter_value()))
}

fn workload(scenario: PerformanceScenario, phase: u32) -> View {
  let children = match scenario {
    PerformanceScenario::Transform200 => transform_hosts(phase),
    PerformanceScenario::Mixed200 => mixed_hosts(phase),
    PerformanceScenario::MixedInteraction => interaction_hosts(phase),
  };
  View::new()
    .name("motion-performance-grid")
    .style(grid())
    .child(children)
}

fn transform_hosts(phase: u32) -> Vec<View> {
  (0..HOST_COUNT)
    .map(|index| {
      let target = alternating_target(index, phase);
      let host = probe(index);
      if index < 60 {
        host
          .initial(StyleTarget::new().x(-5.0).y(-3.0).scale(0.92).opacity(0.55))
          .animate(target)
          .transition(repeating_tween(index))
      } else if index < 120 {
        host
          .initial(StyleTarget::new().x(-6.0))
          .animate(StyleTarget::new().x(if target_side(index, phase) { 6.0 } else { -6.0 }))
          .transition(
            Transition::spring()
              .stiffness(180.0)
              .damping(21.0)
              .mass(0.8),
          )
      } else if index < 160 {
        graph_probe(host, index, phase)
      } else {
        gesture_probe(host, index)
      }
    })
    .collect()
}

fn mixed_hosts(phase: u32) -> Vec<View> {
  (0..HOST_COUNT)
    .map(|index| {
      let host = probe(index);
      if index < 50 {
        host
          .initial(StyleTarget::new().x(-4.0).opacity(0.55))
          .animate(alternating_target(index, phase))
          .transition(repeating_tween(index))
      } else if index < 100 {
        host
          .animate(StyleTarget::new().scale(if target_side(index, phase) {
            1.08
          } else {
            0.92
          }))
          .transition(Transition::spring().stiffness(170.0).damping(20.0))
      } else if index < 125 {
        graph_probe(host, index, phase)
      } else if index < 150 {
        gesture_probe(host, index)
      } else if index < 160 {
        host
          .layout(Layout::Both)
          .animate(alternating_target(index, phase))
      } else if index < 170 {
        host.animate(
          StyleTarget::new()
            .background_color(Color::rgba(0.1, 0.72, 0.86, 1.0))
            .filter(
              MotionFilterList::default().contrast(if target_side(index, phase) {
                1.35
              } else {
                0.72
              }),
            ),
        )
      } else if index < 180 {
        host.animate(
          StyleTarget::new()
            .clip_inset([Length::px(if target_side(index, phase) { 1.0 } else { 5.0 }); 4]),
        )
      } else if index < 190 {
        host.animate(
          StyleTarget::new()
            .box_shadow([battlement::Shadow {
              x: 0.0,
              y: 2.0,
              blur: 0.0,
              spread: 1.0,
              color: Color::rgba(0.0, 0.8, 0.9, 0.5),
              inset: false,
            }])
            .opacity(0.9),
        )
      } else {
        host.animate(
          StyleTarget::new()
            .rotate_x(if target_side(index, phase) {
              14.0
            } else {
              -14.0
            })
            .skew_x(if target_side(index, phase) { 6.0 } else { -6.0 }),
        )
      }
    })
    .collect()
}

fn interaction_hosts(phase: u32) -> Vec<View> {
  (0..HOST_COUNT)
    .map(|index| {
      let family = index % 7;
      let host = probe(index)
        .key((index, phase % 4))
        .initial(StyleTarget::new().opacity(0.35).scale(0.82))
        .transition(repeating_tween(index));
      match family {
        0 => host
          .layout(Layout::Position)
          .animate(StyleTarget::new().x(if phase.is_multiple_of(2) { -6.0 } else { 6.0 })),
        1 => host.animate(StyleTarget::new().y(if phase.is_multiple_of(3) { -5.0 } else { 5.0 })),
        2 => {
          host.animate(StyleTarget::new().opacity(if phase.is_multiple_of(2) { 0.45 } else { 1.0 }))
        }
        3 => gesture_probe(host, index),
        4 => {
          host.animate(StyleTarget::new().scale(if phase.is_multiple_of(2) { 0.8 } else { 1.15 }))
        }
        5 => host.animate(StyleTarget::new().rotate(if phase.is_multiple_of(2) {
          -10.0
        } else {
          10.0
        })),
        _ => graph_probe(host, index, phase),
      }
    })
    .collect()
}

fn graph_probe(host: View, index: usize, phase: u32) -> View {
  let source = use_motion_value(if target_side(index, phase) {
    1.0_f32
  } else {
    0.0
  });
  let mapped = use_transform(
    source.clone(),
    InputRange::new([0.0, 1.0]),
    OutputRange::new([-6.0, 6.0]),
  );
  let spring = use_spring(mapped, SpringOptions::new().stiffness(160.0).damping(19.0));
  host
    .animate(
      StyleTarget::new()
        .x_value(spring)
        .opacity(if target_side(index, phase) {
          0.95
        } else {
          0.62
        }),
    )
    .transition(repeating_tween(index))
}

fn gesture_probe(host: View, index: usize) -> View {
  host
    .name(format!("performance-gesture-{index}"))
    .drag(DragAxis::Both)
    .drag_constraints(DragConstraints::bounds(-8.0, 8.0, -6.0, 6.0))
    .drag_momentum(false)
    .while_hover(StyleTarget::new().scale(1.08))
    .while_tap(StyleTarget::new().scale(0.9))
    .while_drag(StyleTarget::new().opacity(0.7))
    .animate(StyleTarget::new().rotate(if index.is_multiple_of(2) { 4.0 } else { -4.0 }))
    .transition(repeating_tween(index))
}

fn probe(index: usize) -> View {
  View::new()
    .key(index)
    .name(format!("performance-host-{index}"))
    .style(probe_style(index))
}

fn alternating_target(index: usize, phase: u32) -> StyleTarget {
  let positive = target_side(index, phase);
  StyleTarget::new()
    .x(if positive { 5.0 } else { -5.0 })
    .y(if positive { -3.0 } else { 3.0 })
    .scale(if positive { 1.06 } else { 0.94 })
    .opacity(if positive { 1.0 } else { 0.62 })
}

fn target_side(index: usize, phase: u32) -> bool {
  (index + phase as usize).is_multiple_of(2)
}

fn repeating_tween(index: usize) -> Transition {
  Transition::tween()
    .duration_secs(0.7 + (index % 9) as f64 * 0.037)
    .repeat(Repeat::Count(9_999))
    .repeat_type(RepeatType::Mirror)
}

fn action(
  text: &'static str,
  name: &'static str,
  callback: impl Fn(&mut Game) + 'static,
) -> impl Render {
  Button::new(ls(text))
    .host_name(name)
    .style(action_style())
    .on_press(callback)
}

fn scenario_name(scenario: PerformanceScenario) -> &'static str {
  match scenario {
    PerformanceScenario::Transform200 => "TRANSFORM-200",
    PerformanceScenario::Mixed200 => "MIXED-200",
    PerformanceScenario::MixedInteraction => "MIXED INTERACTION",
  }
}

fn content() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .padding(22.0)
    .align_items(Align::FlexStart)
}

fn eyebrow() -> Style {
  Style::new()
    .font_size(11.0)
    .color(Color::rgb(0.25, 0.84, 0.9))
}

fn title() -> Style {
  Style::new()
    .font_size(34.0)
    .color(Color::rgb(0.92, 0.96, 0.98))
    .margin_bottom(10.0)
}

fn status() -> Style {
  Style::new()
    .font_size(11.0)
    .color(Color::rgb(0.65, 0.72, 0.77))
    .margin_bottom(10.0)
}

fn control_row() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin_bottom(10.0)
}

fn action_style() -> Style {
  Style::new()
    .height(28.0)
    .margin_right(6.0)
    .margin_bottom(6.0)
    .font_size(9.0)
    .color(Color::rgb(0.85, 0.92, 0.94))
    .background_color(Color::rgb(0.08, 0.15, 0.19))
}

fn counters() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin_bottom(10.0)
}

fn counter_style() -> Style {
  Style::new()
    .width(116.0)
    .height(40.0)
    .margin_right(5.0)
    .margin_bottom(5.0)
    .padding(5.0)
    .background_color(Color::rgb(0.055, 0.095, 0.12))
}

fn counter_label() -> Style {
  Style::new()
    .font_size(7.0)
    .color(Color::rgb(0.44, 0.62, 0.68))
}

fn counter_value() -> Style {
  Style::new()
    .font_size(12.0)
    .color(Color::rgb(0.92, 0.96, 0.98))
}

fn grid() -> Style {
  Style::new()
    .width(740.0)
    .height(370.0)
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .overflow(Overflow::Hidden)
    .padding(5.0)
    .background_color(Color::rgb(0.025, 0.045, 0.06))
}

fn probe_style(index: usize) -> Style {
  let accent = (index % 10) as f64 / 90.0;
  Style::new()
    .width(31.0)
    .height(28.0)
    .margin(2.5)
    .background_color(Color::rgb(0.06 + accent, 0.36 + accent, 0.46 + accent))
}

#[cfg(test)]
mod tests {
  use super::{PerformanceScenario, structure};

  #[test]
  fn structural_workloads_are_exact() {
    let transform = structure(PerformanceScenario::Transform200, false);
    assert_eq!(
      (
        transform.hosts,
        transform.graph_nodes,
        transform.active_timelines
      ),
      (200, 120, 160)
    );
    let mixed = structure(PerformanceScenario::Mixed200, true);
    assert_eq!(
      (mixed.hosts, mixed.layout_tracks, mixed.subscriptions),
      (200, 10, 1)
    );
    let interaction = structure(PerformanceScenario::MixedInteraction, false);
    assert_eq!(
      (
        interaction.hosts,
        interaction.active_timelines,
        interaction.layout_tracks
      ),
      (200, 200, 29)
    );
  }
}
