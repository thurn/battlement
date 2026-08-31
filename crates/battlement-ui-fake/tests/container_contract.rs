use battlement_types::ObjectId;
use battlement_ui::{
  Prop, UiDocument, UiElement, UiGroupBox, UiLabel, UiNode, UiPopupWindow, VisualElementUpdate,
};
use battlement_ui_fake::UiWorld;

#[test]
fn group_and_popup_updates_preserve_logical_content_order() {
  let document_id = ObjectId::new_v4();
  let root_id = ObjectId::new_v4();
  let group_id = ObjectId::new_v4();
  let group_child_id = ObjectId::new_v4();
  let popup_id = ObjectId::new_v4();
  let first_popup_child_id = ObjectId::new_v4();
  let second_popup_child_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::with_root_id(document_id, root_id)
        .child(
          UiNode::new(group_id, UiGroupBox::new().text("Settings"))
            .child(UiNode::new(group_child_id, UiLabel::new("Music"))),
        )
        .child(
          UiNode::new(popup_id, UiPopupWindow::new().text("Deployment"))
            .child(UiNode::new(first_popup_child_id, UiLabel::new("First")))
            .child(UiNode::new(second_popup_child_id, UiLabel::new("Second"))),
        ),
    ])
    .unwrap();

  world
    .update(VisualElementUpdate::Properties {
      object_id: group_id,
      element: UiElement::from(UiGroupBox::new().text("")).into(),
    })
    .unwrap();
  world
    .update(VisualElementUpdate::Index {
      object_id: second_popup_child_id,
      child_index: 0,
    })
    .unwrap();

  assert_eq!(world.element(group_id).unwrap().text(), Some(""));
  assert_eq!(
    world.element(group_id).unwrap().children(),
    [group_child_id]
  );
  assert_eq!(
    world.element(popup_id).unwrap().children(),
    [second_popup_child_id, first_popup_child_id]
  );

  world
    .update(VisualElementUpdate::Properties {
      object_id: group_id,
      element: UiElement::from(UiGroupBox::new().text(Prop::Reset)).into(),
    })
    .unwrap();
  world
    .update(VisualElementUpdate::Properties {
      object_id: popup_id,
      element: UiElement::from(
        UiPopupWindow::new()
          .text(Prop::Reset)
          .rich_text(Prop::Reset)
          .selectable(Prop::Reset),
      )
      .into(),
    })
    .unwrap();

  assert_eq!(world.element(group_id).unwrap().text(), None);
  assert_eq!(world.element(group_id).unwrap().object_id(), group_id);
  assert_eq!(
    world.element(group_id).unwrap().children(),
    [group_child_id]
  );
  assert_eq!(world.element(popup_id).unwrap().text(), None);
  assert_eq!(
    world.element(popup_id).unwrap().children(),
    [second_popup_child_id, first_popup_child_id]
  );
}
