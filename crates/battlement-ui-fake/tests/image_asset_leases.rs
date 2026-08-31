use battlement_types::{Color, ObjectId, Rect, SpriteAddress, TextureAddress};
use battlement_ui::{
  ImageSource, Prop, UiDocument, UiElement, UiImage, UiNode, VisualElementUpdate,
};
use battlement_ui_fake::{UiJournalEntry, UiWorld, UiWorldError};

#[test]
fn image_usage_counts_follow_replace_update_and_recursive_destroy() {
  let root_id = ObjectId::new_v4();
  let container_id = ObjectId::new_v4();
  let first_id = ObjectId::new_v4();
  let second_id = ObjectId::new_v4();
  let texture = ImageSource::Texture(TextureAddress::new("ui/shared"));
  let sprite = ImageSource::Sprite(SpriteAddress::new("ui/sprite"));
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(root_id).child(
        UiNode::new(container_id, battlement_ui::UiBox::new())
          .child(UiNode::new(
            first_id,
            UiImage::new().source(texture.clone()),
          ))
          .child(UiNode::new(
            second_id,
            UiImage::new().source(texture.clone()),
          )),
      ),
    ])
    .unwrap();

  assert_eq!(world.asset_usage_count(&texture), 2);
  world
    .update(VisualElementUpdate::Properties {
      object_id: first_id,
      element: std::boxed::Box::new(UiImage::new().source(sprite.clone()).into()),
    })
    .unwrap();
  assert_eq!(world.asset_usage_count(&texture), 1);
  assert_eq!(world.asset_usage_count(&sprite), 1);

  world.destroy(container_id).unwrap();
  assert_eq!(world.asset_usage_count(&texture), 0);
  assert_eq!(world.asset_usage_count(&sprite), 0);
}

#[test]
fn invalid_merged_image_state_preserves_source_and_usage_counts() {
  let image_id = ObjectId::new_v4();
  let sprite = ImageSource::Sprite(SpriteAddress::new("ui/sprite"));
  let mut world = UiWorld::default();
  world
    .replace(vec![UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(image_id, UiImage::new().source(sprite.clone())),
    )])
    .unwrap();

  assert_eq!(
    world.update(VisualElementUpdate::Properties {
      object_id: image_id,
      element: std::boxed::Box::new(
        UiImage::new()
          .source_rect(Rect::new(0.0, 0.0, 16.0, 16.0))
          .into(),
      ),
    }),
    Err(UiWorldError::InvalidProperty)
  );
  assert_eq!(
    world.element(image_id).unwrap().image_source(),
    Some(&sprite)
  );
  assert_eq!(world.asset_usage_count(&sprite), 1);
  assert!(world.journal().is_empty());
}

#[test]
fn image_source_and_tint_reset_without_recreating_the_element() {
  let image_id = ObjectId::new_v4();
  let texture = ImageSource::Texture(TextureAddress::new("ui/resettable"));
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
        image_id,
        UiImage::new()
          .source(texture.clone())
          .tint_color(Color::rgba(0.2, 0.4, 0.6, 0.8)),
      )),
    ])
    .unwrap();

  world
    .update(VisualElementUpdate::Properties {
      object_id: image_id,
      element: std::boxed::Box::new(UiImage::new().into()),
    })
    .unwrap();
  assert_eq!(
    world.element(image_id).unwrap().image_source(),
    Some(&texture)
  );

  world
    .update(VisualElementUpdate::Properties {
      object_id: image_id,
      element: std::boxed::Box::new(
        UiImage::new()
          .source(Prop::Reset)
          .tint_color(Prop::Reset)
          .into(),
      ),
    })
    .unwrap();

  let state = world.element(image_id).unwrap();
  let UiElement::Image(image) = state.element() else {
    panic!("expected image");
  };
  assert_eq!(state.object_id(), image_id);
  assert_eq!(state.image_source(), None);
  assert_eq!(image.tint_color, Prop::Reset);
  assert_eq!(world.asset_usage_count(&texture), 0);
  assert!(matches!(
    world.journal().last(),
    Some(UiJournalEntry::Update(_))
  ));
}
