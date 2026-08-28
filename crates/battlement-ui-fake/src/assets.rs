use battlement_types::{MaterialAddress, TextureAddress};
use battlement_ui::{
  BackgroundSource, Cursor, IconSource, ImageSource, Prop, StyleValue, UiElement,
  authored_private_part_styles,
};

use crate::UiWorld;

impl UiWorld {
  /// Returns the number of live image properties retaining one prepared source.
  #[must_use]
  pub fn asset_usage_count(&self, source: &ImageSource) -> usize {
    self.asset_usage.get(source).copied().unwrap_or(0)
  }

  /// Iterates over prepared image sources and their positive live usage counts.
  pub fn asset_usage(&self) -> impl Iterator<Item = (&ImageSource, &usize)> {
    self.asset_usage.iter()
  }

  /// Returns the number of live button properties retaining one prepared icon.
  #[must_use]
  pub fn icon_usage_count(&self, source: &IconSource) -> usize {
    self.icon_usage.get(source).copied().unwrap_or(0)
  }

  /// Returns the number of live inline styles retaining a prepared background source.
  #[must_use]
  pub fn background_usage_count(&self, source: &BackgroundSource) -> usize {
    self.background_usage.get(source).copied().unwrap_or(0)
  }

  /// Returns the number of live inline cursors retaining a prepared texture.
  #[must_use]
  pub fn cursor_usage_count(&self, source: &TextureAddress) -> usize {
    self.cursor_usage.get(source).copied().unwrap_or(0)
  }

  /// Returns the number of live inline styles retaining a prepared material.
  #[must_use]
  pub fn material_usage_count(&self, source: &MaterialAddress) -> usize {
    self.material_usage.get(source).copied().unwrap_or(0)
  }

  pub(super) fn retain_source(&mut self, source: ImageSource) {
    *self.asset_usage.entry(source).or_default() += 1;
  }

  pub(super) fn release_source(&mut self, source: &ImageSource) {
    let count = self
      .asset_usage
      .get_mut(source)
      .expect("live image source had no usage count");
    *count -= 1;
    if *count == 0 {
      self.asset_usage.remove(source);
    }
  }

  pub(super) fn retain_icon(&mut self, source: IconSource) {
    *self.icon_usage.entry(source).or_default() += 1;
  }

  pub(super) fn release_icon(&mut self, source: &IconSource) {
    let count = self
      .icon_usage
      .get_mut(source)
      .expect("live button icon had no usage count");
    *count -= 1;
    if *count == 0 {
      self.icon_usage.remove(source);
    }
  }

  pub(super) fn retain_material(&mut self, source: MaterialAddress) {
    *self.material_usage.entry(source).or_default() += 1;
  }

  pub(super) fn retain_background(&mut self, source: BackgroundSource) {
    *self.background_usage.entry(source).or_default() += 1;
  }

  pub(super) fn retain_cursor(&mut self, source: TextureAddress) {
    *self.cursor_usage.entry(source).or_default() += 1;
  }

  pub(super) fn release_cursor(&mut self, source: &TextureAddress) {
    let count = self
      .cursor_usage
      .get_mut(source)
      .expect("live UI cursor had no usage count");
    *count -= 1;
    if *count == 0 {
      self.cursor_usage.remove(source);
    }
  }

  pub(super) fn release_background(&mut self, source: &BackgroundSource) {
    let count = self
      .background_usage
      .get_mut(source)
      .expect("live UI background had no usage count");
    *count -= 1;
    if *count == 0 {
      self.background_usage.remove(source);
    }
  }

  pub(super) fn release_material(&mut self, source: &MaterialAddress) {
    let count = self
      .material_usage
      .get_mut(source)
      .expect("live UI material had no usage count");
    *count -= 1;
    if *count == 0 {
      self.material_usage.remove(source);
    }
  }

  pub(super) fn retain_part_assets(&mut self, assets: PartAssets) {
    for source in assets.backgrounds {
      self.retain_background(source);
    }
    for source in assets.cursors {
      self.retain_cursor(source);
    }
    for source in assets.materials {
      self.retain_material(source);
    }
  }

  pub(super) fn release_part_assets(&mut self, assets: PartAssets) {
    for source in assets.backgrounds {
      self.release_background(&source);
    }
    for source in assets.cursors {
      self.release_cursor(&source);
    }
    for source in assets.materials {
      self.release_material(&source);
    }
  }
}

#[derive(Default)]
pub(super) struct PartAssets {
  backgrounds: Vec<BackgroundSource>,
  cursors: Vec<TextureAddress>,
  materials: Vec<MaterialAddress>,
}

pub(super) fn part_assets(element: &UiElement) -> PartAssets {
  let mut result = PartAssets::default();
  for style in authored_private_part_styles(element) {
    if let Prop::Set(StyleValue::Value(value)) = &style.background_image {
      result.backgrounds.push(value.clone());
    }
    if let Prop::Set(StyleValue::Value(Cursor::Texture { address, .. })) = &style.cursor {
      result.cursors.push(address.clone());
    }
    if let Prop::Set(StyleValue::Value(value)) = &style.unity_material {
      result.materials.push(value.clone());
    }
  }
  result
}
