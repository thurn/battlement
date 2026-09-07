//! Keyed full-screen replacement with the source CRT reveal treatment.

use crate::{arcade_frame_pulse::ArcadeScreen, assets, frame_styles};
use battlement::{
  Color, ImageScaleMode, Length, LengthUnits, Overflow, PickingMode, Position, SemanticRole,
  Shadow, Style, TransformOrigin,
};
use battlement_reactant::{paint::PaintStyle, prelude::*, semantics::SemanticVisibility};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum ScreenVariant {
  Enter,
  Center,
  Exit,
}

/// Replaces one keyed application screen with the source CRT transition.
#[builder]
pub struct ArcadeMenuTransition {
  #[builder(required)]
  screen_key: ArcadeScreen,
  #[builder(required, into)]
  children: Children,
  play_transition: bool,
  reduce_motion: bool,
}

impl Component for ArcadeMenuTransition {
  fn render(&self) -> impl Render {
    self::transition(self)
  }
}

#[builder]
struct ArcadeMenuScreen {
  #[builder(required)]
  screen_key: ArcadeScreen,
  #[builder(required, into)]
  children: Children,
  play_transition: bool,
  reduce_motion: bool,
}

impl Component for ArcadeMenuScreen {
  fn render(&self) -> impl Render {
    let is_present = use_is_present();
    self::screen(self, is_present)
  }
}

fn transition(component: &ArcadeMenuTransition) -> View {
  View::new()
    .name("arcade-menu-transition")
    .style(
      Style::new()
        .position(Position::Absolute)
        .inset(0)
        .overflow(Overflow::Hidden),
    )
    .child((
      AnimatePresence::new()
        .initial(false)
        .mode(PresenceMode::Sync)
        .child(Node::new(
          ArcadeMenuScreen::new()
            .screen_key(component.screen_key)
            .children(component.children.clone())
            .play_transition(component.play_transition)
            .reduce_motion(component.reduce_motion)
            .key(component.screen_key),
        )),
      self::contained_effect(
        component.screen_key,
        component.play_transition,
        component.reduce_motion,
      ),
    ))
}

fn screen(component: &ArcadeMenuScreen, is_present: bool) -> View {
  let panel = View::new()
    .name(format!(
      "arcade-menu-screen-{}",
      self::slug(component.screen_key)
    ))
    .inert(!is_present)
    .style(
      Style::new()
        .position(Position::Absolute)
        .inset(0)
        .overflow(Overflow::Hidden),
    )
    .variants(self::screen_variants())
    .child(component.children.render());
  let panel = if is_present {
    panel
  } else {
    panel.semantic(SemanticProps::new(SemanticRole::Group).visibility(SemanticVisibility::Hidden))
  };
  if component.reduce_motion {
    panel.initial(false)
  } else if component.play_transition {
    panel
      .initial_variant(ScreenVariant::Enter)
      .animate_variant(ScreenVariant::Center)
      .exit_variant(ScreenVariant::Exit)
  } else {
    panel.initial(false).exit_variant(ScreenVariant::Exit)
  }
}

fn screen_variants() -> Variants<ScreenVariant, ()> {
  Variants::new()
    .target(ScreenVariant::Enter, self::collapsed_screen(2.2))
    .target(
      ScreenVariant::Center,
      MotionTarget::new(
        StyleTarget::new()
          .clip_inset([Length::px(0.0); 4])
          .filter(self::screen_filter(1.0, 0.0))
          .opacity(1.0),
      )
      .transition(self::screen_transition()),
    )
    .target(
      ScreenVariant::Exit,
      MotionTarget::new(self::collapsed_screen(2.35)).transition(self::screen_transition()),
    )
}

fn collapsed_screen(contrast: f32) -> StyleTarget {
  StyleTarget::new()
    .clip_inset([
      Length::percent(49.35),
      Length::percent(8.0),
      Length::percent(49.35),
      Length::percent(8.0),
    ])
    .filter(self::screen_filter(contrast, 3.0))
    .opacity(0.0)
}

