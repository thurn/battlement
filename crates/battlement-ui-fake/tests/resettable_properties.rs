use battlement_types::ObjectId;
use battlement_ui::{
  Prop, UiDocument, UiNode, VisualElement, VisualElementCreate, VisualElementUpdate,
};
use battlement_ui_fake::{UiJournalEntry, UiWorld};

#[test]
fn enabled_journal_covers_create_set_omitted_and_reset() {
  let document_id = ObjectId::new_v4();
  let root_id = ObjectId::new_v4();
  let element_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![UiDocument::with_root_id(document_id, root_id)])
    .unwrap();

  world
    .create(VisualElementCreate::new(
      root_id,
      UiNode::new(element_id, VisualElement::new()),
    ))
    .unwrap();
  assert_eq!(world.element(element_id).unwrap().is_enabled(), None);

  update_enabled(&mut world, element_id, Prop::Set(false));
  assert_eq!(world.element(element_id).unwrap().is_enabled(), Some(false));

  update_enabled(&mut world, element_id, Prop::Unset);
  assert_eq!(world.element(element_id).unwrap().is_enabled(), Some(false));

  update_enabled(&mut world, element_id, Prop::Reset);
  assert_eq!(world.element(element_id).unwrap().is_enabled(), Some(true));
  assert!(matches!(
    world.journal(),
    [
      UiJournalEntry::Create(_),
      UiJournalEntry::Update(_),
      UiJournalEntry::Update(_),
      UiJournalEntry::Update(_),
    ]
  ));
}

fn update_enabled(world: &mut UiWorld, object_id: ObjectId, enabled: Prop<bool>) {
  world
    .update(VisualElementUpdate::Properties {
      object_id,
      element: std::boxed::Box::new(VisualElement::new().enabled(enabled).into()),
    })
    .unwrap();
}
