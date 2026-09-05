//! Scrollable input bindings with conflict-safe keyboard capture.

use battlement::{
  AccessibilityScrollAxis, AccessibilityScrollDirection, Align, AnimationDirection,
  AnimationIterations, Color, FlexDirection, Gradient, GridTrack, KeyEvent, PhysicalKey, Position,
  ScrollerVisibility, Shadow, Sticky, Style, TextAnchor, Vector, WhiteSpace,
};
use battlement_reactant::{
  announcement::{Announce, use_announce},
  component::Component,
  event::ReactantEvent,
  hooks,
  host::{Label, TextField},
  paint::{PaintLayer, PaintStyle},
  portal::PortalTarget,
  prelude::*,
  semantics::{SemanticName, SemanticProps},
};
use trox::{ls, tx};

use crate::{
  action_skin,
  arcade_modal::ArcadeModal,
  font_scale::{self, FontScaleRole},
  input_binding_icons::{
    ControllerButtonIcon, ControllerLabel, DPadIcon, InputDirection, KeyboardArrow,
  },
  setting_row::DISPLAY_FONT,
};

const INPUT_WIDTH: f32 = 839.0;
const HEADER_HEIGHT: f32 = 100.0;
const ROW_HEIGHT: f32 = 159.0;
const SCROLL_OFFSET: f32 = 470.0;

const ACTIONS: [&str; 7] = [
  "Left",
  "Right",
  "Up",
  "Down",
  "Move Piece",
  "Pause",
  "Restart",
];
const DEFAULT_KEYBOARD: [PhysicalKey; 7] = [
  PhysicalKey::ArrowLeft,
  PhysicalKey::ArrowRight,
  PhysicalKey::ArrowUp,
  PhysicalKey::ArrowDown,
  PhysicalKey::Space,
  PhysicalKey::Escape,
  PhysicalKey::KeyR,
];
const CUSTOM_KEYBOARD: [PhysicalKey; 7] = [
  PhysicalKey::KeyA,
  PhysicalKey::KeyD,
  PhysicalKey::KeyW,
  PhysicalKey::KeyS,
  PhysicalKey::Backspace,
  PhysicalKey::Tab,
  PhysicalKey::Enter,
];
const CONTROLLER: [&str; 7] = [
  "D-pad left",
  "D-pad right",
  "D-pad up",
  "D-pad down",
  "A",
  "menu",
  "Y",
];

/// Initial binding set displayed by an input table specimen.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum InputBindingVariant {
  #[default]
  Default,
  Custom,
}

impl InputBindingVariant {
  pub(crate) fn label(self) -> &'static str {
    match self {
      Self::Default => "Default",
      Self::Custom => "Long/custom",
    }
  }

  fn bindings(self) -> [PhysicalKey; 7] {
    match self {
      Self::Default => DEFAULT_KEYBOARD,
      Self::Custom => CUSTOM_KEYBOARD,
    }
  }
}

/// Displays keyboard and controller bindings in a sticky-header table.
#[builder]
pub struct InputSettings {
  overlay: Option<PortalTarget>,
  variant: InputBindingVariant,
}

impl Component for InputSettings {
  fn render(&self) -> impl Render {
    let (scrolled, set_scrolled) = hooks::use_state(false);
    let (bindings, set_bindings) = hooks::use_state(self.variant.bindings());
    let (capture, set_capture) = hooks::use_state(None::<usize>);
    let (status, set_status) = hooks::use_state(None::<String>);
    let capture_focus = use_element_ref();
    let announce = use_announce();
    let font_scale = font_scale::use_font_scale();

    (
      ScrollArea::new(
        Some(ls("Input bindings")),
        AccessibilityScrollAxis::Vertical,
        !scrolled,
        scrolled,
      )
      .on_scroll(move |direction| {
        set_scrolled.set(direction == AccessibilityScrollDirection::Forward)
      })
      .host_name("input-bindings-scroll")
      .configure_host(|host| {
        host
          .scroll_offset(Vector::new(
            0.0,
            if scrolled {
              if font_scale.factor() == 1.0 {
                SCROLL_OFFSET
              } else {
                HEADER_HEIGHT * font_scale.dynamic(FontScaleRole::Control)
                  + 7.0 * ROW_HEIGHT * font_scale.factor()
                  - 720.0
              }
            } else {
              0.0
            },
          ))
          .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
          .vertical_scroller_visibility(ScrollerVisibility::Auto)
          .content_container_style(Style::new().align_items(Align::Center))
      })
      .style(
        Style::new()
          .width(INPUT_WIDTH)
          .height(720)
          .margin_top(48)
          .background_color(Color::rgb8(4, 17, 38)),
      )
      .child(
        Table::new(ls("Input bindings"))
          .style(Style::new().width(INPUT_WIDTH))
          .child((
            self::header(
              font_scale.factor(),
              font_scale.dynamic(FontScaleRole::Control),
            ),
            std::array::from_fn::<_, 7, _>(|index| {
              self::binding_row(
                index,
                bindings[index],
                self.overlay.is_some(),
                set_capture.clone(),
                set_status.clone(),
                font_scale.factor(),
                font_scale.dynamic(FontScaleRole::Control),
              )
            }),
          )),
      ),
      capture.and_then(|index| {
        self.overlay.clone().map(|overlay| {
          self::capture_modal(
            index,
            bindings,
            set_bindings.clone(),
            set_capture.clone(),
            set_status.clone(),
            status.clone(),
            announce,
            overlay,
            capture_focus.clone(),
          )
        })
      }),
    )
  }
}

