use battlement_types::{ObjectId, PointerButton, Rect};
use battlement_ui::{
    GeometryEvent, LifecycleEvent, LinkEvent, PanelPoint, SelectionEvent, UiEvent, UiEventBody,
    UiEventKind,
};

#[test]
fn remaining_event_payloads_have_the_locked_wire_shapes() {
    let target_id = ObjectId::new_v4();
    assert_eq!(
        serde_json::to_value(UiEvent {
            target_id,
            body: UiEventBody::SelectionChanged(SelectionEvent {
                cursor_index: 7,
                selection_index: 3,
            }),
        })
        .unwrap(),
        serde_json::json!({
            "target_id": target_id,
            "body": {"SelectionChanged": {"cursor_index": 7, "selection_index": 3}}
        })
    );
    assert_eq!(
        serde_json::to_value(UiEvent {
            target_id,
            body: UiEventBody::GeometryChanged(GeometryEvent {
                previous: Rect::new(1.0, 2.0, 3.0, 4.0),
                current: Rect::new(5.0, 6.0, 7.0, 8.0),
            }),
        })
        .unwrap(),
        serde_json::json!({
            "target_id": target_id,
            "body": {"GeometryChanged": {
                "previous": {"x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0},
                "current": {"x": 5.0, "y": 6.0, "width": 7.0, "height": 8.0}
            }}
        })
    );
    assert_eq!(
        serde_json::to_value(UiEvent {
            target_id,
            body: UiEventBody::LinkEnter(LinkEvent {
                link_id: "guide".to_owned(),
                link_text: "Field guide".to_owned(),
                pointer_id: 0,
                position: PanelPoint { x: 11.0, y: 13.0 },
                button: None,
            }),
        })
        .unwrap(),
        serde_json::json!({
            "target_id": target_id,
            "body": {"LinkEnter": {
                "link_id": "guide", "link_text": "Field guide",
                "position": {"x": 11.0, "y": 13.0}
            }}
        })
    );
    assert_eq!(
        serde_json::to_value(UiEvent {
            target_id,
            body: UiEventBody::LinkDown(LinkEvent {
                link_id: "guide".to_owned(),
                link_text: "Field guide".to_owned(),
                pointer_id: 9,
                position: PanelPoint { x: 11.0, y: 13.0 },
                button: Some(PointerButton::Right),
            }),
        })
        .unwrap()["body"]["LinkDown"]
            .clone(),
        serde_json::json!({
            "link_id": "guide", "link_text": "Field guide", "pointer_id": 9,
            "position": {"x": 11.0, "y": 13.0}, "button": "Right"
        })
    );
    assert_eq!(
        serde_json::to_value(UiEvent {
            target_id,
            body: UiEventBody::AttachToPanel(LifecycleEvent {}),
        })
        .unwrap()["body"]
            .clone(),
        serde_json::json!({"AttachToPanel": {}})
    );
}

#[test]
fn only_native_propagating_remaining_events_accept_routed_phases() {
    for kind in [
        UiEventKind::LinkEnter,
        UiEventKind::LinkLeave,
        UiEventKind::LinkDown,
        UiEventKind::LinkUp,
    ] {
        assert!(kind.propagates());
    }
    for kind in [
        UiEventKind::GeometryChanged,
        UiEventKind::AttachToPanel,
        UiEventKind::DetachFromPanel,
        UiEventKind::SelectionChanged,
    ] {
        assert!(!kind.propagates());
    }
}

#[test]
#[should_panic(expected = "at least one supported property")]
fn transition_constructor_rejects_empty_property_lists() {
    let _ = battlement_ui::TransitionEvent::new(Vec::new(), 10.0);
}
