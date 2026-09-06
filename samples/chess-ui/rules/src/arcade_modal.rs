//! Accessible arcade dialogs rendered through Reactant's modal overlay.

use trox::{LocalizedString, tx};

use crate::{action_button, action_skin};
use battlement::{
  Align, Color, FlexDirection, Gradient, Justify, KeyEvent, LengthUnits, Overflow, PhysicalKey,
  PickingMode, Position, Shadow, Style, TextAnchor, WhiteSpace,
};
use battlement_reactant::{
  component::Component,
  components::Button,
  element_ref::{ElementRef, use_element_ref},
  event::ReactantEvent,
  hooks,
  host::{TextElement, View},
  overlay::Overlay,
  paint::{PaintLayer, PaintStyle},
  portal::PortalTarget,
  prelude::{
    AnimatePresence, Children, Easing, Either, EventCallback, Keyframes, MotionFilterList,
    MotionTarget, Node, PaintDropShadow, PaintFilterList, Repeat, StyleTarget, Transition, builder,
    use_is_present,
  },
  render::Render,
  semantics::SemanticName,
};

/// A source-shaped modal whose parent owns visibility and outcomes.
#[builder]
pub struct ArcadeModal {
  #[builder(required)]
  open: bool,
  title: Option<LocalizedString>,
  aria_label: Option<LocalizedString>,
  #[builder(required, into)]
  children: Children,
  #[builder(default = tx("OK", "Arcade modal confirmation label."))]
  confirm_label: LocalizedString,
  cancel_label: Option<LocalizedString>,
  danger: bool,
  #[builder(default = true)]
  close_on_escape: bool,
  #[builder(required)]
  reduce_motion: bool,
  #[builder(required)]
  on_confirm: EventCallback<()>,
  #[builder(required)]
  on_close: EventCallback<()>,
  initial_focus: Option<ElementRef>,
  #[builder(required)]
  overlay: PortalTarget,
}

#[builder]
struct OpenArcadeModal {
  open: bool,
  title: Option<LocalizedString>,
  aria_label: Option<LocalizedString>,
  #[builder(required, into)]
  children: Children,
  #[builder(required)]
  confirm_label: LocalizedString,
  cancel_label: Option<LocalizedString>,
  danger: bool,
  close_on_escape: bool,
  reduce_motion: bool,
  #[builder(required)]
  on_confirm: EventCallback<()>,
  #[builder(required)]
  on_close: EventCallback<()>,
  #[builder(required)]
  initial_focus: Option<ElementRef>,
  #[builder(required)]
  overlay: PortalTarget,
}

#[builder]
struct ModalBody {
  title: Option<LocalizedString>,
  #[builder(required, into)]
  children: Children,
  #[builder(required)]
  confirm_label: LocalizedString,
  cancel_label: Option<LocalizedString>,
  danger: bool,
  close_on_escape: bool,
  reduce_motion: bool,
  #[builder(required)]
  on_confirm: EventCallback<()>,
  #[builder(required)]
  on_close: EventCallback<()>,
}

#[builder]
struct ModalButton {
  #[builder(required)]
  label: LocalizedString,
  autofocus: bool,
  danger: bool,
  #[builder(required)]
  reference: ElementRef,
  #[builder(required)]
  on_press: EventCallback<()>,
  #[builder(required)]
  on_close: EventCallback<()>,
  close_on_escape: bool,
}

impl Component for ArcadeModal {
  fn render(&self) -> impl Render {
    OpenArcadeModal::new()
      .open(self.open)
      .title(self.title.clone())
      .aria_label(self.aria_label.clone())
      .children(self.children.clone())
      .confirm_label(self.confirm_label.clone())
      .cancel_label(self.cancel_label.clone())
      .danger(self.danger)
      .close_on_escape(self.close_on_escape)
      .reduce_motion(self.reduce_motion)
      .on_confirm(self.on_confirm.clone())
      .on_close(self.on_close.clone())
      .initial_focus(self.initial_focus.clone())
      .overlay(self.overlay.clone())
  }
}

impl OpenArcadeModal {
  fn dialog_name(&self) -> LocalizedString {
    self
      .title
      .clone()
      .or_else(|| self.aria_label.clone())
      .expect("ArcadeModal requires a title or aria_label")
  }

