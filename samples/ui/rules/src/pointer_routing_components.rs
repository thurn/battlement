use battlement::{
  ObjectId, UiBox, UiButton, UiElement, UiEventKind, UiEventPhase, UiEventSubscription, UiLabel,
  UiNode, UiVisualElement, object_id,
};
use battlement_native::UiEventActionView;
use std::fmt;

use crate::{design_system, native_ui::NativeUiResponseBuilder, pointer_routing_styles};

pub(crate) const ROOT_ROUTE_ID: ObjectId = object_id!("22100000-0000-4000-8000-000000000001");
pub(crate) const PANEL_ROUTE_ID: ObjectId = object_id!("22100000-0000-4000-8000-000000000002");
pub(crate) const TARGET_ROUTE_ID: ObjectId = object_id!("22100000-0000-4000-8000-000000000003");
pub(crate) const PAYLOAD_ID: ObjectId = object_id!("22100000-0000-4000-8000-000000000004");
pub(crate) const CAPTURE_ID: ObjectId = object_id!("22100000-0000-4000-8000-000000000005");
const ROUTE_STEPS: [ObjectId; 5] = [
  object_id!("22100000-0000-4000-8000-000000000010"),
  object_id!("22100000-0000-4000-8000-000000000011"),
  object_id!("22100000-0000-4000-8000-000000000012"),
  object_id!("22100000-0000-4000-8000-000000000013"),
  object_id!("22100000-0000-4000-8000-000000000014"),
];

pub(crate) fn page(page_id: ObjectId) -> UiNode {
  UiNode::new(page_id, UiVisualElement::new().name("pointer-routing-page"))
        .child(node(UiLabel::new("POINTER ROUTING").style(design_system::eyebrow())))
        .child(node(UiLabel::new("One native event. One Rust action.").style(design_system::title())))
        .child(node(UiLabel::new("Press and drag on the target. Rust reconstructs the five-step logical route while Unity reports one complete pointer payload and capture lifecycle.").style(pointer_routing_styles::intro())))
        .child(node(UiVisualElement::new().style(pointer_routing_styles::columns()))
            .child(route_card()).child(inspector()))
}

pub(crate) fn write_event_response(
  event: UiEventActionView<'_>,
  response: &mut NativeUiResponseBuilder,
) -> Result<bool, battlement_native::EngineError> {
  let target_id =
    ObjectId::from_bytes(event.target_id()).expect("UI event view validates target UUIDs");
  if target_id != TARGET_ROUTE_ID {
    return Ok(false);
  }
  let (payload, captured) = match event.event_kind() {
    UiEventKind::PointerDown => {
      let value = event.pointer_button().expect("pointer-down body");
      (
        Some(format!(
          "POINTER DOWN\nposition  {:.0}, {:.0}\ndelta     {:.0}, {:.0}\npointer   {} · {:?}\nbutton    {:?}\nbuttons   {}\npressure  {:.2}\nclicks    {}\nmodifiers {:?}",
          value.position.0,
          value.position.1,
          value.delta.0,
          value.delta.1,
          value.pointer_id,
          value.pointer_type,
          value.button,
          value.buttons,
          value.pressure,
          value.click_count,
          ModifiersDebug(value.modifiers),
        )),
        None,
      )
    }
    UiEventKind::PointerMove => {
      let value = event.pointer_move().expect("pointer-move body");
      (
        Some(format!(
          "POINTER MOVE\nposition  {:.0}, {:.0}\ndelta     {:.0}, {:.0}\npointer   {} · {:?}\nchanged   {:?}\nbuttons   {}\npressure  {:.2}\nclicks    {}\nmodifiers {:?}",
          value.position.0,
          value.position.1,
          value.delta.0,
          value.delta.1,
          value.pointer_id,
          value.pointer_type,
          value.changed_button,
          value.buttons,
          value.pressure,
          value.click_count,
          ModifiersDebug(value.modifiers),
        )),
        None,
      )
    }
    UiEventKind::PointerUp => (None, Some(false)),
    UiEventKind::PointerCapture => (None, Some(true)),
    UiEventKind::PointerCaptureOut => (None, Some(false)),
    UiEventKind::Wheel => {
      let value = event.wheel().expect("wheel body");
      (
        Some(format!(
          "WHEEL\nposition  {:.0}, {:.0}\ndelta     {:.1}, {:.1}, {:.1}\nmodifiers {:?}",
          value.position.0,
          value.position.1,
          value.delta.0,
          value.delta.1,
          value.delta.2,
          ModifiersDebug(value.modifiers),
        )),
        None,
      )
    }
    _ => return Ok(false),
  };
  if let Some(payload) = payload {
    response.label(PAYLOAD_ID, &payload)?;
  }
  let subscriptions = [
    (
      TARGET_ROUTE_ID,
      kinds()
        .map(UiEventSubscription::target)
        .into_iter()
        .collect(),
    ),
    (PANEL_ROUTE_ID, routed().into_iter().collect()),
    (ROOT_ROUTE_ID, routed().into_iter().collect()),
  ];
  let deliveries = battlement::routing::route_subscriptions_for_kind(
    &subscriptions,
    target_id,
    event.event_kind(),
  );
  let route_keys = [
    (ROOT_ROUTE_ID, UiEventPhase::Trickle),
    (PANEL_ROUTE_ID, UiEventPhase::Trickle),
    (TARGET_ROUTE_ID, UiEventPhase::Target),
    (PANEL_ROUTE_ID, UiEventPhase::Bubble),
    (ROOT_ROUTE_ID, UiEventPhase::Bubble),
  ];
  for (id, key) in ROUTE_STEPS.into_iter().zip(route_keys) {
    let active = deliveries
      .iter()
      .any(|delivery| (delivery.object_id, delivery.phase) == key);
    let (background, foreground) = if active {
      (
        design_system::ACCENT,
        battlement::Color::rgb(0.018, 0.055, 0.075),
      )
    } else {
      (
        battlement::Color::rgb(0.045, 0.14, 0.17),
        battlement::Color::rgb(0.55, 0.66, 0.7),
      )
    };
    let element = response
      .writer()
      .label_update_builder()
      .padding(6.0, 8.0)
      .margin(2.0, 2.0)
      .background_color(rgba(background))
      .color(rgba(foreground))
      .border_radius(6.0)
      .font_size(10.0)
      .text_align_middle_center()
      .finish();
    response.update(id, element)?;
  }
  if let Some(active) = captured {
    let text = if active {
      "● CAPTURED · POINTER OWNED BY TARGET"
    } else {
      "✓ ACTIVE CAPTURE OBSERVED\n✓ RELEASE OBSERVED · ROUTING COMPLETE"
    };
    let (background, foreground) = pointer_routing_styles::capture_colors(active);
    let element = response
      .writer()
      .label_update_builder()
      .text(text)
      .background_color(rgba(background))
      .color(rgba(foreground))
      .finish();
    response.update(CAPTURE_ID, element)?;
  }
  Ok(true)
}

