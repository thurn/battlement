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
  let child_id = ObjectId::new_v4();
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

  let assigned = UiVisualElement {
    grid_item: Prop::Set(GridItem {
      column: Some(2),
      ..GridItem::default()
    }),
    stack_item: Prop::Set(StackItem {
      order: 9,
      align_self: Align::Center,
      ..StackItem::default()
    }),
    sticky: Prop::Set(Sticky {
      top: Some(-4.0),
      ..Sticky::default()
    }),
    overlay_placement: Prop::Set(OverlayPlacement::Layer(OverlayLayer::Popover)),
    ..UiVisualElement::new()
  };
  world
    .create(VisualElementCreate::new(
      root_id,
      UiNode::new(child_id, assigned.clone()),
    ))
    .unwrap();

  update(&mut world, child_id, UiVisualElement::new());
  assert_eq!(
    world.element(child_id).unwrap().element(),
    &UiElement::from(assigned)
  );

  update(
    &mut world,
    child_id,
    UiVisualElement {
      grid_item: Prop::Reset,
      stack_item: Prop::Reset,
      sticky: Prop::Reset,
      overlay_placement: Prop::Reset,
      ..UiVisualElement::new()
    },
  );
  let UiElement::VisualElement(value) = world.element(child_id).unwrap().element() else {
    panic!("unexpected element kind");
  };
  assert!(matches!(value.grid_item, Prop::Reset));
  assert!(matches!(value.stack_item, Prop::Reset));
  assert!(matches!(value.sticky, Prop::Reset));
  assert!(matches!(value.overlay_placement, Prop::Reset));
  assert!(matches!(
    world.journal(),
    [
      UiJournalEntry::Create(_),
      UiJournalEntry::Update(_),
      UiJournalEntry::Update(_)
    ]
  ));
}

fn update(world: &mut UiWorld, object_id: ObjectId, element: UiVisualElement) {
  world
    .update(VisualElementUpdate::Properties {
      object_id,
      element: std::boxed::Box::new(element.into()),
    })
    .unwrap();
}