  fn overlay(&self) -> Overlay {
    if self.open {
      let overlay = Overlay::modal(self.overlay.clone(), SemanticName::Text(self.dialog_name()))
        .host_name("arcade-modal");
      let overlay = if let Some(initial_focus) = self.initial_focus.clone() {
        overlay.initial_focus(initial_focus)
      } else {
        overlay
      };
      if self.close_on_escape {
        overlay.on_dismiss(self.on_close.clone())
      } else {
        overlay
      }
    } else {
      Overlay::layer(self.overlay.clone()).host_name("arcade-modal-exit")
    }
  }

  fn backdrop(&self) -> View {
    View::new()
      .name("arcade-modal-backdrop")
      .on_click(self.on_close.clone())
      .on_pointer_down(self.on_close.clone())
      .on_key_down_event_callback(self.on_close.clone().filter_map_input(self::escape))
      .on_navigation_cancel(self.on_close.clone())
      .style(self::backdrop_style())
      .paint(self::backdrop_paint())
      .initial(StyleTarget::new().opacity(0.0))
      .animate(
        MotionTarget::new(StyleTarget::new().opacity(1.0)).transition(
          Transition::tween()
            .duration_secs(if self.reduce_motion { 0.01 } else { 0.2 })
            .ease(Easing::EaseOut),
        ),
      )
      .exit(
        MotionTarget::new(StyleTarget::new().opacity(0.0)).transition(
          Transition::tween()
            .duration_secs(if self.reduce_motion { 0.01 } else { 0.2 })
            .ease(Easing::EaseOut),
        ),
      )
      .child(
        ModalBody::new()
          .title(self.title.clone())
          .children(self.children.clone())
          .confirm_label(self.confirm_label.clone())
          .cancel_label(self.cancel_label.clone())
          .danger(self.danger)
          .close_on_escape(self.close_on_escape)
          .reduce_motion(self.reduce_motion)
          .on_confirm(self.on_confirm.clone())
          .on_close(self.on_close.clone()),
      )
  }
}

impl Component for OpenArcadeModal {
  fn render(&self) -> impl Render {
    self.overlay().child(
      AnimatePresence::new().child(
        self
          .open
          .then(|| Node::new(self.backdrop().key("arcade-modal-presence"))),
      ),
    )
  }
}

impl Component for ModalBody {
  fn render(&self) -> impl Render {
    let cancel = use_element_ref();
    let confirm = use_element_ref();
    self::panel_motion(View::new(), self.reduce_motion)
      .name("arcade-modal-panel")
      .on_click_event(|event| event.stop_propagation())
      .on_pointer_down_event(|event| event.stop_propagation())
      .style(self::panel_style())
      .child(self::panel_chrome(self.reduce_motion))
      .child(self::panel_shine(self.reduce_motion))
      .child(self.title.as_ref().map(|title| {
        TextElement::new(title.clone())
          .picking_mode(PickingMode::Ignore)
          .style(self::title_style(self.danger))
      }))
      .child(
        View::new()
          .name("arcade-modal-description")
          .style(self::description_style(self.title.is_some()))
          .child(self.children.render()),
      )
      .child(
        View::new()
          .style(self::actions_style())
          .child(self.cancel_label.as_ref().map(|label| {
            ModalButton::new()
              .label(label.clone())
              .autofocus(true)
              .reference(cancel.clone())
              .on_press(self.on_close.clone())
              .on_close(self.on_close.clone())
              .close_on_escape(self.close_on_escape)
          }))
          .child(
            ModalButton::new()
              .label(self.confirm_label.clone())
              .autofocus(self.cancel_label.is_none())
              .danger(self.danger)
              .reference(confirm)
              .on_press(self.on_confirm.clone())
              .on_close(self.on_close.clone())
              .close_on_escape(self.close_on_escape),
          ),
      )
  }
}

