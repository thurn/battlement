use battlement_ui::{
  LanguageDirection, PickingMode, Prop, UiElement, UiEventKind, UiEventPhase, UiEventSubscription,
  VisualElement,
};

#[test]
fn enabled_serializes_omit_set_and_reset_wire_states() {
  assert_eq!(
    serde_json::to_value(UiElement::from(VisualElement::new())).unwrap(),
    serde_json::json!({"VisualElement": {}})
  );
  assert_eq!(
    serde_json::to_value(UiElement::from(VisualElement::new().enabled(false))).unwrap(),
    serde_json::json!({"VisualElement": {"enabled": false}})
  );
  assert_eq!(
    serde_json::to_value(UiElement::from(VisualElement::new().enabled(Prop::Reset))).unwrap(),
    serde_json::json!({"VisualElement": {"enabled": null}})
  );
}

#[test]
fn enabled_deserializes_only_unambiguous_wire_states() {
  for (json, expected) in [
    (r#"{"VisualElement":{}}"#, Prop::Unset),
    (r#"{"VisualElement":{"enabled":false}}"#, Prop::Set(false)),
    (r#"{"VisualElement":{"enabled":null}}"#, Prop::Reset),
  ] {
    let UiElement::VisualElement(value) = serde_json::from_str(json).unwrap() else {
      panic!("decoded the wrong element kind");
    };
    assert_eq!(value.enabled, expected);
  }

  for json in [
    r#"{"VisualElement":{"enabled":"false"}}"#,
    r#"{"VisualElement":{"enabled":0}}"#,
    r#"{"VisualElement":{"enabled":{}}}"#,
    r#"{"VisualElement":{"enabled":false,"enabled":null}}"#,
  ] {
    assert!(serde_json::from_str::<UiElement>(json).is_err(), "{json}");
  }
}

#[test]
fn shared_visual_properties_serialize_set_and_reset_states() {
  let set = VisualElement::new()
    .name("status")
    .enabled(false)
    .picking_mode(PickingMode::Ignore)
    .language_direction(LanguageDirection::Rtl)
    .focusable(true)
    .tab_index(4)
    .delegates_focus(true)
    .class("primary")
    .events([UiEventKind::Click])
    .event_subscriptions([UiEventSubscription::new(
      UiEventKind::PointerDown,
      UiEventPhase::Bubble,
    )]);
  let value = serde_json::to_value(UiElement::from(set)).unwrap();
  let fields = &value["VisualElement"];
  assert_eq!(fields["name"], "status");
  assert_eq!(fields["enabled"], false);
  assert_eq!(fields["picking_mode"], "Ignore");
  assert_eq!(fields["language_direction"], "Rtl");
  assert_eq!(fields["focusable"], true);
  assert_eq!(fields["tab_index"], 4);
  assert_eq!(fields["delegates_focus"], true);
  assert_eq!(fields["classes"], serde_json::json!(["primary"]));
  assert_eq!(fields["events"], serde_json::json!(["Click"]));
  assert_eq!(
    fields["event_subscriptions"],
    serde_json::json!([{"kind": "PointerDown", "phase": "Bubble"}])
  );

  let reset = VisualElement {
    name: Prop::Reset,
    enabled: Prop::Reset,
    picking_mode: Prop::Reset,
    language_direction: Prop::Reset,
    focusable: Prop::Reset,
    tab_index: Prop::Reset,
    delegates_focus: Prop::Reset,
    classes: Prop::Reset,
    events: Prop::Reset,
    event_subscriptions: Prop::Reset,
    ..VisualElement::new()
  };
  assert_eq!(
    serde_json::to_value(UiElement::from(reset)).unwrap(),
    serde_json::json!({
      "VisualElement": {
        "name": null,
        "enabled": null,
        "picking_mode": null,
        "language_direction": null,
        "focusable": null,
        "tab_index": null,
        "delegates_focus": null,
        "classes": null,
        "events": null,
        "event_subscriptions": null
      }
    })
  );
}
