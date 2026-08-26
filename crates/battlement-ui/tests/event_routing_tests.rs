use battlement_types::{ObjectId, PhysicalKey, PointerButton, object_id};
use battlement_ui::{
    Box, FocusDirection, FocusEvent, KeyEvent, KeyModifier, KeyModifiers, NavigationDirection,
    NavigationMoveEvent, PanelPoint, PointerBoundaryEvent, PointerButtonEvent, PointerType,
    UiDocument, UiEvent, UiEventBody, UiEventKind, UiEventPhase, UiEventSubscription, UiNode,
    Vector, VisualElement,
};
use serde_json::json;

const DOCUMENT_ID: ObjectId = object_id!("22000000-0000-4000-8000-000000000001");
const ROOT_ID: ObjectId = object_id!("22000000-0000-4000-8000-000000000002");
const PANEL_ID: ObjectId = object_id!("22000000-0000-4000-8000-000000000003");
const TARGET_ID: ObjectId = object_id!("22000000-0000-4000-8000-000000000004");

#[test]
fn routed_pointer_order_is_deterministic_and_matches_the_fake() {
    let documents = routed_documents();
    let event = pointer_down();
    let expected = vec![
        (ROOT_ID, UiEventPhase::Trickle),
        (PANEL_ID, UiEventPhase::Trickle),
        (TARGET_ID, UiEventPhase::Target),
        (PANEL_ID, UiEventPhase::Bubble),
        (ROOT_ID, UiEventPhase::Bubble),
    ];
    let deliveries = battlement_ui::routing::route_event(&documents, &event)
        .into_iter()
        .map(|value| (value.object_id, value.phase))
        .collect::<Vec<_>>();
    assert_eq!(deliveries, expected);
}

#[test]
fn target_only_events_ignore_ancestors_and_reject_ancestor_phases() {
    let event = UiEvent {
        target_id: TARGET_ID,
        body: UiEventBody::PointerEnter(PointerBoundaryEvent {
            pointer_id: 0,
            position: PanelPoint::new(12.0, 24.0),
            pointer_type: PointerType::Mouse,
        }),
    };
    let documents = vec![
        UiDocument::with_root_id(DOCUMENT_ID, ROOT_ID).child(UiNode::new(
            TARGET_ID,
            VisualElement::new().events([UiEventKind::PointerEnter]),
        )),
    ];
    let deliveries = battlement_ui::routing::route_event(&documents, &event);
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].object_id, TARGET_ID);
    assert_eq!(deliveries[0].phase, UiEventPhase::Target);

    let invalid = vec![
        UiDocument::with_root_id(DOCUMENT_ID, ROOT_ID).event_subscriptions([
            UiEventSubscription::new(UiEventKind::PointerEnter, UiEventPhase::Trickle),
        ]),
    ];
    assert!(battlement_ui::validate_documents(&invalid).is_err());
}

#[test]
fn pointer_payloads_omit_defaults_and_preserve_other_buttons() {
    assert_eq!(
        serde_json::to_value(pointer_down()).unwrap(),
        json!({
            "target_id": TARGET_ID,
            "body": {
                "PointerDown": {
                    "position": { "x": 4.0, "y": 8.0 },
                    "delta": { "x": 1.0, "y": 2.0 }
                }
            }
        })
    );
    let mut event = pointer_down();
    let UiEventBody::PointerDown(value) = &mut event.body else {
        unreachable!();
    };
    value.button = PointerButton::Other(7);
    assert_eq!(
        serde_json::to_value(event).unwrap()["body"]["PointerDown"]["button"],
        json!({ "Other": 7 })
    );
}

#[test]
fn focus_payloads_preserve_owned_relations_and_omit_external_relations() {
    let related = UiEvent {
        target_id: TARGET_ID,
        body: UiEventBody::FocusIn(FocusEvent {
            related_target_id: Some(PANEL_ID),
            ..FocusEvent::default()
        }),
    };
    assert_eq!(
        serde_json::to_value(related).unwrap()["body"]["FocusIn"]["related_target_id"],
        json!(PANEL_ID)
    );
    let external = UiEvent {
        target_id: TARGET_ID,
        body: UiEventBody::Blur(FocusEvent::default()),
    };
    assert_eq!(
        serde_json::to_value(external).unwrap(),
        json!({ "target_id": TARGET_ID, "body": { "Blur": {} } })
    );
}

#[test]
fn keyboard_navigation_payloads_are_exact_and_route_deterministically() {
    let key = UiEvent {
        target_id: TARGET_ID,
        body: UiEventBody::KeyDown(KeyEvent {
            physical_key: Some(PhysicalKey::KeyA),
            text: "A".to_owned(),
            modifiers: KeyModifiers::new(vec![KeyModifier::Shift]).unwrap(),
        }),
    };
    assert_eq!(
        serde_json::to_value(&key).unwrap()["body"]["KeyDown"],
        json!({ "physical_key": "KeyA", "text": "A", "modifiers": ["Shift"] })
    );
    let unmapped = UiEvent {
        target_id: TARGET_ID,
        body: UiEventBody::KeyUp(KeyEvent::default()),
    };
    assert_eq!(
        serde_json::to_value(unmapped).unwrap()["body"]["KeyUp"],
        json!({ "text": "" })
    );

    let route = vec![
        (
            TARGET_ID,
            vec![UiEventSubscription::target(UiEventKind::NavigationMove)],
        ),
        (
            PANEL_ID,
            vec![
                UiEventSubscription::new(UiEventKind::NavigationMove, UiEventPhase::Trickle),
                UiEventSubscription::new(UiEventKind::NavigationMove, UiEventPhase::Bubble),
            ],
        ),
    ];
    let movement = UiEvent {
        target_id: TARGET_ID,
        body: UiEventBody::NavigationMove(NavigationMoveEvent {
            direction: NavigationDirection::Right,
            move_vector: Vector::new(1.0, 0.0),
        }),
    };
    let deliveries = battlement_ui::routing::route_subscriptions(&route, &movement);
    assert_eq!(
        deliveries
            .into_iter()
            .map(|value| (value.object_id, value.phase))
            .collect::<Vec<_>>(),
        vec![
            (PANEL_ID, UiEventPhase::Trickle),
            (TARGET_ID, UiEventPhase::Target),
            (PANEL_ID, UiEventPhase::Bubble),
        ]
    );
}

#[test]
fn focus_direction_is_preserved_without_serializing_native_objects() {
    let event = UiEvent {
        target_id: TARGET_ID,
        body: UiEventBody::Focus(FocusEvent {
            related_target_id: Some(PANEL_ID),
            direction: FocusDirection::Other(27),
        }),
    };
    assert_eq!(
        serde_json::to_value(event).unwrap()["body"]["Focus"]["direction"],
        json!({ "Other": 27 })
    );
}

fn routed_documents() -> Vec<UiDocument> {
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
                    VisualElement::new()
                        .events([UiEventKind::PointerDown])
                        .event_subscriptions([
                            UiEventSubscription::new(
                                UiEventKind::PointerDown,
                                UiEventPhase::Trickle,
                            ),
                            UiEventSubscription::new(
                                UiEventKind::PointerDown,
                                UiEventPhase::Bubble,
                            ),
                        ]),
                )),
            ),
    ]
}

fn pointer_down() -> UiEvent {
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