#[allow(clippy::too_many_arguments)]
fn capture_modal(
  index: usize,
  bindings: [PhysicalKey; 7],
  set_bindings: hooks::StateSetter<[PhysicalKey; 7]>,
  set_capture: hooks::StateSetter<Option<usize>>,
  set_status: hooks::StateSetter<Option<String>>,
  status: Option<String>,
  announce: Announce,
  overlay: PortalTarget,
  capture_focus: ElementRef,
) -> impl Render {
  let action = ACTIONS[index];
  let close_capture = set_capture.callback().map_input(|_| None);
  let reset_bindings = set_bindings.clone();
  let reset_status = set_status.clone();
  let capture_key = EventCallback::new({
    let set_bindings = set_bindings.clone();
    let set_capture = set_capture.clone();
    let set_status = set_status.clone();
    move |event: ReactantEvent<KeyEvent>| {
      let Some(key) = self::captured_key(event.payload()) else {
        return;
      };
      if self::is_bare_modifier(key) {
        return;
      }
      event.prevent_default();
      event.stop_propagation();
      self::apply_key(
        key,
        index,
        bindings,
        &set_bindings,
        &set_capture,
        &set_status,
        announce,
      );
    }
  });
  ArcadeModal::new()
    .open(true)
    .title(tx("Change Shortcut", "Keyboard shortcut dialog title."))
    .children(
      View::new()
        .style(
          Style::new()
            .width(650)
            .flex_direction(FlexDirection::Column)
            .align_items(Align::Center),
        )
        .child((
          Text::new(ls(format!("Press a key for {action}"))).style(self::capture_prompt_style()),
          TextField::new()
            .value("●")
            .select_all_on_focus(false)
            .select_all_on_mouse_up(false)
            .cursor_index(0)
            .select_index(0)
            .name("shortcut-waiting-marker")
            .element_ref(capture_focus.clone())
            .focusable(true)
            .tab_index(0)
            .on_key_down_event_callback(capture_key.clone())
            .on_change_value({
              let set_bindings = set_bindings.clone();
              let set_capture = set_capture.clone();
              let set_status = set_status.clone();
              move |value: String| {
                if let Some(key) = self::text_key(&value) {
                  self::apply_key(
                    key,
                    index,
                    bindings,
                    &set_bindings,
                    &set_capture,
                    &set_status,
                    announce,
                  );
                }
              }
            })
            .on_navigation_cancel({
              let set_bindings = set_bindings.clone();
              let set_capture = set_capture.clone();
              let set_status = set_status.clone();
              move || {
                self::apply_key(
                  PhysicalKey::Escape,
                  index,
                  bindings,
                  &set_bindings,
                  &set_capture,
                  &set_status,
                  announce,
                );
              }
            })
            .semantic(
              SemanticProps::new(SemanticRole::StaticText).name(SemanticName::Text(tx(
                "Waiting for keyboard input",
                "Keyboard shortcut capture status.",
              ))),
            )
            .style(self::waiting_marker_style())
            .input_style(
              Style::new()
                .padding(0)
                .border_width(0)
                .background_color(Color::TRANSPARENT),
            )
            .text_element_style(Style::new().padding(0).background_color(Color::TRANSPARENT))
            .animation(
              Animation::new(Keyframes::new([
                StyleTarget::new().opacity(1.0),
                StyleTarget::new().opacity(0.22),
              ]))
              .duration_secs(0.72)
              .iterations(AnimationIterations::Forever)
              .direction(AnimationDirection::Alternate)
              .animation_key("shortcut-waiting-blink"),
            ),
          status.map(|message| {
            Text::new(ls(message))
              .host_name("shortcut-status")
              .style(self::status_style())
          }),
        )),
    )
    .confirm_label(tx("Reset", "Reset keyboard shortcut action."))
    .cancel_label(tx("Cancel", "Cancel keyboard shortcut capture."))
    .close_on_escape(false)
    .reduce_motion(false)
    .initial_focus(capture_focus)
    .on_confirm(EventCallback::new({
      let set_capture = set_capture.clone();
      move |()| {
        reset_bindings.update(move |mut current| {
          current[index] = DEFAULT_KEYBOARD[index];
          current
        });
        reset_status.set(None);
        set_capture.set(None);
        announce.send(ls(format!("{action} reset to default")));
      }
    }))
    .on_close(
      close_capture
        .clone()
        .then(set_status.callback().map_input(|_| None)),
    )
    .overlay(overlay)
}

