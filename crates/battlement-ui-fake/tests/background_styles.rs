use battlement_types::{ObjectId, TextureAddress};
use battlement_ui::{
  BackgroundPosition, BackgroundPositionKeyword, BackgroundRepeat, BackgroundRepeatMode,
  BackgroundSize, Box, Cursor, CursorHotspot, InlineKeyword, LengthUnits, Style, UiDocument,
  UiElement, UiNode, VisualElementUpdate,
};
use battlement_ui_fake::{UiWorld, UiWorldError};

#[test]
fn background_and_cursor_updates_merge_atomically_and_release_cursor_usage() {
  let target_id = ObjectId::new_v4();
  let initial_cursor = TextureAddress::new("ui/cursor/initial");
  let replacement_cursor = TextureAddress::new("ui/cursor/replacement");
  let initial_style = Style::new()
    .background_position_x(BackgroundPosition::new(
      BackgroundPositionKeyword::Right,
      12.pct(),
    ))
    .background_position_y(BackgroundPosition::new(
      BackgroundPositionKeyword::Bottom,
      8,
    ))
    .background_repeat(BackgroundRepeat::new(
      BackgroundRepeatMode::Space,
      BackgroundRepeatMode::Round,
    ))
    .background_size(BackgroundSize::axes(45.pct(), 72))
    .cursor(Cursor::texture(
      initial_cursor.clone(),
      CursorHotspot::new(2.0, 3.0),
    ));
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4())
        .child(UiNode::new(target_id, Box::new().style(initial_style))),
    ])
    .unwrap();

  assert_eq!(world.cursor_usage_count(&initial_cursor), 1);
  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(Box::default().style(Style::new().cursor(Cursor::texture(
        replacement_cursor.clone(),
        CursorHotspot::new(4.0, 5.0),
      ))))
      .into(),
    })
    .unwrap();
  let committed = world.element(target_id).unwrap().style().clone();
  assert_eq!(world.cursor_usage_count(&initial_cursor), 0);
  assert_eq!(world.cursor_usage_count(&replacement_cursor), 1);

  assert_eq!(
    world.update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(
        Box::default().style(Style::new().background_size(BackgroundSize::axes(-1, 10),))
      )
      .into(),
    }),
    Err(UiWorldError::InvalidProperty)
  );
  assert_eq!(world.element(target_id).unwrap().style(), &committed);
  assert_eq!(world.cursor_usage_count(&replacement_cursor), 1);

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(Box::default().style(Style::new().cursor(InlineKeyword::Initial)))
        .into(),
    })
    .unwrap();
  assert_eq!(world.cursor_usage_count(&replacement_cursor), 0);
  world.destroy(target_id).unwrap();
}
