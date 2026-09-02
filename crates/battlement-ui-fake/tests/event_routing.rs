use battlement_types::{ObjectId, PointerButton, object_id};
use battlement_ui::{
  KeyModifiers, PanelPoint, PointerButtonEvent, PointerCrossingEvent, PointerMoveEvent,
  PointerType, UiBox, UiDocument, UiEvent, UiEventBody, UiEventKind, UiEventPhase,
  UiEventSubscription, UiNode, UiVisualElement, Vector,
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

#[test]
fn fake_serialization_preserves_crossing_relations_and_distinct_intervening_events() {
  let mut world = UiWorld::default();
  world.replace(crossing_documents()).unwrap();
  let events = [
    crossing(
      TARGET_ID,
      UiEventBody::PointerOut(PointerCrossingEvent {
        related_target_id: Some(PANEL_ID),
        pointer_id: 7,
        position: PanelPoint::new(12.0, 8.0),
        pointer_type: PointerType::Pen,
      }),
    ),
    UiEvent {
      target_id: PANEL_ID,
      cancelable: false,
      default_prevented: false,
      body: UiEventBody::PointerMove(PointerMoveEvent {
        pointer_id: 7,
        position: PanelPoint::new(12.0, 8.0),
        delta: Vector::new(4.0, 0.0),
        changed_button: None,
        buttons: 0,
        pressure: 0.0,
        click_count: 0,
        modifiers: KeyModifiers::default(),
        pointer_type: PointerType::Pen,
      }),
    },
    crossing(
      PANEL_ID,
      UiEventBody::PointerOver(PointerCrossingEvent {
        related_target_id: None,
        pointer_id: 7,
        position: PanelPoint::new(12.0, 8.0),
        pointer_type: PointerType::Pen,
      }),
    ),
  ];
  let restored = events
    .iter()
    .map(|event| serde_json::from_str(&serde_json::to_string(event).unwrap()).unwrap())
    .collect::<Vec<UiEvent>>();
  assert_eq!(restored, events);
  assert_eq!(
    serde_json::to_value(&restored[0]).unwrap()["body"]["PointerOut"]["related_target_id"],
    serde_json::json!(PANEL_ID)
  );
  assert!(
    serde_json::to_value(&restored[2]).unwrap()["body"]["PointerOver"]
      .get("related_target_id")
      .is_none()
  );
  assert_eq!(
    restored
      .iter()
      .flat_map(|event| world.route_event(event))
      .map(|delivery| delivery.object_id)
      .collect::<Vec<_>>(),
    vec![TARGET_ID, PANEL_ID, PANEL_ID]
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
          UiBox::new().event_subscriptions([
            UiEventSubscription::new(UiEventKind::PointerDown, UiEventPhase::Trickle),
            UiEventSubscription::new(UiEventKind::PointerDown, UiEventPhase::Bubble),
          ]),
        )
        .child(UiNode::new(
          TARGET_ID,
          UiVisualElement::new().events([UiEventKind::PointerDown]),
        )),
      ),
  ]
}

fn crossing_documents() -> Vec<UiDocument> {
  vec![
    UiDocument::with_root_id(DOCUMENT_ID, ROOT_ID).child(
      UiNode::new(
        PANEL_ID,
        UiBox::new().events([UiEventKind::PointerMove, UiEventKind::PointerOver]),
      )
      .child(UiNode::new(
        TARGET_ID,
        UiVisualElement::new().events([UiEventKind::PointerOut]),
      )),
    ),
  ]
}

fn crossing(target_id: ObjectId, body: UiEventBody) -> UiEvent {
  UiEvent::new(target_id, false, false, body)
}

fn event() -> UiEvent {
  UiEvent {
    target_id: TARGET_ID,
    cancelable: true,
    default_prevented: false,
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