fn header(font_scale: f32, control_scale: f32) -> TableRow {
  TableRow::new()
    .host_name("input-bindings-header")
    .configure_host(|host| host.sticky(Sticky::top(0.0).order(4)))
    .style(
      Style::new()
        .width(INPUT_WIDTH)
        .height(HEADER_HEIGHT * control_scale)
        .background_color(Color::rgb8(4, 17, 38))
        .border_bottom_width(2)
        .border_bottom_color(Color::rgb8(43, 74, 123).with_alpha(0.3)),
    )
    .child(
      Grid::new()
        .columns([
          GridTrack::px(if font_scale > 1.0 { 260.0 } else { 310.0 }),
          GridTrack::px(if font_scale > 1.0 { 340.0 } else { 310.0 }),
          GridTrack::fr(1.0),
        ])
        .align_items(Align::Center)
        .style(Style::new().full_size())
        .child([
          ColumnHeader::new(ls("Action")).style(self::heading_style(font_scale)),
          ColumnHeader::new(ls("Keyboard")).style(self::heading_style(font_scale)),
          ColumnHeader::new(ls("Controller")).style(self::heading_style(font_scale)),
        ]),
    )
}

fn binding_row(
  index: usize,
  keyboard: PhysicalKey,
  interactive: bool,
  set_capture: hooks::StateSetter<Option<usize>>,
  set_status: hooks::StateSetter<Option<String>>,
  font_scale: f32,
  control_scale: f32,
) -> TableRow {
  let action = ACTIONS[index];
  TableRow::new()
    .host_name(format!(
      "input-binding-{}",
      action.to_ascii_lowercase().replace(' ', "-")
    ))
    .style(
      Style::new()
        .width(INPUT_WIDTH)
        .height(ROW_HEIGHT * font_scale)
        .border_bottom_width(2)
        .border_bottom_color(Color::rgb8(43, 74, 123).with_alpha(0.25)),
    )
    .child(
      Grid::new()
        .columns([
          GridTrack::px(if font_scale > 1.0 { 260.0 } else { 310.0 }),
          GridTrack::px(if font_scale > 1.0 { 340.0 } else { 310.0 }),
          GridTrack::fr(1.0),
        ])
        .align_items(Align::Center)
        .style(Style::new().full_size())
        .child((
          RowHeader::new(ls(action)).style(self::action_style(action, font_scale, control_scale)),
          self::keyboard_cell(
            index,
            keyboard,
            interactive,
            set_capture,
            set_status,
            font_scale,
            control_scale,
          ),
          self::controller_cell(index),
        )),
    )
}

fn keyboard_cell(
  index: usize,
  keyboard: PhysicalKey,
  interactive: bool,
  set_capture: hooks::StateSetter<Option<usize>>,
  set_status: hooks::StateSetter<Option<String>>,
  font_scale: f32,
  control_scale: f32,
) -> impl Render {
  let name = ls(self::key_name(keyboard));
  let direction = self::key_direction(keyboard);
  let compact = self::key_name(keyboard).len() == 1 && direction.is_none();
  let host = View::new()
    .semantic(SemanticProps::new(SemanticRole::Cell).name(SemanticName::Text(name.clone())))
    .name(format!("keyboard-binding-{index}"))
    .style(self::keycap_style(compact, font_scale, control_scale))
    .paint(self::keycap_paint())
    .child((
      direction.map(|direction| KeyboardArrow::new().direction(direction)),
      direction.is_none().then(|| {
        Label::new(name)
          .name(format!("keyboard-binding-label-{index}"))
          .style(self::keycap_label_style(
            keyboard,
            font_scale,
            control_scale,
          ))
      }),
    ))
    .focusable(interactive)
    .tab_index(if interactive { 0 } else { -1 });
  if interactive {
    host.on_click(move || {
      set_status.set(None);
      set_capture.set(Some(index));
    })
  } else {
    host
  }
}

