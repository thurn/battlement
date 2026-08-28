use battlement_types::ObjectId;
use battlement_ui::{Prop, Tab, TabView, UiDocument, UiElement, UiNode, VisualElementUpdate};
use battlement_ui_fake::{UiWorld, UiWorldError};

#[test]
fn navigation_resets_preserve_hierarchy_and_reject_invalid_selection_atomically() {
  let view_id = ObjectId::new_v4();
  let first_id = ObjectId::new_v4();
  let second_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4()).child(
        UiNode::new(
          view_id,
          TabView::new().selected_tab_index(1).reorderable(true),
        )
        .child(UiNode::new(first_id, Tab::new("First").closeable(true)))
        .child(UiNode::new(second_id, Tab::new("Second"))),
      ),
    ])
    .unwrap();

  assert_eq!(
    world.update(VisualElementUpdate::Properties {
      object_id: view_id,
      element: UiElement::from(TabView::new().selected_tab_index(4).reorderable(false),).into(),
    }),
    Err(UiWorldError::InvalidProperty)
  );
  let UiElement::TabView(view) = world.element(view_id).unwrap().element() else {
    panic!("expected tab view");
  };
  assert_eq!(view.selected_tab_index, Prop::Set(1));
  assert_eq!(view.reorderable, Prop::Set(true));

  world
    .update(VisualElementUpdate::Properties {
      object_id: view_id,
      element: UiElement::from(
        TabView::new()
          .selected_tab_index(Prop::Reset)
          .reorderable(Prop::Reset),
      )
      .into(),
    })
    .unwrap();
  world
    .update(VisualElementUpdate::Properties {
      object_id: first_id,
      element: UiElement::from(Tab::default().text(Prop::Reset).closeable(Prop::Reset)).into(),
    })
    .unwrap();

  let state = world.element(view_id).unwrap();
  assert_eq!(state.object_id(), view_id);
  assert_eq!(state.children(), [first_id, second_id]);
  let UiElement::TabView(view) = state.element() else {
    panic!("expected tab view");
  };
  assert_eq!(view.selected_tab_index, Prop::Reset);
  assert_eq!(view.reorderable, Prop::Reset);
  assert_eq!(world.element(first_id).unwrap().text(), None);
}
