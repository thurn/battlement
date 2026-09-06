//! Directional presence animation for controlled settings-tab content.

use battlement::{
  Color, Gradient, Length, LengthUnits, Overflow, Position, SemanticRole, Style, TransformOrigin,
};
use battlement_reactant::{
  paint::PaintStyle,
  prelude::*,
  semantics::{SemanticName, SemanticProps, SemanticVisibility},
};

use crate::settings_tabs::SettingsTab;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum TabVariant {
  Enter,
  Center,
  Exit,
}

#[derive(Clone, Copy, Hash)]
struct TravelDirection(i32);

/// Replaces one keyed settings panel with the source directional motion.
#[builder]
pub struct ArcadeTabTransition {
  #[builder(required)]
  active_key: SettingsTab,
  #[builder(required, into)]
  children: Children,
  direction: i32,
  reduce_motion: bool,
  #[builder(default = true)]
  show_effects: bool,
}

impl Component for ArcadeTabTransition {
  fn render(&self) -> impl Render {
    self::render_transition(
      self.active_key,
      self.children.clone(),
      self.direction,
      self.reduce_motion,
      self.show_effects,
    )
  }
}

fn render_transition(
  active_key: SettingsTab,
  children: Children,
  direction: i32,
  reduce_motion: bool,
  show_effects: bool,
) -> View {
  assert!(matches!(direction, -1 | 1), "tab direction must be -1 or 1");
  let direction = TravelDirection(direction);
  View::new()
    .name("arcade-tab-transition")
    .style(Style::new().position(Position::Relative).full_size())
    .child(
      AnimatePresence::new()
        .initial(false)
        .custom(direction)
        .mode(PresenceMode::PopLayout)
        .child(Node::new(
          self::panel(
            active_key,
            children,
            direction,
            !reduce_motion,
            show_effects && !reduce_motion,
          )
          .key(active_key),
        )),
    )
}

fn panel(
  active_key: SettingsTab,
  children: Children,
  direction: TravelDirection,
  enable_motion: bool,
  show_effects: bool,
) -> View {
  let panel = View::new()
    .name(format!("arcade-tab-panel-{}", active_key.slug()))
    .layout(Layout::Both)
    .style(
      Style::new()
        .position(Position::Absolute)
        .inset(0)
        .overflow(Overflow::Hidden)
        .transform_origin(TransformOrigin::two_dimensional(
          Length::Percent(if direction.0 > 0 { 100.0 } else { 0.0 }),
          Length::Percent(50.0),
        )),
    )
    .variants(self::panel_variants())
    .custom(direction)
    .animate_variant(TabVariant::Center)
    .child((
      ArcadeTabContent::new()
        .active_key(active_key)
        .children(children),
      show_effects.then(|| self::light_sweep(direction)),
      show_effects.then(self::scan_line),
    ));
  if enable_motion {
    panel
      .initial_variant(TabVariant::Enter)
      .exit_variant(TabVariant::Exit)
  } else {
    panel.initial(false)
  }
}

#[builder]
struct ArcadeTabContent {
  #[builder(required)]
  active_key: SettingsTab,
  #[builder(required, into)]
  children: Children,
}

impl Component for ArcadeTabContent {
  fn render(&self) -> impl Render {
    let is_present = use_is_present();
    View::new()
      .name(format!("arcade-tab-content-{}", self.active_key.slug()))
      .inert(!is_present)
      .semantic(
        SemanticProps::new(SemanticRole::TabPanel)
          .name(SemanticName::Text(self.active_key.label()))
          .visibility(if is_present {
            SemanticVisibility::Exposed
          } else {
            SemanticVisibility::Hidden
          }),
      )
      .style(Style::new().full_size())
      .child(self.children.render())
  }
}

