use crate::{Game, design_system};
use battlement::{
  Align, Color, FlexDirection, FlexWrap, LengthUnits, MotionGestureEvent, MotionGestureEventKind,
  MotionPointerDevice, ScrollViewMode, ScrollerVisibility, Style,
};
use battlement_reactant::{motion_value::MotionValue, prelude::*};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GesturesDragState {
  device: MotionPointerDevice,
  boundary: &'static str,
  trace: Vec<&'static str>,
}

impl Default for GesturesDragState {
  fn default() -> Self {
    Self {
      device: MotionPointerDevice::Mouse,
      boundary: "READY",
      trace: vec!["native recognizers mounted"],
    }
  }
}

impl GesturesDragState {
  fn record(&mut self, event: &MotionGestureEvent) {
    self.device = event.device;
    self.boundary = match event.kind {
      MotionGestureEventKind::HoverStart => "HOVER",
      MotionGestureEventKind::HoverEnd => "HOVER END",
      MotionGestureEventKind::TapStart => "TAP START",
      MotionGestureEventKind::Tap => "TAP",
      MotionGestureEventKind::TapCancel => "TAP CANCEL",
      MotionGestureEventKind::FocusStart => "FOCUS",
      MotionGestureEventKind::FocusEnd => "FOCUS END",
      MotionGestureEventKind::PanSessionStart => "PAN SESSION",
      MotionGestureEventKind::PanStart => "PAN START",
      MotionGestureEventKind::PanEnd => "PAN END",
      MotionGestureEventKind::PanCancel => "PAN CANCEL",
      MotionGestureEventKind::DragStart => "DRAG START",
      MotionGestureEventKind::DragDirectionLock => "DIRECTION LOCK",
      MotionGestureEventKind::DragEnd => "DRAG END",
      MotionGestureEventKind::DragCancel => "DRAG CANCEL",
      MotionGestureEventKind::DragMomentumComplete => "MOMENTUM COMPLETE",
      MotionGestureEventKind::DragConstraintsMeasured => "CONSTRAINTS",
      MotionGestureEventKind::InViewEnter => "IN VIEW",
      MotionGestureEventKind::InViewLeave => "OUT OF VIEW",
      MotionGestureEventKind::Pan
      | MotionGestureEventKind::Drag
      | MotionGestureEventKind::Scroll => {
        return;
      }
    };
    self.trace.push(self.boundary);
    if self.trace.len() > 7 {
      self.trace.remove(0);
    }
  }
}

#[builder]
pub(crate) struct GesturesDrag {
  pub(crate) state: GesturesDragState,
  pub(crate) compact: bool,
}

impl Component for GesturesDrag {
  fn render(&self) -> impl Render {
    let drag_x = use_motion_value(0.0_f32);
    let drag_y = use_motion_value(0.0_f32);
    let scroll_x = use_motion_value(0.0_f32);
    let scroll_y = use_motion_value(0.0_f32);
    let in_view = use_motion_value(0.0_f32);
    let drag_energy = use_transform(
      drag_x.clone(),
      InputRange::new([-120.0, 0.0, 120.0]),
      OutputRange::new([0.85, 1.0, 1.15]),
    );
    let scroll_progress = use_transform(
      scroll_y.clone(),
      InputRange::new([0.0, 220.0]),
      OutputRange::new([0.0, 1.0]),
    );
    let controls = use_drag_controls();
    let drag_gallery = drag_gallery(
      constrained_drag(drag_x, drag_y),
      momentum_drag(),
      external_drag(controls.clone(), controls),
      drag_energy,
    );
    let value_gallery = value_gallery(
      scroll_specimen(scroll_x, scroll_y),
      scroll_meter(scroll_progress),
      in_view_specimen(in_view),
    );

    ScrollView::new()
      .name("gestures-drag-canvas")
      .mode(ScrollViewMode::Vertical)
      .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
      .vertical_scroller_visibility(ScrollerVisibility::Auto)
      .style(design_system::canvas(self.compact).padding(0.0))
      .content_container_style(content())
      .child(Label::new("UNITY-LOCAL INPUT · RELIABLE BOUNDARIES").style(eyebrow()))
      .child(
        Label::new("Gestures & Drag")
          .name("page-title")
          .style(title()),
      )
      .child(
        Label::new(format!(
          "DEVICE {} · {} · CAPTURE + COALESCING ACTIVE",
          device_name(self.state.device),
          self.state.boundary,
        ))
        .name("gesture-device-state")
        .style(status()),
      )
      .child(device_indicators(self.state.device))
      .child(drag_gallery)
      .child(value_gallery)
      .child(
        Label::new(format!("TRACE  {}", self.state.trace.join("  ›  ")))
          .name("gestures-trace")
          .style(trace()),
      )
  }
}

