use battlement_types::{Color, ObjectId, TextureAddress};
use battlement_ui::{
  BackgroundSource, Cursor, CursorHotspot, Prop, Slider, Style, Toggle, UiDocument, UiElement,
  UiNode, VisualElementUpdate,
};
use battlement_ui_fake::UiWorld;

#[test]
fn private_part_updates_merge_sparsely_and_release_asset_usage() {
  let target_id = ObjectId::new_v4();
  let initial = TextureAddress::new("ui/parts/initial");
  let replacement = TextureAddress::new("ui/parts/replacement");
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
        target_id,
        Toggle::new().text("Option").input_style(
          Style::new()
            .background_image(BackgroundSource::Texture(initial.clone()))
            .background_color(Color::rgb(0.1, 0.2, 0.3)),
        ),
      )),
    ])
    .unwrap();

  assert_eq!(
    world.background_usage_count(&BackgroundSource::Texture(initial.clone())),
    1
  );
  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(
        Toggle::new().input_style(Style::new().background_color(Color::rgb(0.4, 0.5, 0.6))),
      )
      .into(),
    })
    .unwrap();
  assert_eq!(
    world.background_usage_count(&BackgroundSource::Texture(initial.clone())),
    1
  );

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(Toggle::new().input_style(
        Style::new().background_image(BackgroundSource::Texture(replacement.clone())),
      ))
      .into(),
    })
    .unwrap();
  assert_eq!(
    world.background_usage_count(&BackgroundSource::Texture(initial)),
    0
  );
  assert_eq!(
    world.background_usage_count(&BackgroundSource::Texture(replacement.clone())),
    1
  );

  world.destroy(target_id).unwrap();
  assert_eq!(
    world.background_usage_count(&BackgroundSource::Texture(replacement)),
    0
  );
}

#[test]
fn private_part_resets_release_only_the_targeted_shared_asset_usage() {
  let target_id = ObjectId::new_v4();
  let address = TextureAddress::new("ui/parts/shared");
  let source = BackgroundSource::Texture(address.clone());
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
        target_id,
        Toggle::new().text("Option").input_style(
          Style::new()
            .background_image(source.clone())
            .cursor(Cursor::texture(
              address.clone(),
              CursorHotspot::new(2.0, 3.0),
            )),
        ),
      )),
    ])
    .unwrap();

  assert_eq!(world.background_usage_count(&source), 1);
  assert_eq!(world.cursor_usage_count(&address), 1);
  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(
        Toggle::new().input_style(Style::new().background_image(Prop::Reset)),
      )
      .into(),
    })
    .unwrap();
  assert_eq!(world.background_usage_count(&source), 0);
  assert_eq!(world.cursor_usage_count(&address), 1);

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(Toggle::new().input_style(Style::new().cursor(Prop::Reset))).into(),
    })
    .unwrap();
  assert_eq!(world.cursor_usage_count(&address), 0);
}

#[test]
fn conditional_part_updates_remove_dormant_style_and_asset_state() {
  let target_id = ObjectId::new_v4();
  let fill = TextureAddress::new("ui/parts/fill");
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
        target_id,
        Slider::new()
          .fill(true)
          .fill_style(Style::new().background_image(BackgroundSource::Texture(fill.clone()))),
      )),
    ])
    .unwrap();

  assert_eq!(
    world.background_usage_count(&BackgroundSource::Texture(fill.clone())),
    1
  );
  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(Slider::new().fill(false)).into(),
    })
    .unwrap();
  assert_eq!(
    world.background_usage_count(&BackgroundSource::Texture(fill.clone())),
    0
  );

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(
        Slider::new()
          .fill(true)
          .fill_style(Style::new().background_image(BackgroundSource::Texture(fill.clone()))),
      )
      .into(),
    })
    .unwrap();
  assert_eq!(
    world.background_usage_count(&BackgroundSource::Texture(fill)),
    1
  );
}