fn screen_filter(contrast: f32, blur: f32) -> MotionFilterList {
  MotionFilterList::default().contrast(contrast).blur(blur)
}

fn screen_transition() -> Transition {
  Transition::tween()
    .duration_secs(0.3)
    .delay_secs(0.17)
    .ease(Easing::CubicBezier([0.16, 1.0, 0.3, 1.0]))
}

fn contained_effect(
  screen_key: ArcadeScreen,
  play_transition: bool,
  reduce_motion: bool,
) -> Option<View> {
  (play_transition && !reduce_motion).then(|| {
    View::decorative()
      .name("arcade-menu-contained-effect")
      .key((screen_key, "effect"))
      .style(
        Style::new()
          .position(Position::Absolute)
          .top(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
          .right(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
          .bottom(frame_styles::OUTER_BOTTOM + frame_styles::BORDER_THICKNESS)
          .left(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
          .overflow(Overflow::Hidden),
      )
      .child((self::reveal_scan(), self::beam(screen_key)))
  })
}

fn reveal_scan() -> impl Render {
  assets::MENU_REVEAL_SCAN
    .image()
    .name("arcade-menu-reveal-scan")
    .picking_mode(PickingMode::Ignore)
    .scale_mode(ImageScaleMode::StretchToFill)
    .style(Style::new().position(Position::Absolute).inset(0))
    .initial(
      StyleTarget::new()
        .clip_inset(self::scan_clip(49.7))
        .opacity(0.0),
    )
    .animate(
      StyleTarget::new()
        .clip_inset_keyframes(
          Keyframes::new([
            self::scan_clip(49.7),
            self::scan_clip(46.0),
            self::scan_clip(0.0),
          ])
          .times([0.0, 0.44, 1.0]),
        )
        .opacity_keyframes(Keyframes::new([0.0, 0.48, 0.0]).times([0.0, 0.44, 1.0])),
    )
    .transition(
      Transition::tween()
        .duration_secs(0.5)
        .ease(Easing::CubicBezier([0.65, 0.0, 0.35, 1.0])),
    )
}

fn scan_clip(vertical: f32) -> [Length; 4] {
  [
    Length::percent(vertical),
    Length::px(0.0),
    Length::percent(vertical),
    Length::px(0.0),
  ]
}

fn beam(screen_key: ArcadeScreen) -> View {
  View::decorative()
    .name("arcade-menu-transition-beam")
    .style(
      Style::new()
        .position(Position::Absolute)
        .top(50.pct())
        .right(1.9.pct())
        .left(1.9.pct())
        .height(3)
        .transform_origin(TransformOrigin::two_dimensional(
          Length::percent(if screen_key == ArcadeScreen::Settings {
            0.0
          } else {
            100.0
          }),
          Length::percent(50.0),
        )),
    )
    .paint(
      PaintStyle::new()
        .background(Color::hex(0xd7f8ff))
        .box_shadow([
          Shadow::outer(0.0, 0.0, 6.0, 0.0, Color::WHITE.with_alpha(0.9)),
          Shadow::outer(0.0, 0.0, 16.0, 0.0, Color::hex(0x48bfff).with_alpha(0.75)),
          Shadow::outer(0.0, 0.0, 32.0, 0.0, Color::hex(0xac52ff).with_alpha(0.5)),
        ]),
    )
    .initial(StyleTarget::new().opacity(0.0).scale_x(0.25))
    .animate(
      StyleTarget::new()
        .opacity_keyframes(Keyframes::new([0.0, 0.68, 0.0]).times([0.0, 0.46, 1.0]))
        .scale_x_keyframes(Keyframes::new([0.25, 1.0, 0.7]).times([0.0, 0.46, 1.0])),
    )
    .transition(Transition::tween().duration_secs(0.5))
}

const fn slug(screen: ArcadeScreen) -> &'static str {
  match screen {
    ArcadeScreen::Main => "main",
    ArcadeScreen::Settings => "settings",
  }
}