fn panel_variants() -> Variants<TabVariant, TravelDirection> {
  Variants::new()
    .resolver(TabVariant::Enter, |direction: &TravelDirection| {
      VariantTarget::new(
        StyleTarget::new()
          .opacity(0.0)
          .x(direction.0 as f32 * 58.0)
          .scale(0.99),
      )
    })
    .target(
      TabVariant::Center,
      MotionTarget::new(StyleTarget::new().opacity(1.0).x(0.0).scale(1.0)).transition(
        Transition::tween()
          .duration_secs(0.36)
          .ease(Easing::CubicBezier([0.16, 1.0, 0.3, 1.0])),
      ),
    )
    .resolver(TabVariant::Exit, |direction: &TravelDirection| {
      VariantTarget::new(
        MotionTarget::new(
          StyleTarget::new()
            .opacity(0.0)
            .x(direction.0 as f32 * -34.0)
            .scale(1.01),
        )
        .transition(
          Transition::tween()
            .duration_secs(0.15)
            .ease(Easing::CubicBezier([0.7, 0.0, 1.0, 0.5])),
        ),
      )
    })
}

fn light_sweep(direction: TravelDirection) -> View {
  let (start, end, skew) = if direction.0 > 0 {
    (-90.0, 940.0, -12.0)
  } else {
    (940.0, -90.0, 12.0)
  };
  View::decorative()
    .name("arcade-tab-light-sweep")
    .style(
      Style::new()
        .position(Position::Absolute)
        .top(Length::Percent(-8.0))
        .bottom(Length::Percent(-8.0))
        .left(0)
        .width(7.pct()),
    )
    .paint(
      PaintStyle::new().background(
        Gradient::linear(90.0)
          .stop(0.0, Color::TRANSPARENT)
          .stop(0.28, Color::TRANSPARENT)
          .stop(0.28, Color::rgba8(35, 213, 255, 41))
          .stop(0.42, Color::rgba8(35, 213, 255, 41))
          .stop(0.42, Color::rgba8(211, 250, 255, 235))
          .stop(0.47, Color::rgba8(211, 250, 255, 235))
          .stop(0.47, Color::rgba8(255, 68, 210, 158))
          .stop(0.53, Color::rgba8(255, 68, 210, 158))
          .stop(0.53, Color::rgba8(72, 136, 255, 33))
          .stop(0.69, Color::rgba8(72, 136, 255, 33))
          .stop(0.69, Color::TRANSPARENT)
          .stop(1.0, Color::TRANSPARENT),
      ),
    )
    .initial(StyleTarget::new().x(start).opacity(0.0).skew_x(skew))
    .animate(
      StyleTarget::new()
        .x(end)
        .opacity_keyframes(Keyframes::new([0.0, 0.68, 0.68, 0.0]).times([0.0, 0.22, 0.72, 1.0]))
        .skew_x(skew),
    )
    .transition(
      Transition::tween()
        .duration_secs(0.34)
        .ease(Easing::CubicBezier([0.4, 0.0, 0.2, 1.0])),
    )
}

fn scan_line() -> View {
  View::decorative()
    .name("arcade-tab-scan-line")
    .style(
      Style::new()
        .position(Position::Absolute)
        .top(0)
        .left(0)
        .right(0)
        .height(3),
    )
    .paint(
      PaintStyle::new().background(
        Gradient::linear(90.0)
          .stop(0.0, Color::TRANSPARENT)
          .stop(0.14, Color::rgba8(99, 243, 255, 230))
          .stop(0.68, Color::rgba8(99, 243, 255, 230))
          .stop(0.88, Color::rgba8(255, 82, 212, 219))
          .stop(1.0, Color::TRANSPARENT),
      ),
    )
    .initial(StyleTarget::new().y(-12.0).opacity(0.0))
    .animate(
      StyleTarget::new()
        .y(1000.0)
        .opacity_keyframes(Keyframes::new([0.0, 0.38, 0.22, 0.0]).times([0.0, 0.1, 0.72, 1.0])),
    )
    .transition(Transition::tween().duration_secs(0.42).ease(Easing::Linear))
}