fn constrained_drag(drag_x: MotionValue<f32>, drag_y: MotionValue<f32>) -> Node {
  let target = Node::new(
    View::new()
      .name("constrained-drag-target")
      .style(knob())
      .drag(DragAxis::Both)
      .drag_constraints(DragConstraints::bounds(-115.0, 115.0, -42.0, 42.0))
      .drag_elastic(DragElastic::axes(0.16, 0.1))
      .drag_direction_lock(true)
      .drag_transition(
        DragTransition::new()
          .velocity_retention(0.025)
          .rest_speed(6.0),
      )
      .drag_motion_values(drag_x, drag_y)
      .while_hover(MotionStyle::new().scale(1.05))
      .while_tap(MotionStyle::new().scale(0.94))
      .while_drag(MotionStyle::new().scale(1.08))
      .on_hover_start(record)
      .on_hover_end(record)
      .on_tap_start(record)
      .on_tap(record)
      .on_tap_cancel(record)
      .on_focus_start(record)
      .on_focus_end(record)
      .on_drag_start(record)
      .on_drag_direction_lock(record)
      .on_drag_end(record)
      .on_drag_cancel(record)
      .on_drag_momentum_complete(record)
      .child(Label::new("DRAG").style(knob_label())),
  );
  Node::new(
    View::new()
      .name("gesture-threshold-guide")
      .style(field())
      .child(Label::new("3 PX START · 10 PX LOCK").style(caption()))
      .child(target),
  )
}

fn momentum_drag() -> Node {
  let target = Node::new(
    View::new()
      .style(knob())
      .drag(DragAxis::X)
      .drag_constraints(DragConstraints::bounds(-110.0, 110.0, 0.0, 0.0))
      .drag_momentum(true)
      .drag_snap_to_origin(DragAxis::X)
      .while_drag(MotionStyle::new().scale(1.1))
      .on_drag_start(record)
      .on_drag_end(record)
      .on_drag_momentum_complete(record)
      .child(Label::new("MOMENTUM").style(knob_label())),
  );
  Node::new(
    View::new()
      .name("momentum-catch-field")
      .style(field())
      .child(Label::new("THROW · CATCH · RELEASE").style(caption()))
      .child(target),
  )
}

fn external_drag(external_controls: DragControls, controls: DragControls) -> Node {
  let target = Node::new(
    View::new()
      .name("external-drag-target")
      .style(knob())
      .drag(DragAxis::Both)
      .drag_listener(false)
      .drag_controls(controls)
      .drag_constraints(DragConstraints::bounds(-110.0, 110.0, -40.0, 40.0))
      .while_drag(MotionStyle::new().scale(1.08))
      .on_drag_start(record)
      .on_drag_end(record)
      .on_drag_cancel(record)
      .child(Label::new("TARGET").style(knob_label())),
  );
  Node::new(
    View::new()
      .name("external-drag-control")
      .style(field())
      .pan(true)
      .on_pan_session_start(move |_game: &mut Game, event| {
        external_controls.start(event, DragStartOptions::default().snap_to_cursor(true));
      })
      .on_pan_start(record)
      .on_pan_end(record)
      .on_pan_cancel(record)
      .child(Label::new("EXTERNAL HANDLE").style(caption()))
      .child(target),
  )
}

fn drag_gallery(
  constrained: Node,
  momentum: Node,
  external: Node,
  energy: MotionValue<f32>,
) -> Node {
  let meter = Node::new(
    View::new()
      .name("drag-motion-value-meter")
      .style(field())
      .animate(MotionStyle::new().scale_value(energy))
      .child(Label::new("MOTION VALUE OUTPUT").style(caption())),
  );
  Node::new(
    View::new()
      .style(gallery())
      .child(constrained)
      .child(momentum)
      .child(external)
      .child(meter),
  )
}

fn scroll_specimen(scroll_x: MotionValue<f32>, scroll_y: MotionValue<f32>) -> Node {
  Node::new(
    ScrollView::new()
      .name("gesture-scroll-progress")
      .mode(ScrollViewMode::Vertical)
      .style(scroll_field())
      .content_container_style(scroll_content())
      .scroll_motion_values(scroll_x, scroll_y)
      .child(Label::new("SCROLL  ↓").style(caption()))
      .child(View::new().style(spacer()))
      .child(Label::new("SCROLL  ↑").style(caption())),
  )
}

