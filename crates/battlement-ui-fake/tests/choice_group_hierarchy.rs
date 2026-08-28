use battlement_types::ObjectId;
use battlement_ui::{
  Box, Button, ToggleButtonGroup, UiDocument, UiNode, VisualElementCreate, VisualElementUpdate,
};
use battlement_ui_fake::{UiWorld, UiWorldError};

#[test]
fn toggle_selection_tracks_insert_reorder_remove_and_reparent() {
  let document_id = ObjectId::new_v4();
  let root_id = ObjectId::new_v4();
  let group_id = ObjectId::new_v4();
  let outside_id = ObjectId::new_v4();
  let first_id = ObjectId::new_v4();
  let selected_id = ObjectId::new_v4();
  let third_id = ObjectId::new_v4();
  let inserted_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::with_root_id(document_id, root_id)
        .child(
          UiNode::new(group_id, ToggleButtonGroup::new().selected_indices([1])).children([
            UiNode::new(first_id, Button::new("First")),
            UiNode::new(selected_id, Button::new("Selected")),
            UiNode::new(third_id, Button::new("Third")),
          ]),
        )
        .child(UiNode::new(outside_id, Box::new())),
    ])
    .unwrap();

  world
    .create(
      VisualElementCreate::new(group_id, UiNode::new(inserted_id, Button::new("New")))
        .child_index(1),
    )
    .unwrap();
  assert_eq!(
    world.element(group_id).unwrap().selected_indices(),
    Some(&[2][..])
  );

  world
    .update(VisualElementUpdate::Index {
      object_id: selected_id,
      child_index: 0,
    })
    .unwrap();
  assert_eq!(
    world.element(group_id).unwrap().selected_indices(),
    Some(&[0][..])
  );

  world.destroy(selected_id).unwrap();
  assert_eq!(
    world.element(group_id).unwrap().selected_indices(),
    Some(&[0][..])
  );

  world
    .update(VisualElementUpdate::Parent {
      object_id: first_id,
      parent_id: outside_id,
      child_index: None,
    })
    .unwrap();
  assert_eq!(
    world.element(group_id).unwrap().selected_indices(),
    Some(&[0][..])
  );
}

#[test]
fn reparent_cannot_exceed_toggle_group_mask_capacity() {
  let document_id = ObjectId::new_v4();
  let root_id = ObjectId::new_v4();
  let group_id = ObjectId::new_v4();
  let outside_id = ObjectId::new_v4();
  let moving_id = ObjectId::new_v4();
  let children = (0..64)
    .map(|index| UiNode::new(ObjectId::new_v4(), Button::new(index.to_string())))
    .collect::<Vec<_>>();
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::with_root_id(document_id, root_id)
        .child(
          UiNode::new(
            group_id,
            ToggleButtonGroup::new()
              .allow_empty_selection(true)
              .selected_indices([]),
          )
          .children(children),
        )
        .child(
          UiNode::new(outside_id, Box::new())
            .child(UiNode::new(moving_id, Button::new("Overflow"))),
        ),
    ])
    .unwrap();

  assert_eq!(
    world.update(VisualElementUpdate::Parent {
      object_id: moving_id,
      parent_id: group_id,
      child_index: None,
    }),
    Err(UiWorldError::InvalidHierarchy)
  );
  assert_eq!(
    world.element(moving_id).unwrap().parent_id(),
    Some(outside_id)
  );
}
