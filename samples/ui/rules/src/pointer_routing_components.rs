use battlement::{
  Box, Button, Command, Label, ObjectId, UiElement, UiEvent, UiEventBody, UiEventKind,
  UiEventPhase, UiEventSubscription, UiNode, VisualElement, object_id,
};

use crate::{design_system, pointer_routing_styles};

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
  UiNode::new(page_id, VisualElement::new().name("pointer-routing-page"))
        .child(node(Label::new("POINTER ROUTING").style(design_system::eyebrow())))
        .child(node(Label::new("One native event. One Rust action.").style(design_system::title())))
        .child(node(Label::new("Press and drag on the target. Rust reconstructs the five-step logical route while Unity reports one complete pointer payload and capture lifecycle.").style(pointer_routing_styles::intro())))
        .child(node(VisualElement::new().style(pointer_routing_styles::columns()))
            .child(route_card()).child(inspector()))
}

pub(crate) fn event_commands(event: &UiEvent) -> Option<Vec<Command>> {
  if event.target_id != TARGET_ROUTE_ID {
    return None;
  }
  let (payload, captured) = match &event.body {
    UiEventBody::PointerDown(value) => (
      Some(format!(
        "POINTER DOWN\nposition  {:.0}, {:.0}\ndelta     {:.0}, {:.0}\npointer   {} · {:?}\nbutton    {:?}\nbuttons   {}\npressure  {:.2}\nclicks    {}\nmodifiers {:?}",
        value.position.x,
        value.position.y,
        value.delta.x,
        value.delta.y,
        value.pointer_id,
        value.pointer_type,
        value.button,
        value.buttons,
        value.pressure,
        value.click_count,
        value.modifiers,
      )),
      None,
    ),
    UiEventBody::PointerMove(value) => (
      Some(format!(
        "POINTER MOVE\nposition  {:.0}, {:.0}\ndelta     {:.0}, {:.0}\npointer   {} · {:?}\nchanged   {:?}\nbuttons   {}\npressure  {:.2}\nclicks    {}\nmodifiers {:?}",
        value.position.x,
        value.position.y,
        value.delta.x,
        value.delta.y,
        value.pointer_id,
        value.pointer_type,
        value.changed_button,
        value.buttons,
        value.pressure,
        value.click_count,
        value.modifiers,
      )),
      None,
    ),
    UiEventBody::PointerUp(_) => (None, Some(false)),
    UiEventBody::PointerCapture(_) => (None, Some(true)),
    UiEventBody::PointerCaptureOut(_) => (None, Some(false)),
    UiEventBody::Wheel(value) => (
      Some(format!(
        "WHEEL\nposition  {:.0}, {:.0}\ndelta     {:.1}, {:.1}, {:.1}\nmodifiers {:?}",
        value.position.x,
        value.position.y,
        value.delta.x,
        value.delta.y,
        value.delta.z,
        value.modifiers
      )),
      None,
    ),
    _ => return None,
  };
  let mut commands = Vec::new();
  if let Some(payload) = payload {
    commands.push(Command::update_visual_element(
      PAYLOAD_ID,
      Label::new(payload),
    ));
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
  let deliveries = battlement::routing::route_subscriptions(&subscriptions, event);
  let route_keys = [
    (ROOT_ROUTE_ID, UiEventPhase::Trickle),
    (PANEL_ROUTE_ID, UiEventPhase::Trickle),
    (TARGET_ROUTE_ID, UiEventPhase::Target),
    (PANEL_ROUTE_ID, UiEventPhase::Bubble),
    (ROOT_ROUTE_ID, UiEventPhase::Bubble),
  ];
  commands.extend(ROUTE_STEPS.into_iter().zip(route_keys).map(|(id, key)| {
    let active = deliveries
      .iter()
      .any(|delivery| (delivery.object_id, delivery.phase) == key);
    Command::update_visual_element(
      id,
      Label::default().style(pointer_routing_styles::route_step(active)),
    )
  }));
  if let Some(active) = captured {
    commands.push(Command::update_visual_element(
      CAPTURE_ID,
      Label::new(if active {
        "● CAPTURED · POINTER OWNED BY TARGET"
      } else {
        "✓ ACTIVE CAPTURE OBSERVED\n✓ RELEASE OBSERVED · ROUTING COMPLETE"
      })
      .style(pointer_routing_styles::capture(active)),
    ));
  }
  Some(commands)
}

fn route_card() -> UiNode {
  node(Box::new().style(pointer_routing_styles::route_card()))
    .child(node(
      Label::new("LOGICAL ROUTE").style(pointer_routing_styles::caption()),
    ))
    .child(
      UiNode::new(
        ROOT_ROUTE_ID,
        Box::new()
          .name("pointer-route-root")
          .event_subscriptions(routed())
          .style(pointer_routing_styles::root(false)),
      )
      .child(node(
        Label::new("ROOT · trickle + bubble").style(pointer_routing_styles::node_label()),
      ))
      .child(
        UiNode::new(
          PANEL_ROUTE_ID,
          Box::new()
            .name("pointer-route-panel")
            .event_subscriptions(routed())
            .style(pointer_routing_styles::panel(false)),
        )
        .child(node(
          Label::new("PANEL · trickle + bubble").style(pointer_routing_styles::node_label()),
        ))
        .child(UiNode::new(
          TARGET_ROUTE_ID,
          Button::new("PRESS + DRAG\nCAPTURE TARGET")
            .name("pointer-capture-target")
            .events(kinds())
            .style(pointer_routing_styles::target(false)),
        )),
      ),
    )
    .child(
      node(VisualElement::new().style(pointer_routing_styles::route_strip()))
        .child(route_step(0, "ROOT ↓"))
        .child(route_step(1, "PANEL ↓"))
        .child(route_step(2, "TARGET"))
        .child(route_step(3, "PANEL ↑"))
        .child(route_step(4, "ROOT ↑")),
    )
}

fn inspector() -> UiNode {
  node(Box::new().style(pointer_routing_styles::inspector_card()))
        .child(node(Label::new("RUST EVENT INSPECTOR").style(pointer_routing_styles::caption())))
        .child(UiNode::new(CAPTURE_ID, Label::new("○ READY · PRESS THE TARGET").style(pointer_routing_styles::capture(false))))
        .child(UiNode::new(PAYLOAD_ID, Label::new("No event yet.\n\nDefaults remain omitted on the wire; this inspector shows their restored typed values.").style(pointer_routing_styles::payload())))
        .child(node(Label::new("The native target is mapped to the nearest Rust-owned ancestor before a single action crosses the transport.").style(pointer_routing_styles::hint())))
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
    Label::new(label).style(pointer_routing_styles::route_step(false)),
  )
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