fn panel_motion(panel: View, reduce_motion: bool) -> View {
  if reduce_motion {
    return panel
      .initial(StyleTarget::new().opacity(0.0))
      .animate(
        MotionTarget::new(StyleTarget::new().opacity(1.0)).transition(
          Transition::tween()
            .duration_secs(0.01)
            .ease(Easing::EaseOut),
        ),
      )
      .exit(
        MotionTarget::new(StyleTarget::new().opacity(0.0)).transition(
          Transition::tween()
            .duration_secs(0.01)
            .ease(Easing::EaseOut),
        ),
      );
  }

  panel
    .initial(
      StyleTarget::new()
        .opacity(0.0)
        .scale_x(0.72)
        .scale_y(0.04)
        .x(-30.0)
        .filter(MotionFilterList::default().blur(5.0)),
    )
    .animate(
      MotionTarget::new(
        StyleTarget::new()
          .opacity_keyframes(Keyframes::new([0.0, 1.0, 0.72, 1.0]).times([0.0, 0.48, 0.72, 1.0]))
          .scale_x_keyframes(Keyframes::new([0.72, 1.04, 0.985, 1.0]).times([0.0, 0.48, 0.72, 1.0]))
          .scale_y_keyframes(Keyframes::new([0.04, 1.08, 0.97, 1.0]).times([0.0, 0.48, 0.72, 1.0]))
          .x_keyframes(Keyframes::new([-30.0, 18.0, -7.0, 0.0]).times([0.0, 0.48, 0.72, 1.0]))
          .filter_keyframes(
            Keyframes::new([
              MotionFilterList::default().blur(5.0),
              MotionFilterList::default().blur(0.0),
              MotionFilterList::default().blur(0.0),
              MotionFilterList::default().blur(0.0),
            ])
            .times([0.0, 0.48, 0.72, 1.0]),
          ),
      )
      .transition(
        Transition::tween()
          .duration_secs(0.42)
          .ease(Easing::EaseOut),
      ),
    )
    .exit(
      MotionTarget::new(
        StyleTarget::new()
          .opacity_keyframes(Keyframes::new([1.0, 0.8, 0.0]).times([0.0, 0.5, 1.0]))
          .scale_x_keyframes(Keyframes::new([1.0, 1.07, 0.78]).times([0.0, 0.5, 1.0]))
          .scale_y_keyframes(Keyframes::new([1.0, 0.82, 0.035]).times([0.0, 0.5, 1.0]))
          .x_keyframes(Keyframes::new([0.0, -18.0, 34.0]).times([0.0, 0.5, 1.0]))
          .filter_keyframes(
            Keyframes::new([
              MotionFilterList::default().blur(0.0),
              MotionFilterList::default().blur(0.0),
              MotionFilterList::default().blur(5.0),
            ])
            .times([0.0, 0.5, 1.0]),
          ),
      )
      .transition(Transition::tween().duration_secs(0.3).ease(Easing::EaseOut)),
    )
}

fn panel_chrome(reduce_motion: bool) -> View {
  let chrome = View::decorative()
    .name("arcade-modal-panel-chrome")
    .style(Style::new().position(Position::Absolute).inset(0))
    .paint(self::panel_paint());
  if reduce_motion {
    return chrome;
  }
  chrome
    .initial(
      StyleTarget::new()
        .skew_x(-7.0)
        .paint_filter(PaintFilterList::default().brightness(1.0)),
    )
    .animate(
      MotionTarget::new(
        StyleTarget::new()
          .skew_x_keyframes(Keyframes::new([-7.0, 2.5, -1.0, 0.0]).times([0.0, 0.48, 0.72, 1.0]))
          .paint_filter_keyframes(
            Keyframes::new([
              PaintFilterList::default().brightness(1.0),
              PaintFilterList::default().brightness(1.0),
              PaintFilterList::default().brightness(1.7),
              PaintFilterList::default().brightness(1.0),
            ])
            .times([0.0, 0.48, 0.72, 1.0]),
          ),
      )
      .transition(
        Transition::tween()
          .duration_secs(0.42)
          .ease(Easing::EaseOut),
      ),
    )
    .exit(
      MotionTarget::new(
        StyleTarget::new()
          .skew_x_keyframes(Keyframes::new([0.0, -3.0, 8.0]).times([0.0, 0.5, 1.0]))
          .paint_filter_keyframes(
            Keyframes::new([
              PaintFilterList::default().brightness(1.0),
              PaintFilterList::default().brightness(2.0),
              PaintFilterList::default().brightness(1.0),
            ])
            .times([0.0, 0.5, 1.0]),
          ),
      )
      .transition(Transition::tween().duration_secs(0.3).ease(Easing::EaseOut)),
    )
}