fn controller_cell(index: usize) -> impl Render {
  View::new()
    .name(format!("controller-binding-{index}"))
    .semantic(
      SemanticProps::new(SemanticRole::Cell).name(SemanticName::Text(ls(CONTROLLER[index]))),
    )
    .style(Style::new().center_content())
    .child((
      (index == 0).then(|| DPadIcon::new().direction(InputDirection::Left)),
      (index == 1).then(|| DPadIcon::new().direction(InputDirection::Right)),
      (index == 2).then(|| DPadIcon::new().direction(InputDirection::Up)),
      (index == 3).then(|| DPadIcon::new().direction(InputDirection::Down)),
      (index == 4).then(|| ControllerButtonIcon::new().label(ControllerLabel::A)),
      (index == 5).then(|| ControllerButtonIcon::new().label(ControllerLabel::Menu)),
      (index == 6).then(|| ControllerButtonIcon::new().label(ControllerLabel::Y)),
    ))
}

fn is_bare_modifier(key: PhysicalKey) -> bool {
  matches!(
    key,
    PhysicalKey::ShiftLeft
      | PhysicalKey::ShiftRight
      | PhysicalKey::ControlLeft
      | PhysicalKey::ControlRight
      | PhysicalKey::AltLeft
      | PhysicalKey::AltRight
      | PhysicalKey::MetaLeft
      | PhysicalKey::MetaRight
  )
}

fn captured_key(event: &KeyEvent) -> Option<PhysicalKey> {
  event.physical_key.or_else(|| self::text_key(&event.text))
}

fn text_key(text: &str) -> Option<PhysicalKey> {
  match text
    .chars()
    .next_back()
    .map(|character| character.to_ascii_uppercase())
  {
    Some('A') => Some(PhysicalKey::KeyA),
    Some('B') => Some(PhysicalKey::KeyB),
    Some('C') => Some(PhysicalKey::KeyC),
    Some('D') => Some(PhysicalKey::KeyD),
    Some('E') => Some(PhysicalKey::KeyE),
    Some('F') => Some(PhysicalKey::KeyF),
    Some('G') => Some(PhysicalKey::KeyG),
    Some('H') => Some(PhysicalKey::KeyH),
    Some('I') => Some(PhysicalKey::KeyI),
    Some('J') => Some(PhysicalKey::KeyJ),
    Some('K') => Some(PhysicalKey::KeyK),
    Some('L') => Some(PhysicalKey::KeyL),
    Some('M') => Some(PhysicalKey::KeyM),
    Some('N') => Some(PhysicalKey::KeyN),
    Some('O') => Some(PhysicalKey::KeyO),
    Some('P') => Some(PhysicalKey::KeyP),
    Some('Q') => Some(PhysicalKey::KeyQ),
    Some('R') => Some(PhysicalKey::KeyR),
    Some('S') => Some(PhysicalKey::KeyS),
    Some('T') => Some(PhysicalKey::KeyT),
    Some('U') => Some(PhysicalKey::KeyU),
    Some('V') => Some(PhysicalKey::KeyV),
    Some('W') => Some(PhysicalKey::KeyW),
    Some('X') => Some(PhysicalKey::KeyX),
    Some('Y') => Some(PhysicalKey::KeyY),
    Some('Z') => Some(PhysicalKey::KeyZ),
    _ => None,
  }
}

#[allow(clippy::too_many_arguments)]
fn apply_key(
  key: PhysicalKey,
  index: usize,
  bindings: [PhysicalKey; 7],
  set_bindings: &hooks::StateSetter<[PhysicalKey; 7]>,
  set_capture: &hooks::StateSetter<Option<usize>>,
  set_status: &hooks::StateSetter<Option<String>>,
  announce: Announce,
) {
  if let Some(conflict) = bindings
    .iter()
    .enumerate()
    .find_map(|(other, binding)| (*binding == key && other != index).then_some(other))
  {
    let message = format!("Already used by {}", ACTIONS[conflict]);
    set_status.set(Some(message.clone()));
    announce.send(ls(message));
    return;
  }
  set_bindings.update(move |mut current| {
    current[index] = key;
    current
  });
  set_status.set(None);
  set_capture.set(None);
  announce.send(ls(format!(
    "{} assigned to {}",
    ACTIONS[index],
    self::key_name(key)
  )));
}

