use battlement_types::ObjectId;
use battlement_ui::{
  NestedInteraction, Prop, ScrollViewMode, ScrollerVisibility, SliderDirection,
  TouchScrollBehavior, UiDocument, UiElement, UiNode, UiScrollView, UiScroller, Vector,
  VisualElementUpdate,
};
use battlement_ui_fake::{UiJournalEntry, UiWorld, UiWorldError};

#[test]
fn scrolling_properties_set_preserve_and_reset_without_remounting() {
  let scroll_id = ObjectId::new_v4();
  let scroller_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4())
        .child(UiNode::new(scroll_id, UiScrollView::new()))
        .child(UiNode::new(scroller_id, UiScroller::new())),
    ])
    .unwrap();

  update(
    &mut world,
    scroll_id,
    UiScrollView::new()
      .mode(ScrollViewMode::VerticalAndHorizontal)
      .nested_interaction(NestedInteraction::ForwardScrolling)
      .horizontal_scroller_visibility(ScrollerVisibility::AlwaysVisible)
      .vertical_scroller_visibility(ScrollerVisibility::Hidden)
      .scroll_offset(Vector::new(12.0, 24.0))
      .horizontal_page_size(0.75)
      .vertical_page_size(1.25)
      .mouse_wheel_scroll_size(36.0)
      .touch_scroll_behavior(TouchScrollBehavior::Elastic)
      .scroll_deceleration_rate(0.135)
      .elasticity(0.1)
      .elastic_animation_interval(16)
      .into(),
  );
  update(
    &mut world,
    scroller_id,
    UiScroller::new()
      .low_value(2.0)
      .high_value(100.0)
      .direction(SliderDirection::Horizontal)
      .value(25.0)
      .into(),
  );
  update(&mut world, scroll_id, UiScrollView::new().into());
  update(&mut world, scroller_id, UiScroller::new().into());

  let UiElement::ScrollView(scroll) = world.element(scroll_id).unwrap().element() else {
    panic!("expected scroll view");
  };
  assert_eq!(scroll.scroll_offset, Prop::Set(Vector::new(12.0, 24.0)));
  let UiElement::Scroller(scroller) = world.element(scroller_id).unwrap().element() else {
    panic!("expected scroller");
  };
  assert_eq!(scroller.value, Prop::Set(25.0));

  update(
    &mut world,
    scroll_id,
    UiScrollView::new()
      .mode(Prop::Reset)
      .nested_interaction(Prop::Reset)
      .horizontal_scroller_visibility(Prop::Reset)
      .vertical_scroller_visibility(Prop::Reset)
      .scroll_offset(Prop::Reset)
      .horizontal_page_size(Prop::Reset)
      .vertical_page_size(Prop::Reset)
      .mouse_wheel_scroll_size(Prop::Reset)
      .touch_scroll_behavior(Prop::Reset)
      .scroll_deceleration_rate(Prop::Reset)
      .elasticity(Prop::Reset)
      .elastic_animation_interval(Prop::Reset)
      .into(),
  );
  update(
    &mut world,
    scroller_id,
    UiScroller::new()
      .low_value(Prop::Reset)
      .high_value(Prop::Reset)
      .direction(Prop::Reset)
      .value(Prop::Reset)
      .into(),
  );

  let UiElement::ScrollView(scroll) = world.element(scroll_id).unwrap().element() else {
    panic!("expected scroll view");
  };
  assert_eq!(scroll.mode, Prop::Reset);
  assert_eq!(scroll.scroll_offset, Prop::Reset);
  assert_eq!(scroll.elastic_animation_interval, Prop::Reset);
  let UiElement::Scroller(scroller) = world.element(scroller_id).unwrap().element() else {
    panic!("expected scroller");
  };
  assert_eq!(scroller.low_value, Prop::Reset);
  assert_eq!(scroller.high_value, Prop::Reset);
  assert_eq!(scroller.direction, Prop::Reset);
  assert_eq!(scroller.value, Prop::Reset);
  assert_eq!(world.element(scroll_id).unwrap().object_id(), scroll_id);
  assert!(
    world
      .journal()
      .iter()
      .all(|entry| matches!(entry, UiJournalEntry::Update(_)))
  );
}

#[test]
fn scroller_limit_update_rejects_atomically_against_retained_state() {
  let scroller_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(
        scroller_id,
        UiScroller::new().low_value(0.0).high_value(10.0).value(5.0),
      ),
    )])
    .unwrap();
  let before = world.element(scroller_id).unwrap().element().clone();

  let result = world.update(VisualElementUpdate::Properties {
    object_id: scroller_id,
    element: std::boxed::Box::new(UiScroller::new().low_value(11.0).into()),
  });

  assert_eq!(result, Err(UiWorldError::InvalidProperty));
  assert_eq!(world.element(scroller_id).unwrap().element(), &before);
}

fn update(world: &mut UiWorld, object_id: ObjectId, element: UiElement) {
  world
    .update(VisualElementUpdate::Properties {
      object_id,
      element: std::boxed::Box::new(element),
    })
    .unwrap();
}