fn panel_shine(reduce_motion: bool) -> Option<View> {
  if reduce_motion {
    return None;
  }
  Some(
    View::decorative()
      .name("arcade-modal-shine")
      .style(
        Style::new()
          .position(Position::Absolute)
          .top(0)
          .bottom(0)
          .left(312)
          .width(165),
      )
      .paint(PaintStyle::new().background(self::shine_gradient()))
      .initial(StyleTarget::new().x(-190.0).skew_x(-16.0))
      .animate(
        MotionTarget::new(StyleTarget::new().x(190.0).skew_x(-16.0)).transition(
          Transition::tween()
            .duration_secs(1.8)
            .ease(Easing::Linear)
            .repeat(Repeat::Forever)
            .repeat_delay_secs(1.2),
        ),
      ),
  )
}

fn shine_gradient() -> Gradient {
  Gradient::linear(90.0)
    .stop(0.0, Color::TRANSPARENT)
    .stop(0.36, Color::TRANSPARENT)
    .stop(0.48, Color::rgba8(104, 241, 255, 20))
    .stop(0.62, Color::rgba8(255, 106, 222, 31))
    .stop(1.0, Color::TRANSPARENT)
}

impl Component for ModalButton {
  fn render(&self) -> impl Render {
    let is_present = use_is_present();
    hooks::use_effect(
      {
        let reference = self.reference.clone();
        let autofocus = self.autofocus && is_present;
        move || {
          if autofocus {
            reference.focus();
          }
        }
      },
      (self.autofocus, is_present),
    );
    match self.close_on_escape {
      true => Either::left(
        Button::content(
          TextElement::new(self.label.clone())
            .picking_mode(PickingMode::Ignore)
            .style(self::button_label_style(self.danger)),
        )
        .semantic_name(SemanticName::Text(self.label.clone()))
        .host_name(if self.danger {
          "arcade-modal-danger"
        } else {
          "arcade-modal-action"
        })
        .element_ref(self.reference.clone())
        .on_press(self.on_press.clone())
        .configure_host(|host| {
          host
            .on_key_down_event_callback(self.on_close.clone().filter_map_input(self::escape))
            .on_navigation_cancel(self.on_close.clone())
        })
        .style(self::button_style())
        .paint(self::button_paint(self.danger)),
      ),
      false => Either::right(
        Button::content(
          TextElement::new(self.label.clone())
            .picking_mode(PickingMode::Ignore)
            .style(self::button_label_style(self.danger)),
        )
        .semantic_name(SemanticName::Text(self.label.clone()))
        .host_name(if self.danger {
          "arcade-modal-danger"
        } else {
          "arcade-modal-action"
        })
        .element_ref(self.reference.clone())
        .on_press(self.on_press.clone())
        .style(self::button_style())
        .paint(self::button_paint(self.danger)),
      ),
    }
  }
}

fn escape(event: ReactantEvent<KeyEvent>) -> Option<()> {
  (event.payload().physical_key == Some(PhysicalKey::Escape)).then(|| {
    event.prevent_default();
    event.stop_propagation();
  })
}

fn backdrop_style() -> Style {
  Style::new()
    .position(Position::Absolute)
    .full_size()
    .padding((0, 75, 80, 75))
    .center_content()
    .background_color(Color::rgba8(0, 2, 10, 201))
}

fn backdrop_paint() -> PaintStyle {
  PaintStyle::new()
    .background(Color::rgba8(0, 2, 10, 201))
    .layer(PaintLayer::new(
      Gradient::radial([0.5, 0.48], [0.42, 0.42])
        .stop(0.0, Color::rgba8(18, 61, 129, 61))
        .stop(1.0, Color::TRANSPARENT),
    ))
}

fn panel_style() -> Style {
  Style::new()
    .position(Position::Relative)
    .width(790)
    .min_height(500)
    .padding((74, 70, 62, 70))
    .flex_direction(FlexDirection::Column)
    .align_items(Align::Center)
    .justify_content(Justify::Center)
    .overflow(Overflow::Hidden)
    .color(Color::hex(0xf7fbff))
}

