use battlement_types::{ObjectId, PointerButton, object_id};
use battlement_ui::{
  Box, KeyModifiers, PanelPoint, PointerButtonEvent, PointerType, UiDocument, UiEvent, UiEventBody,
  UiEventKind, UiEventPhase, UiEventSubscription, UiNode, Vector, VisualElement,
};
use battlement_ui_fake::UiWorld;

const DOCUMENT_ID: ObjectId = object_id!("22000000-0000-4000-8000-000000000011");
const ROOT_ID: ObjectId = object_id!("22000000-0000-4000-8000-000000000012");
const PANEL_ID: ObjectId = object_id!("22000000-0000-4000-8000-000000000013");
const TARGET_ID: ObjectId = object_id!("22000000-0000-4000-8000-000000000014");

#[test]
fn fake_routes_the_same_trickle_target_and_bubble_order() {
  let mut world = UiWorld::default();
  world.replace(documents()).unwrap();
  let deliveries = world
    .route_event(&event())
    .into_iter()
    .map(|value| (value.object_id, value.phase))
    .collect::<Vec<_>>();
  assert_eq!(
    deliveries,
    vec![
      (ROOT_ID, UiEventPhase::Trickle),
      (PANEL_ID, UiEventPhase::Trickle),
      (TARGET_ID, UiEventPhase::Target),
      (PANEL_ID, UiEventPhase::Bubble),
      (ROOT_ID, UiEventPhase::Bubble),
    ]
  );
}

fn documents() -> Vec<UiDocument> {
  vec![
    UiDocument::with_root_id(DOCUMENT_ID, ROOT_ID)
      .event_subscriptions([
        UiEventSubscription::new(UiEventKind::PointerDown, UiEventPhase::Trickle),
        UiEventSubscription::new(UiEventKind::PointerDown, UiEventPhase::Bubble),
      ])
      .child(
        UiNode::new(
          PANEL_ID,
          Box::new().event_subscriptions([
            UiEventSubscription::new(UiEventKind::PointerDown, UiEventPhase::Trickle),
            UiEventSubscription::new(UiEventKind::PointerDown, UiEventPhase::Bubble),
          ]),
        )
        .child(UiNode::new(
          TARGET_ID,
          VisualElement::new().events([UiEventKind::PointerDown]),
        )),
      ),
  ]
}

fn event() -> UiEvent {
  UiEvent {
    target_id: TARGET_ID,
    body: UiEventBody::PointerDown(PointerButtonEvent {
      pointer_id: 0,
      position: PanelPoint::new(4.0, 8.0),
      delta: Vector::new(1.0, 2.0),
      button: PointerButton::Left,
      buttons: 0,
      pressure: 0.0,
      click_count: 1,
      modifiers: KeyModifiers::default(),
      pointer_type: PointerType::Mouse,
    }),
  }
}