fn rgba(value: battlement::Color) -> [f64; 4] {
  [value.r, value.g, value.b, value.a]
}

struct ModifiersDebug(u32);

impl fmt::Debug for ModifiersDebug {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    const NAMES: [&str; 7] = [
      "Alt",
      "Control",
      "Command",
      "Shift",
      "CapsLock",
      "Numeric",
      "FunctionKey",
    ];
    formatter.write_str("KeyModifiers([")?;
    let mut separator = "";
    for (index, name) in NAMES.into_iter().enumerate() {
      if self.0 & (1 << index) == 0 {
        continue;
      }
      formatter.write_str(separator)?;
      formatter.write_str(name)?;
      separator = ", ";
    }
    formatter.write_str("])")
  }
}

fn route_card() -> UiNode {
  node(UiBox::new().style(pointer_routing_styles::route_card()))
    .child(node(
      UiLabel::new("LOGICAL ROUTE").style(pointer_routing_styles::caption()),
    ))
    .child(
      UiNode::new(
        ROOT_ROUTE_ID,
        UiBox::new()
          .name("pointer-route-root")
          .event_subscriptions(routed())
          .style(pointer_routing_styles::root(false)),
      )
      .child(node(
        UiLabel::new("ROOT · trickle + bubble").style(pointer_routing_styles::node_label()),
      ))
      .child(
        UiNode::new(
          PANEL_ROUTE_ID,
          UiBox::new()
            .name("pointer-route-panel")
            .event_subscriptions(routed())
            .style(pointer_routing_styles::panel(false)),
        )
        .child(node(
          UiLabel::new("PANEL · trickle + bubble").style(pointer_routing_styles::node_label()),
        ))
        .child(UiNode::new(
          TARGET_ROUTE_ID,
          UiButton::new("PRESS + DRAG\nCAPTURE TARGET")
            .name("pointer-capture-target")
            .events(kinds())
            .style(pointer_routing_styles::target(false)),
        )),
      ),
    )
    .child(
      node(UiVisualElement::new().style(pointer_routing_styles::route_strip()))
        .child(route_step(0, "ROOT ↓"))
        .child(route_step(1, "PANEL ↓"))
        .child(route_step(2, "TARGET"))
        .child(route_step(3, "PANEL ↑"))
        .child(route_step(4, "ROOT ↑")),
    )
}

fn inspector() -> UiNode {
  node(UiBox::new().style(pointer_routing_styles::inspector_card()))
        .child(node(UiLabel::new("RUST EVENT INSPECTOR").style(pointer_routing_styles::caption())))
        .child(UiNode::new(CAPTURE_ID, UiLabel::new("○ READY · PRESS THE TARGET").style(pointer_routing_styles::capture(false))))
        .child(UiNode::new(PAYLOAD_ID, UiLabel::new("No event yet.\n\nDefaults remain omitted on the wire; this inspector shows their restored typed values.").style(pointer_routing_styles::payload())))
        .child(node(UiLabel::new("The native target is mapped to the nearest Rust-owned ancestor before a single action crosses the transport.").style(pointer_routing_styles::hint())))
}

fn routed() -> [UiEventSubscription; 10] {
  [
    UiEventKind::PointerDown,
    UiEventKind::PointerMove,
    UiEventKind::PointerUp,
    UiEventKind::PointerCapture,
    UiEventKind::PointerCaptureOut,
  ]
  .into_iter()
  .flat_map(|kind| {
    [
      UiEventSubscription::new(kind, UiEventPhase::Trickle),
      UiEventSubscription::new(kind, UiEventPhase::Bubble),
    ]
  })
  .collect::<Vec<_>>()
  .try_into()
  .expect("route subscription count is fixed")
}

fn kinds() -> [UiEventKind; 6] {
  [
    UiEventKind::PointerDown,
    UiEventKind::PointerMove,
    UiEventKind::PointerUp,
    UiEventKind::Wheel,
    UiEventKind::PointerCapture,
    UiEventKind::PointerCaptureOut,
  ]
}

fn route_step(index: usize, label: &str) -> UiNode {
  UiNode::new(
    ROUTE_STEPS[index],
    UiLabel::new(label).style(pointer_routing_styles::route_step(false)),
  )
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
