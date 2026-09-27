use battlement_types::ObjectId;
use battlement_ui::{
  Align, GridItem, OverlayLayer, OverlayPlacement, Prop, StackItem, Sticky, UiDocument, UiElement,
  UiFlex, UiGrid, UiNode, UiStack, UiVisualElement, VisualElementCreate, VisualElementUpdate,
};
use battlement_ui_fake::{UiJournalEntry, UiWorld};

#[test]
fn fake_reconstructs_layout_hosts_and_applies_sparse_descriptor_resets() {
  let document_id = ObjectId::new_v4();
  let root_id = ObjectId::new_v4();
  let flex_id = ObjectId::new_v4();
  let grid_id = ObjectId::new_v4();
  let stack_id = ObjectId::new_v4();
  let grid_child_id = ObjectId::new_v4();
  let stack_child_id = ObjectId::new_v4();
  let sticky_child_id = ObjectId::new_v4();
  let overlay_child_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::with_root_id(document_id, root_id)
        .child(UiNode::new(flex_id, UiFlex::new()))
        .child(UiNode::new(grid_id, UiGrid::new()))
        .child(UiNode::new(stack_id, UiStack::new())),
    ])
    .unwrap();

  assert!(matches!(
    world.element(flex_id).unwrap().element(),
    UiElement::Flex(_)
  ));
  assert!(matches!(
    world.element(grid_id).unwrap().element(),
    UiElement::Grid(_)
  ));
  assert!(matches!(
    world.element(stack_id).unwrap().element(),
    UiElement::Stack(_)
  ));

  let grid_child = UiVisualElement {
    grid_item: Prop::Set(GridItem {
      column: Some(2),
      ..GridItem::default()
    }),
    ..UiVisualElement::new()
  };
  let stack_child = UiVisualElement {
    stack_item: Prop::Set(StackItem {
      order: 9,
      align_self: Align::Center,
      ..StackItem::default()
    }),
    ..UiVisualElement::new()
  };
  let sticky_child = UiVisualElement {
    sticky: Prop::Set(Sticky {
      top: Some(-4.0),
      ..Sticky::default()
    }),
    ..UiVisualElement::new()
  };
  let overlay_child = UiVisualElement {
    overlay_placement: Prop::Set(OverlayPlacement::Layer(OverlayLayer::Popover)),
    ..UiVisualElement::new()
  };
  for (parent, id, value) in [
    (grid_id, grid_child_id, grid_child.clone()),
    (stack_id, stack_child_id, stack_child.clone()),
    (root_id, sticky_child_id, sticky_child.clone()),
    (root_id, overlay_child_id, overlay_child.clone()),
  ] {
    world
      .create(VisualElementCreate::new(parent, UiNode::new(id, value)))
      .unwrap();
    update(&mut world, id, UiVisualElement::new());
  }

  assert_eq!(
    world.element(grid_child_id).unwrap().element(),
    &UiElement::from(grid_child)
  );
  assert_eq!(
    world.element(stack_child_id).unwrap().element(),
    &UiElement::from(stack_child)
  );
  assert_eq!(
    world.element(sticky_child_id).unwrap().element(),
    &UiElement::from(sticky_child)
  );
  assert_eq!(
    world.element(overlay_child_id).unwrap().element(),
    &UiElement::from(overlay_child)
  );

  update(
    &mut world,
    grid_child_id,
    UiVisualElement {
      grid_item: Prop::Reset,
      ..UiVisualElement::new()
    },
  );
  update(
    &mut world,
    stack_child_id,
    UiVisualElement {
      stack_item: Prop::Reset,
      ..UiVisualElement::new()
    },
  );
  update(
    &mut world,
    sticky_child_id,
    UiVisualElement {
      sticky: Prop::Reset,
      ..UiVisualElement::new()
    },
  );
  update(
    &mut world,
    overlay_child_id,
    UiVisualElement {
      overlay_placement: Prop::Reset,
      ..UiVisualElement::new()
    },
  );
  let UiElement::VisualElement(grid) = world.element(grid_child_id).unwrap().element() else {
    panic!("unexpected element kind");
  };
  let UiElement::VisualElement(stack) = world.element(stack_child_id).unwrap().element() else {
    panic!("unexpected element kind");
  };
  let UiElement::VisualElement(sticky) = world.element(sticky_child_id).unwrap().element() else {
    panic!("unexpected element kind");
  };
  let UiElement::VisualElement(overlay) = world.element(overlay_child_id).unwrap().element() else {
    panic!("unexpected element kind");
  };
  assert!(matches!(grid.grid_item, Prop::Reset));
  assert!(matches!(stack.stack_item, Prop::Reset));
  assert!(matches!(sticky.sticky, Prop::Reset));
  assert!(matches!(overlay.overlay_placement, Prop::Reset));
  assert_eq!(world.journal().len(), 12);
  assert_eq!(
    world
      .journal()
      .iter()
      .filter(|entry| matches!(entry, UiJournalEntry::Create(_)))
      .count(),
    4
  );
}

fn update(world: &mut UiWorld, object_id: ObjectId, element: UiVisualElement) {
  world
    .update(VisualElementUpdate::Properties {
      object_id,
      element: std::boxed::Box::new(element.into()),
    })
    .unwrap();
}
