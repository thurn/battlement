use battlement_types::ObjectId;
use battlement_ui::{
  LanguageDirection, PickingMode, Prop, UiDocument, UiEventKind, UiEventPhase, UiEventSubscription,
  UiNode, UiVisualElement, VisualElementCreate, VisualElementUpdate,
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
      UiNode::new(element_id, UiVisualElement::new()),
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
      element: std::boxed::Box::new(UiVisualElement::new().enabled(enabled).into()),
    })
    .unwrap();
}

#[test]
fn shared_visual_state_round_trips_through_set_omit_and_reset() {
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
      UiNode::new(element_id, UiVisualElement::new()),
    ))
    .unwrap();

  update_shared(
    &mut world,
    element_id,
    UiVisualElement::new()
      .name("changed")
      .enabled(false)
      .picking_mode(PickingMode::Ignore)
      .language_direction(LanguageDirection::Rtl)
      .focusable(true)
      .tab_index(7)
      .delegates_focus(true)
      .class("changed")
      .events([UiEventKind::Click])
      .event_subscriptions([UiEventSubscription::new(
        UiEventKind::PointerDown,
        UiEventPhase::Bubble,
      )]),
  );
  require_changed_state(&world, element_id);

  update_shared(&mut world, element_id, UiVisualElement::new());
  require_changed_state(&world, element_id);

  update_shared(
    &mut world,
    element_id,
    UiVisualElement {
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
      ..UiVisualElement::new()
    },
  );

  let state = world.element(element_id).unwrap();
  assert_eq!(state.name(), Some(""));
  assert_eq!(state.is_enabled(), Some(true));
  assert_eq!(state.picking_mode(), Some(PickingMode::Position));
  assert_eq!(state.language_direction(), Some(LanguageDirection::Inherit));
  assert_eq!(state.is_focusable(), Some(false));
  assert_eq!(state.tab_index(), Some(0));
  assert_eq!(state.delegates_focus(), Some(false));
  assert_eq!(state.classes(), Some([].as_slice()));
  assert_eq!(state.events(), Some([].as_slice()));
  assert!(!world.has_subscription(element_id, UiEventKind::Click));
  assert!(!world.has_phase_subscription(
    element_id,
    UiEventKind::PointerDown,
    UiEventPhase::Bubble
  ));
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

fn update_shared(world: &mut UiWorld, object_id: ObjectId, element: UiVisualElement) {
  world
    .update(VisualElementUpdate::Properties {
      object_id,
      element: std::boxed::Box::new(element.into()),
    })
    .unwrap();
}

fn require_changed_state(world: &UiWorld, object_id: ObjectId) {
  let state = world.element(object_id).unwrap();
  assert_eq!(state.name(), Some("changed"));
  assert_eq!(state.is_enabled(), Some(false));
  assert_eq!(state.picking_mode(), Some(PickingMode::Ignore));
  assert_eq!(state.language_direction(), Some(LanguageDirection::Rtl));
  assert_eq!(state.is_focusable(), Some(true));
  assert_eq!(state.tab_index(), Some(7));
  assert_eq!(state.delegates_focus(), Some(true));
  assert_eq!(state.classes(), Some(["changed".to_owned()].as_slice()));
  assert_eq!(state.events(), Some([UiEventKind::Click].as_slice()));
  assert!(world.has_subscription(object_id, UiEventKind::Click));
  assert!(world.has_phase_subscription(object_id, UiEventKind::PointerDown, UiEventPhase::Bubble));
}
