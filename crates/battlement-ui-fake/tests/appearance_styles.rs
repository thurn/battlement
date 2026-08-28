use battlement_types::{Color, MaterialAddress, ObjectId, SpriteAddress, TextureAddress};
use battlement_ui::{
  BackgroundSource, Box, Display, InlineKeyword, Overflow, Prop, Style, StyleValue, UiDocument,
  UiElement, UiNode, Visibility, VisualElementUpdate,
};
use battlement_ui_fake::{UiWorld, UiWorldError};

#[test]
fn appearance_updates_merge_atomically_and_move_material_usage() {
  let target_id = ObjectId::new_v4();
  let initial_material = MaterialAddress::new("ui/material/initial");
  let replacement_material = MaterialAddress::new("ui/material/replacement");
  let initial_background = BackgroundSource::Sprite(SpriteAddress::new("ui/panel/initial"));
  let replacement_background =
    BackgroundSource::Texture(TextureAddress::new("ui/panel/replacement"));
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
        target_id,
        Box::new().style(
          Style::new()
            .background_image(initial_background.clone())
            .border_color(Color::rgb(0.2, 0.8, 0.9))
            .border_radius((8, 16))
            .border_width(2)
            .display(Display::Flex)
            .overflow(Overflow::Hidden)
            .unity_material(initial_material.clone())
            .visibility(Visibility::Visible),
        ),
      )),
    ])
    .unwrap();

  assert_eq!(world.material_usage_count(&initial_material), 1);
  assert_eq!(world.background_usage_count(&initial_background), 1);
  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(
        Box::default().style(
          Style::new()
            .background_image(replacement_background.clone())
            .opacity(0.5)
            .unity_material(replacement_material.clone())
            .visibility(Visibility::Hidden),
        ),
      )
      .into(),
    })
    .unwrap();

  let committed = world.element(target_id).unwrap().style().clone();
  assert_eq!(
    committed.display,
    Prop::Set(StyleValue::Value(Display::Flex))
  );
  assert_eq!(committed.opacity, Some(StyleValue::Value(0.5.into())));
  assert_eq!(
    committed.visibility,
    Some(StyleValue::Value(Visibility::Hidden))
  );
  assert_eq!(world.material_usage_count(&initial_material), 0);
  assert_eq!(world.material_usage_count(&replacement_material), 1);
  assert_eq!(world.background_usage_count(&initial_background), 0);
  assert_eq!(world.background_usage_count(&replacement_background), 1);

  assert_eq!(
    world.update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(Box::default().style(Style::new().border_left_width(-1))).into(),
    }),
    Err(UiWorldError::InvalidProperty)
  );
  assert_eq!(world.element(target_id).unwrap().style(), &committed);
  assert_eq!(world.material_usage_count(&replacement_material), 1);
  assert_eq!(world.background_usage_count(&replacement_background), 1);

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(
        Box::default().style(Style::new().unity_material(InlineKeyword::Initial)),
      )
      .into(),
    })
    .unwrap();
  assert_eq!(world.material_usage_count(&replacement_material), 0);

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(
        Box::default().style(Style::new().background_image(InlineKeyword::Initial)),
      )
      .into(),
    })
    .unwrap();
  assert_eq!(world.background_usage_count(&replacement_background), 0);

  world.destroy(target_id).unwrap();
  assert_eq!(world.material_usage_count(&replacement_material), 0);
  assert_eq!(world.background_usage_count(&replacement_background), 0);
}
