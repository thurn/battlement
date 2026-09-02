use battlement_types::ObjectId;
use battlement_ui::{UiBox, UiDocument, UiNode, UiValidationError, validate_documents};

#[test]
fn complete_tree_accepts_one_auto_focus_and_inertness() {
  let document = UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
    ObjectId::new_v4(),
    UiBox::new().focusable(true).auto_focus(true).inert(false),
  ));

  assert!(validate_documents(&[document]).is_ok());
}

#[test]
fn complete_tree_rejects_duplicate_auto_focus_candidates() {
  let document = UiDocument::new(ObjectId::new_v4())
    .child(UiNode::new(
      ObjectId::new_v4(),
      UiBox::new().focusable(true).auto_focus(true),
    ))
    .child(UiNode::new(
      ObjectId::new_v4(),
      UiBox::new().focusable(true).auto_focus(true),
    ));

  assert_eq!(
    validate_documents(&[document]),
    Err(UiValidationError::InvalidProperty)
  );
}

#[test]
fn focus_properties_round_trip_without_focus_state_records() {
  let element = UiBox::new().auto_focus(true).inert(true);
  let json = serde_json::to_value(element).unwrap();

  assert_eq!(json["auto_focus"], true);
  assert_eq!(json["inert"], true);
  assert_eq!(json.get("focused_element"), None);
  assert_eq!(json.get("focus_request"), None);
}