fn key_name(key: PhysicalKey) -> String {
  match key {
    PhysicalKey::Escape => "Esc".to_owned(),
    PhysicalKey::Space => "Space".to_owned(),
    PhysicalKey::ArrowLeft => "Left arrow".to_owned(),
    PhysicalKey::ArrowRight => "Right arrow".to_owned(),
    PhysicalKey::ArrowUp => "Up arrow".to_owned(),
    PhysicalKey::ArrowDown => "Down arrow".to_owned(),
    _ => {
      let name = format!("{key:?}");
      name
        .strip_prefix("Key")
        .or_else(|| name.strip_prefix("Digit"))
        .unwrap_or(&name)
        .to_owned()
    }
  }
}

fn key_direction(key: PhysicalKey) -> Option<InputDirection> {
  match key {
    PhysicalKey::ArrowLeft => Some(InputDirection::Left),
    PhysicalKey::ArrowRight => Some(InputDirection::Right),
    PhysicalKey::ArrowUp => Some(InputDirection::Up),
    PhysicalKey::ArrowDown => Some(InputDirection::Down),
    _ => None,
  }
}

fn keycap_style(compact: bool, font_scale: f32, control_scale: f32) -> Style {
  Style::new()
    .position(Position::Relative)
    .width(if compact { 120.0 } else { 205.0 } * control_scale)
    .height(75.0 * control_scale)
    .padding(3.0 * control_scale)
    .border_width(0)
    .align_self(Align::Center)
    .center_content()
    .color(Color::hex(0xf6f6fa))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(49.0 * font_scale)
}

fn keycap_paint() -> PaintStyle {
  PaintStyle::new()
    .background(
      Gradient::linear(110.0)
        .stop(0.0, Color::hex(0x55f1ff))
        .stop(0.54, Color::hex(0x7ba3ff))
        .stop(1.0, Color::hex(0xff48c6)),
    )
    .paint_filter(PaintFilterList::default().drop_shadow(PaintDropShadow::new(
      0.0,
      0.0,
      7.0,
      0.0,
      Color::rgba8(42, 103, 255, 117),
    )))
    .clip_polygon(action_skin::clip(10.0, 10.0))
    .layer(
      PaintLayer::new(
        Gradient::linear(180.0)
          .stop(0.0, Color::hex(0x050b1c))
          .stop(1.0, Color::hex(0x020611)),
      )
      .bounds_inset(3.0)
      .box_shadow([Shadow::inset(0.0, 0.0, 22.0, 0.0, Color::BLACK)])
      .clip_polygon(action_skin::clip(7.0, 7.0)),
    )
}

fn keycap_label_style(keyboard: PhysicalKey, font_scale: f32, control_scale: f32) -> Style {
  let value = self::key_name(keyboard);
  Style::new()
    .position(Position::Relative)
    .full_size()
    .color(Color::hex(0xf6f6fa))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(
      if value.len() > 2 { 49.0 } else { 60.0 }
        * if value.len() > 2 {
          control_scale
        } else {
          font_scale
        },
    )
    .letter_spacing(if value.len() > 2 { 1.0 } else { 0.0 })
    .unity_text_align(TextAnchor::MiddleCenter)
}

fn heading_style(font_scale: f32) -> Style {
  Style::new()
    .color(Color::rgb8(244, 245, 250))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(47.0 * (1.0 + (font_scale - 1.0) * 0.2))
    .letter_spacing(1.2)
    .unity_text_align(TextAnchor::MiddleCenter)
}

fn action_style(action: &str, font_scale: f32, control_scale: f32) -> Style {
  Style::new()
    .padding_left(18)
    .color(Color::rgb8(245, 245, 248))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(
      54.0
        * if action.len() >= 7 {
          control_scale
        } else {
          font_scale
        },
    )
    .letter_spacing(1.3)
    .unity_text_align(TextAnchor::MiddleLeft)
}

fn capture_prompt_style() -> Style {
  Style::new()
    .color(Color::rgb8(246, 246, 250))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(46)
    .white_space(WhiteSpace::Normal)
    .unity_text_align(TextAnchor::MiddleCenter)
}

fn waiting_marker_style() -> Style {
  Style::new()
    .width(100)
    .height(82)
    .margin_top(18)
    .padding(0)
    .border_width(0)
    .background_color(Color::TRANSPARENT)
    .color(Color::hex(0x5cecff))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(62)
    .unity_text_align(TextAnchor::MiddleCenter)
}

fn status_style() -> Style {
  Style::new()
    .margin_top(12)
    .color(Color::hex(0xff5ca8))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(36)
    .white_space(WhiteSpace::Normal)
    .unity_text_align(TextAnchor::MiddleCenter)
}