fn scroll_meter(progress: MotionValue<f32>) -> Node {
  Node::new(
    View::new()
      .name("scroll-progress-meter")
      .style(field())
      .animate(MotionStyle::new().opacity_value(progress))
      .child(Label::new("SCROLL PROGRESS").style(caption())),
  )
}

fn in_view_specimen(in_view: MotionValue<f32>) -> Node {
  Node::new(
    View::new()
      .name("gesture-in-view-specimen")
      .style(field())
      .in_view_motion_value(in_view.clone())
      .while_in_view(MotionStyle::new().scale(1.08).opacity(1.0))
      .animate(MotionStyle::new().scale_value(in_view).opacity(0.45))
      .on_viewport_enter(record)
      .on_viewport_leave(record)
      .child(Label::new("IN-VIEW VALUE").style(caption())),
  )
}

fn value_gallery(scroll: Node, meter: Node, in_view: Node) -> Node {
  Node::new(
    View::new()
      .style(gallery())
      .child(scroll)
      .child(meter)
      .child(in_view),
  )
}

fn record(game: &mut Game, event: &MotionGestureEvent) {
  game.gestures_drag.record(event);
}

fn device_indicators(active: MotionPointerDevice) -> View {
  let devices = [
    (MotionPointerDevice::Mouse, "MOUSE"),
    (MotionPointerDevice::Pen, "PEN"),
    (MotionPointerDevice::Touch, "TOUCH"),
    (MotionPointerDevice::Keyboard, "KEYBOARD"),
    (MotionPointerDevice::Gamepad, "GAMEPAD"),
  ];
  View::new().style(indicator_row()).child(Fragment::new(
    devices
      .into_iter()
      .map(|(device, label)| Label::new(label).style(indicator(device == active)))
      .collect::<Vec<_>>(),
  ))
}

fn device_name(value: MotionPointerDevice) -> &'static str {
  match value {
    MotionPointerDevice::Mouse => "MOUSE",
    MotionPointerDevice::Pen => "PEN",
    MotionPointerDevice::Touch => "TOUCH",
    MotionPointerDevice::Keyboard => "KEYBOARD",
    MotionPointerDevice::Gamepad => "GAMEPAD",
  }
}

fn content() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .padding(28.0)
    .align_items(Align::FlexStart)
}

fn eyebrow() -> Style {
  Style::new()
    .font_size(18.0)
    .color(Color::rgb(0.98, 0.4, 0.16))
}

fn title() -> Style {
  Style::new()
    .font_size(40.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
    .margin((6, 0, 10, 0))
}

fn status() -> Style {
  Style::new()
    .font_size(15.0)
    .color(Color::rgb(0.55, 0.82, 0.78))
}

fn indicator_row() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin((10, 0))
}

fn indicator(active: bool) -> Style {
  Style::new()
    .margin((0, 6, 5, 0))
    .padding((5, 9))
    .font_size(11.0)
    .background_color(if active {
      Color::rgb(0.08, 0.42, 0.44)
    } else {
      Color::rgb(0.035, 0.09, 0.115)
    })
    .color(Color::rgb(0.9, 0.97, 0.98))
}

fn gallery() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin((8, 0))
}

fn field() -> Style {
  Style::new()
    .width(260.0)
    .height(132.0)
    .margin((0, 10, 10, 0))
    .align_items(Align::Center)
    .background_color(Color::rgb(0.035, 0.09, 0.115))
    .border_color(Color::rgb(0.18, 0.38, 0.42))
    .border_width(1.0)
}

fn knob() -> Style {
  Style::new()
    .width(78.0)
    .height(50.0)
    .margin((18, 0, 0, 0))
    .align_items(Align::Center)
    .background_color(Color::rgb(0.08, 0.48, 0.52))
    .border_color(Color::rgb(0.45, 0.98, 0.94))
    .border_width(1.0)
}

fn knob_label() -> Style {
  Style::new()
    .font_size(11.0)
    .color(Color::rgb(0.96, 1.0, 1.0))
}

fn caption() -> Style {
  Style::new()
    .font_size(12.0)
    .color(Color::rgb(0.78, 0.9, 0.91))
    .margin((10, 0, 0, 0))
}

fn scroll_field() -> Style {
  field().height(150.0)
}

fn scroll_content() -> Style {
  Style::new()
    .align_items(Align::Center)
    .width(100.0_f32.pct())
}

fn spacer() -> Style {
  Style::new()
    .height(280.0)
    .width(2.0)
    .background_color(Color::rgb(0.1, 0.75, 0.72))
}

fn trace() -> Style {
  Style::new()
    .font_size(13.0)
    .color(Color::rgb(0.64, 0.72, 0.75))
    .margin((8, 0, 20, 0))
}