fn panel_paint() -> PaintStyle {
  PaintStyle::new()
    .background(Color::hex(0x5cecff))
    .clip_polygon(action_skin::clip(39.5, 40.0))
    .box_shadow(vec![
      Shadow::outer(0.0, 0.0, 11.0, 0.0, Color::hex(0xaafaff)),
      Shadow::outer(0.0, 0.0, 34.0, 0.0, Color::hex(0x167dff)),
      Shadow::outer(0.0, 0.0, 55.0, 0.0, Color::hex(0xff34ca).with_alpha(0.48)),
    ])
    .layer(
      PaintLayer::new(
        Gradient::linear(55.0)
          .stop(0.0, Color::hex(0x071a3a))
          .stop(0.58, Color::hex(0x020817))
          .stop(1.0, Color::hex(0x180622)),
      )
      .bounds_inset(5.0)
      .clip_polygon(action_skin::clip(34.5, 35.0))
      .box_shadow(vec![
        Shadow::inset(0.0, 0.0, 0.0, 3.0, Color::hex(0x133f78)),
        Shadow::inset(0.0, 0.0, 58.0, 0.0, Color::BLACK),
      ]),
    )
}

fn title_style(danger: bool) -> Style {
  Style::new()
    .font_size(86)
    .letter_spacing(3)
    .white_space(WhiteSpace::NoWrap)
    .unity_font_definition(action_button::ACTION_FONT)
    .unity_text_align(TextAnchor::MiddleCenter)
    .color(Color::hex(if danger { 0xff496b } else { 0x6eeeff }))
}

fn description_style(has_title: bool) -> Style {
  Style::new()
    .max_width(620)
    .margin_top(if has_title { 42 } else { 0 })
    .font_size(47)
    .letter_spacing(1)
    .white_space(WhiteSpace::Normal)
    .unity_font_definition(action_button::ACTION_FONT)
    .unity_text_align(TextAnchor::MiddleCenter)
    .color(Color::hex(0xf5f7ff))
}

fn actions_style() -> Style {
  Style::new()
    .width(100.pct())
    .margin_top(58)
    .flex_direction(FlexDirection::Row)
    .justify_content(Justify::Center)
}

fn button_style() -> Style {
  Style::new()
    .position(Position::Relative)
    .width(250)
    .height(94)
    .margin(0)
    .margin_left(14)
    .margin_right(14)
    .padding(0)
    .border_width(0)
    .center_content()
}

fn button_label_style(danger: bool) -> Style {
  Style::new()
    .full_size()
    .font_size(56)
    .letter_spacing(2)
    .unity_font_definition(action_button::ACTION_FONT)
    .unity_text_align(TextAnchor::MiddleCenter)
    .color(Color::hex(if danger { 0xff4b69 } else { 0xeafbff }))
}

fn button_paint(danger: bool) -> PaintStyle {
  PaintStyle::new()
    .background(if danger {
      Gradient::linear(20.0)
        .stop(0.0, Color::hex(0xff6a81))
        .stop(0.56, Color::hex(0xff2454))
        .stop(1.0, Color::hex(0xff77a4))
    } else {
      Gradient::linear(20.0)
        .stop(0.0, Color::hex(0x74f5ff))
        .stop(0.55, Color::hex(0x3994ff))
        .stop(1.0, Color::hex(0xff5bd1))
    })
    .clip_polygon(action_skin::clip(18.0, 17.0))
    .paint_filter(PaintFilterList::default().drop_shadow(PaintDropShadow::new(
      0.0,
      0.0,
      11.0,
      0.0,
      if danger {
        Color::hex(0xff1f5b).with_alpha(0.75)
      } else {
        Color::hex(0x2fcdff).with_alpha(0.65)
      },
    )))
    .layer(
      PaintLayer::new(
        Gradient::linear(90.0)
          .stop(0.0, Color::hex(0x08172f))
          .stop(1.0, Color::hex(0x020713)),
      )
      .bounds_inset(4.0)
      .clip_polygon(action_skin::clip(14.0, 13.0))
      .box_shadow([Shadow::inset(0.0, 0.0, 22.0, 0.0, Color::BLACK)]),
    )
}
