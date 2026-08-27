use battlement_ui::{Prop, UiElement, VisualElement};

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
