//! Scrollable input bindings with conflict-safe keyboard capture.

use crate::menu::input_labels;
use battlement::{
  Align, AnimationDirection, AnimationIterations, Color, FlexDirection, Gradient, GridItem,
  GridTrack, KeyEvent, LengthUnits, PhysicalKey, Position, SemanticRole, Shadow, Sticky, Style,
  TextAnchor, WhiteSpace,
};
use reactant::{
  announcement::{Announce, use_announce},
  component::Component,
  components::Button,
  event::ReactantEvent,
  hooks,
  host::{Label, TextField},
  paint::{PaintLayer, PaintStyle},
  portal::PortalTarget,
  prelude::*,
  semantics::{SemanticName, SemanticProps},
};
use trox::{LocalizedString, opaque, tx, tx_args, txa};

use crate::menu::settings_panel;

use crate::menu::{
  action_skin,
  arcade_modal::ArcadeModal,
  font_scale,
  input_binding_icons::{
    ControllerButtonIcon, ControllerLabel, DPadIcon, InputDirection, KeyboardArrow,
  },
  setting_row::DISPLAY_FONT,
};

use crate::settings::{
  self, SettingsChange, SettingsContext,
  bindings::{self, Bindings},
};

const INPUT_WIDTH: f32 = 839.0;
const HEADER_HEIGHT: f32 = 100.0;
const ROW_HEIGHT: f32 = 159.0;

const ACTIONS: [&str; 7] = [
  "Left",
  "Right",
  "Up",
  "Down",
  "Move Piece",
  "Pause",
  "Restart",
];

