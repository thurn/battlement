use battlement::{Command, ParallelCommandGroup, UiButton, UiImage, UiLabel};

use crate::{
  ACTIVE_ADDRESS_ID, RENDER_IMAGE_ID, SOURCE_SWITCH_ID, SPRITE_IMAGE_ID, SWITCHED_IMAGE_ID,
  TEXTURE_IMAGE_ID, VECTOR_IMAGE_ID, asset_catalog::ui::assets, components,
};

pub(crate) fn switch_source(sprite_active: bool) -> Vec<ParallelCommandGroup<Command>> {
  let (image, address, action) = if sprite_active {
    (
      UiImage::new().source(assets::SPRITE.clone()),
      assets::SPRITE.as_str().to_owned(),
      "Show texture",
    )
  } else {
    (
      UiImage::new().source(assets::TEXTURE.clone()),
      assets::TEXTURE.as_str().to_owned(),
      "Show sprite",
    )
  };
  vec![ParallelCommandGroup::new(vec![
    Command::update_visual_element(SWITCHED_IMAGE_ID, image),
    Command::update_visual_element(ACTIVE_ADDRESS_ID, UiLabel::new(address)),
    Command::update_visual_element(SOURCE_SWITCH_ID, UiButton::new(action)),
  ])]
}

pub(crate) fn ids() -> components::AssetIds {
  components::AssetIds {
    texture: TEXTURE_IMAGE_ID,
    sprite: SPRITE_IMAGE_ID,
    vector: VECTOR_IMAGE_ID,
    render_texture: RENDER_IMAGE_ID,
    switched: SWITCHED_IMAGE_ID,
    active_address: ACTIVE_ADDRESS_ID,
    switch_action: SOURCE_SWITCH_ID,
  }
}
