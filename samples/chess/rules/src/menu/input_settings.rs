//! Scrollable saved bindings with exclusive physical capture.
use crate::menu::{
  font_scale,
  input_binding_icons::{
    ControllerButtonIcon, ControllerLabel, DPadIcon, InputDirection, KeyboardArrow,
  },
  input_capture_dialog::{CaptureTarget, InputCaptureDialog},
  input_devices::InputDevices,
  input_labels, input_styles, settings_panel,
};
use crate::settings::{self, bindings::ControllerBinding};
use battlement::{
  Align, Color, GridItem, GridTrack, InputCaptureDevice, LengthUnits, PhysicalKey, SemanticRole,
  Sticky, Style,
};
use reactant::{
  components::Button,
  hooks,
  host::Label,
  portal::PortalTarget,
  prelude::*,
  semantics::{SemanticName, SemanticProps},
};
use trox::{opaque, tx, tx_args, txa};

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

/// Displays bindings for currently available input devices.
#[builder]
pub struct InputSettings {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for InputSettings {
  fn render(&self) -> impl Render {
    let settings = settings::use_settings();
    let devices = InputDevices::from_host(&reactant::use_host_settings());
    let keyboard = settings.desired.keyboard.values();
    let controller = settings.desired.controller.values();
    let (capture, set_capture) = hooks::use_state(None::<CaptureTarget>);
    let keyboard_refs: [ElementRef; 7] = std::array::from_fn(|_| use_element_ref());
    let controller_refs: [ElementRef; 7] = std::array::from_fn(|_| use_element_ref());
    let font_scale = font_scale::use_font_scale();
    (
      ScrollRegion::new(tx("Input bindings", "Input bindings table label."))
        .host_name("input-bindings-scroll")
        .style(
          Style::new()
            .width(INPUT_WIDTH)
            .height(settings_panel::content_height(
              font_scale,
              settings.failed || settings.pending,
            ))
            .background_color(Color::rgb8(4, 17, 38)),
        )
        .child(
          Table::new(tx("Input bindings", "Input bindings table label."))
            .style(Style::new().width(INPUT_WIDTH))
            .child((
              self::header(font_scale.factor(), font_scale.factor(), devices),
              std::array::from_fn::<_, 7, _>(|index| {
                self::binding_row(
                  index,
                  keyboard[index],
                  controller[index],
                  controller_refs[index].clone(),
                  devices,
                  keyboard_refs[index].clone(),
                  set_capture.clone(),
                  font_scale.factor(),
                  font_scale.factor(),
                )
              }),
            )),
        ),
      capture.map(|target| {
        InputCaptureDialog::new()
          .target(target)
          .overlay(self.overlay.clone())
          .restore_focus(match target.device {
            InputCaptureDevice::Keyboard => keyboard_refs[target.index].clone(),
            InputCaptureDevice::Controller => controller_refs[target.index].clone(),
          })
          .set_capture(set_capture.clone())
      }),
    )
  }
}

fn header(font_scale: f32, control_scale: f32, devices: InputDevices) -> TableRow {
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
        .columns(devices.columns(font_scale))
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
        .child((
          ColumnHeader::new(tx("Action", "Input bindings table label."))
            .configure_host(|host| {
              host.grid_item(GridItem::new().span_columns(if font_scale > 1.0 {
                devices.count()
              } else {
                1
              }))
            })
            .style(input_styles::heading_style(font_scale)),
          devices.keyboard.then(|| {
            ColumnHeader::new(tx("Keyboard", "Input bindings table label."))
              .style(input_styles::heading_style(font_scale))
          }),
          devices.controller.then(|| {
            ColumnHeader::new(tx("Controller", "Input bindings table label."))
              .style(input_styles::heading_style(font_scale))
          }),
        )),
    )
}

#[allow(clippy::too_many_arguments)]
fn binding_row(
  index: usize,
  keyboard: PhysicalKey,
  controller: ControllerBinding,
  controller_reference: ElementRef,
  devices: InputDevices,
  reference: ElementRef,
  set_capture: hooks::StateSetter<Option<CaptureTarget>>,
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
        .columns(devices.columns(font_scale))
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
              host.grid_item(GridItem::new().span_columns(if font_scale > 1.0 {
                devices.count()
              } else {
                1
              }))
            })
            .style(input_styles::action_style(
              action,
              font_scale,
              control_scale,
            )),
          devices.keyboard.then(|| {
            self::keyboard_cell(
              index,
              keyboard,
              reference,
              set_capture.clone(),
              font_scale,
              control_scale,
            )
          }),
          devices
            .controller
            .then(|| self::controller_cell(index, controller, controller_reference, set_capture)),
        )),
    )
}

fn keyboard_cell(
  index: usize,
  keyboard: PhysicalKey,
  reference: ElementRef,
  set_capture: hooks::StateSetter<Option<CaptureTarget>>,
  font_scale: f32,
  control_scale: f32,
) -> impl Render {
  let name = input_labels::key(keyboard);
  let direction = input_styles::key_direction(keyboard);
  let compact = input_styles::key_name(keyboard).len() == 1 && direction.is_none();
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
            .style(input_styles::keycap_label_style(
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
        set_capture.set(Some(CaptureTarget {
          index,
          device: InputCaptureDevice::Keyboard,
        }));
      })
      .element_ref(reference)
      .host_name(format!("keyboard-binding-{index}"))
      .style(input_styles::keycap_style(
        compact,
        font_scale,
        control_scale,
      ))
      .paint(input_styles::keycap_paint()),
    )
}

fn controller_cell(
  index: usize,
  binding: ControllerBinding,
  reference: ElementRef,
  set_capture: hooks::StateSetter<Option<CaptureTarget>>,
) -> impl Render {
  let direction = match binding {
    ControllerBinding::DpadLeft => Some(InputDirection::Left),
    ControllerBinding::DpadRight => Some(InputDirection::Right),
    ControllerBinding::DpadUp => Some(InputDirection::Up),
    ControllerBinding::DpadDown => Some(InputDirection::Down),
    _ => None,
  };
  let label = match binding {
    ControllerBinding::South => Some(ControllerLabel::A),
    ControllerBinding::North => Some(ControllerLabel::Y),
    ControllerBinding::Start => Some(ControllerLabel::Menu),
    ControllerBinding::West => Some(ControllerLabel::X),
    ControllerBinding::LeftShoulder => Some(ControllerLabel::LeftShoulder),
    ControllerBinding::RightShoulder => Some(ControllerLabel::RightShoulder),
    ControllerBinding::Select => Some(ControllerLabel::Select),
    _ => None,
  };
  View::new()
    .name(format!("controller-binding-cell-{index}"))
    .semantic(
      SemanticProps::new(SemanticRole::Cell)
        .name(SemanticName::Text(input_labels::controller(binding))),
    )
    .style(Style::new().center_content())
    .child(
      Button::content((
        direction.map(|direction| DPadIcon::new().direction(direction)),
        label.map(|label| ControllerButtonIcon::new().label(label)),
      ))
      .semantic_name(SemanticName::Text(txa(
        "Change {action} controller binding",
        tx_args![action => opaque(input_labels::action(index))],
        "Controller binding change button.",
      )))
      .host_name(format!("controller-binding-{index}"))
      .element_ref(reference)
      .style(
        Style::new()
          .margin(0)
          .padding(0)
          .border_width(0)
          .background_color(Color::TRANSPARENT)
          .center_content(),
      )
      .on_press(move || {
        set_capture.set(Some(CaptureTarget {
          index,
          device: InputCaptureDevice::Controller,
        }))
      }),
    )
}