/// Displays keyboard and controller bindings in a sticky-header table.
#[builder]
pub struct InputSettings {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for InputSettings {
  fn render(&self) -> impl Render {
    let set_bindings = settings::use_settings();
    let bindings = set_bindings.desired.keyboard.values();
    let (capture, set_capture) = hooks::use_state(None::<usize>);
    let (status, set_status) = hooks::use_state(None::<LocalizedString>);
    let capture_focus = use_element_ref();
    let binding_refs: [ElementRef; 7] = std::array::from_fn(|_| use_element_ref());
    let announce = use_announce();
    let font_scale = font_scale::use_font_scale();

    (
      ScrollRegion::new(tx("Input bindings", "Input bindings table label."))
        .host_name("input-bindings-scroll")
        .style(
          Style::new()
            .width(INPUT_WIDTH)
            .height(settings_panel::content_height(
              font_scale,
              set_bindings.failed || set_bindings.pending,
            ))
            .background_color(Color::rgb8(4, 17, 38)),
        )
        .child(
          Table::new(tx("Input bindings", "Input bindings table label."))
            .style(Style::new().width(INPUT_WIDTH))
            .child((
              self::header(font_scale.factor(), font_scale.factor()),
              std::array::from_fn::<_, 7, _>(|index| {
                self::binding_row(
                  index,
                  bindings[index],
                  binding_refs[index].clone(),
                  set_capture.clone(),
                  set_status.clone(),
                  font_scale.factor(),
                  font_scale.factor(),
                )
              }),
            )),
        ),
      capture.map(|index| {
        self::capture_modal(
          index,
          bindings,
          set_bindings.clone(),
          set_capture.clone(),
          set_status.clone(),
          status.clone(),
          announce,
          self.overlay.clone(),
          capture_focus.clone(),
          binding_refs[index].clone(),
        )
      }),
    )
  }
}

#[allow(clippy::too_many_arguments)]
fn capture_modal(
  index: usize,
  bindings: [PhysicalKey; 7],
  set_bindings: SettingsContext,
  set_capture: hooks::StateSetter<Option<usize>>,
  set_status: hooks::StateSetter<Option<LocalizedString>>,
  status: Option<LocalizedString>,
  announce: Announce,
  overlay: PortalTarget,
  capture_focus: ElementRef,
  restore_focus: ElementRef,
) -> impl Render {
  let scale = set_bindings.desired.text_size.factor();
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
      if bindings::is_modifier(key) {
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
          Text::new(txa(
            "Press a key for {action}",
            tx_args![action => opaque(input_labels::action(index))],
            "Keyboard capture prompt.",
          ))
          .style(self::capture_prompt_style(scale)),
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
            .style(self::waiting_marker_style(scale))
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
            Text::new(message)
              .host_name("shortcut-status")
              .style(self::status_style(scale))
          }),
        )),
    )
    .confirm_label(tx("Reset", "Reset keyboard shortcut action."))
    .cancel_label(tx("Cancel", "Cancel the current dialog."))
    .close_on_escape(false)
    .reduce_motion(false)
    .initial_focus(capture_focus)
    .restore_focus(restore_focus)
    .on_confirm(EventCallback::new({
      let set_capture = set_capture.clone();
      move |()| {
        self::apply_key(
          Bindings::<PhysicalKey>::default().values()[index],
          index,
          bindings,
          &reset_bindings,
          &set_capture,
          &reset_status,
          announce,
        );
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
        .min_height(HEADER_HEIGHT * control_scale)
        .background_color(Color::rgb8(4, 17, 38))
        .border_bottom_width(2)
        .border_bottom_color(Color::rgb8(43, 74, 123).with_alpha(0.3)),
    )
    .child(
      Grid::new()
        .columns(if font_scale > 1.0 {
          vec![GridTrack::fr(1.0), GridTrack::fr(1.0)]
        } else {
          vec![
            GridTrack::px(310.0),
            GridTrack::px(310.0),
            GridTrack::fr(1.0),
          ]
        })
        .rows(if font_scale > 1.0 {
          vec![
            GridTrack::px(65.0 * font_scale),
            GridTrack::px(65.0 * font_scale),
          ]
        } else {
          Vec::new()
        })
        .align_items(Align::Center)
        .style(if font_scale > 1.0 {
          Style::new().width(100.pct())
        } else {
          Style::new().full_size()
        })
        .child([
          ColumnHeader::new(tx("Action", "Input bindings table label."))
            .configure_host(|host| {
              host.grid_item(GridItem::new().span_columns(if font_scale > 1.0 { 2 } else { 1 }))
            })
            .style(self::heading_style(font_scale)),
          ColumnHeader::new(tx("Keyboard", "Input bindings table label."))
            .style(self::heading_style(font_scale)),
          ColumnHeader::new(tx("Controller", "Input bindings table label."))
            .style(self::heading_style(font_scale)),
        ]),
    )
}

fn binding_row(
  index: usize,
  keyboard: PhysicalKey,
  reference: ElementRef,
  set_capture: hooks::StateSetter<Option<usize>>,
  set_status: hooks::StateSetter<Option<LocalizedString>>,
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
        .min_height(ROW_HEIGHT * font_scale)
        .border_bottom_width(2)
        .border_bottom_color(Color::rgb8(43, 74, 123).with_alpha(0.25)),
    )
    .child(
      Grid::new()
        .columns(if font_scale > 1.0 {
          vec![GridTrack::fr(1.0), GridTrack::fr(1.0)]
        } else {
          vec![
            GridTrack::px(310.0),
            GridTrack::px(310.0),
            GridTrack::fr(1.0),
          ]
        })
        .rows(if font_scale > 1.0 {
          vec![
            GridTrack::px(70.0 * font_scale),
            GridTrack::px(110.0 * font_scale),
          ]
        } else {
          Vec::new()
        })
        .align_items(Align::Center)
        .style(if font_scale > 1.0 {
          Style::new().width(100.pct())
        } else {
          Style::new().full_size()
        })
        .child((
          RowHeader::new(input_labels::action(index))
            .configure_host(|host| {
              host.grid_item(GridItem::new().span_columns(if font_scale > 1.0 { 2 } else { 1 }))
            })
            .style(self::action_style(action, font_scale, control_scale)),
          self::keyboard_cell(
            index,
            keyboard,
            reference,
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
  reference: ElementRef,
  set_capture: hooks::StateSetter<Option<usize>>,
  set_status: hooks::StateSetter<Option<LocalizedString>>,
  font_scale: f32,
  control_scale: f32,
) -> impl Render {
  let name = input_labels::key(keyboard);
  let direction = self::key_direction(keyboard);
  let compact = self::key_name(keyboard).len() == 1 && direction.is_none();
  View::new()
    .semantic(SemanticProps::new(SemanticRole::Cell).name(SemanticName::Text(name.clone())))
    .name(format!("keyboard-binding-cell-{index}"))
    .style(Style::new().center_content())
    .child(
      Button::content((
        direction.map(|direction| KeyboardArrow::new().direction(direction)),
        direction.is_none().then(|| {
          Label::new(name.clone())
            .name(format!("keyboard-binding-label-{index}"))
            .style(self::keycap_label_style(
              keyboard,
              font_scale,
              control_scale,
            ))
        }),
      ))
      .semantic_name(SemanticName::Text(txa(
        "Change {action} keyboard binding",
        tx_args![action => opaque(input_labels::action(index))],
        "Keyboard binding change button.",
      )))
      .on_press(move || {
        set_status.set(None);
        set_capture.set(Some(index));
      })
      .element_ref(reference)
      .host_name(format!("keyboard-binding-{index}"))
      .style(self::keycap_style(compact, font_scale, control_scale))
      .paint(self::keycap_paint()),
    )
}

fn controller_cell(index: usize) -> impl Render {
  View::new()
    .name(format!("controller-binding-{index}"))
    .semantic(
      SemanticProps::new(SemanticRole::Cell)
        .name(SemanticName::Text(input_labels::controller(index))),
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
  set_bindings: &SettingsContext,
  set_capture: &hooks::StateSetter<Option<usize>>,
  set_status: &hooks::StateSetter<Option<LocalizedString>>,
  announce: Announce,
) {
  if let Some(conflict) = bindings
    .iter()
    .enumerate()
    .find_map(|(other, binding)| (*binding == key && other != index).then_some(other))
  {
    let message = txa(
      "Already used by {action}",
      tx_args![action => opaque(input_labels::action(conflict))],
      "Duplicate shortcut error.",
    );
    set_status.set(Some(message.clone()));
    announce.send(message);
    return;
  }
  let mut current = bindings;
  current[index] = key;
  let updated = Bindings::from_values(current);
  if !bindings::valid_keyboard(updated) {
    let message = tx(
      "Escape is reserved for Pause",
      "Reserved keyboard shortcut error.",
    );
    set_status.set(Some(message.clone()));
    announce.send(message);
    return;
  }
  set_bindings.change(SettingsChange::Keyboard(updated));
  set_status.set(None);
  set_capture.set(None);
  announce.send(txa(
    "{action} assigned to {key}",
    tx_args![action => opaque(input_labels::action(index)), key => opaque(input_labels::key(key))],
    "Accepted keyboard shortcut announcement.",
  ));
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
    .margin(0)
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
    .font_size(47.0 * font_scale)
    .white_space(WhiteSpace::Normal)
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
    .white_space(WhiteSpace::Normal)
    .unity_text_align(TextAnchor::MiddleLeft)
}

fn capture_prompt_style(scale: f32) -> Style {
  Style::new()
    .color(Color::rgb8(246, 246, 250))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(46.0 * scale)
    .white_space(WhiteSpace::Normal)
    .unity_text_align(TextAnchor::MiddleCenter)
}

fn waiting_marker_style(scale: f32) -> Style {
  Style::new()
    .width(100)
    .height(82.0 * scale)
    .margin_top(18)
    .padding(0)
    .border_width(0)
    .background_color(Color::TRANSPARENT)
    .color(Color::hex(0x5cecff))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(62.0 * scale)
    .unity_text_align(TextAnchor::MiddleCenter)
}

fn status_style(scale: f32) -> Style {
  Style::new()
    .margin_top(12)
    .color(Color::hex(0xff5ca8))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(36.0 * scale)
    .white_space(WhiteSpace::Normal)
    .unity_text_align(TextAnchor::MiddleCenter)
}
