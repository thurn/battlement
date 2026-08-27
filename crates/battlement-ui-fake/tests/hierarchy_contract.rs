use battlement_types::ObjectId;
use battlement_ui::{
  Box, Label, UiDocument, UiNode, VisualElement, VisualElementCreate, VisualElementUpdate,
};
use battlement_ui_fake::{UiWorld, UiWorldError};

#[test]
fn rejected_placements_preserve_logical_hierarchy() {
  let first_document = ObjectId::new_v4();
  let first_root = ObjectId::new_v4();
  let parent = ObjectId::new_v4();
  let child = ObjectId::new_v4();
  let second_document = ObjectId::new_v4();
  let second_root = ObjectId::new_v4();
  let second_parent = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::with_root_id(first_document, first_root).child(
        UiNode::new(parent, VisualElement::new()).child(UiNode::new(child, VisualElement::new())),
      ),
      UiDocument::with_root_id(second_document, second_root)
        .child(UiNode::new(second_parent, VisualElement::new())),
    ])
    .unwrap();

  assert_eq!(
    world.update(VisualElementUpdate::Parent {
      object_id: parent,
      parent_id: child,
    }),
    Err(UiWorldError::InvalidHierarchy)
  );
  assert_eq!(
    world.update(VisualElementUpdate::Parent {
      object_id: child,
      parent_id: second_parent,
    }),
    Err(UiWorldError::InvalidHierarchy)
  );
  assert_eq!(world.element(parent).unwrap().children(), [child]);
  assert_eq!(world.element(child).unwrap().parent_id(), Some(parent));
}

#[test]
fn detached_failure_and_recursive_destroy_leave_no_partial_state() {
  let document_id = ObjectId::new_v4();
  let root_id = ObjectId::new_v4();
  let parent_id = ObjectId::new_v4();
  let duplicate_id = ObjectId::new_v4();
  let detached_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![UiDocument::with_root_id(document_id, root_id).child(
      UiNode::new(parent_id, Box::new()).child(UiNode::new(duplicate_id, Label::new("existing"))),
    )])
    .unwrap();

  let invalid =
    UiNode::new(detached_id, Box::new()).child(UiNode::new(duplicate_id, Label::new("duplicate")));
  assert_eq!(
    world.create(VisualElementCreate::new(parent_id, invalid)),
    Err(UiWorldError::DuplicateObject)
  );
  assert!(world.element(detached_id).is_none());
  assert_eq!(world.element(parent_id).unwrap().children(), [duplicate_id]);

  world.destroy(parent_id).unwrap();
  assert!(world.element(parent_id).is_none());
  assert!(world.element(duplicate_id).is_none());
  assert!(world.element(root_id).unwrap().children().is_empty());
}
