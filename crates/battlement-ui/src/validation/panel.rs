use std::collections::HashSet;

use crate::PanelSettings;

use super::UiValidationError;

/// Validates panel settings before Unity creates or configures a runtime panel.
///
/// Numeric fields must be finite, dimensions and density values must be
/// positive, normalized values must fall in `0..=1`, and `target_display` must
/// fall in `0..=7`. Each scale mode accepts nondefault values only for its own
/// fields. Dynamic-atlas sizes must be ordered nonzero powers of two, and atlas
/// filters must be unique.
///
/// # Errors
///
/// Returns [`UiValidationError::InvalidProperty`] when any setting violates
/// these requirements. No input value is modified.
pub fn validate_panel_settings(value: &PanelSettings) -> Result<(), UiValidationError> {
  let floats = [value.reference_sprite_pixels_per_unit];
  if floats.iter().any(|number| !number.is_finite())
    || value.reference_sprite_pixels_per_unit <= 0.0
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if value.target_display > 7 {
    return Err(UiValidationError::InvalidProperty);
  }
  if value.target_texture.is_some()
    && (value.target_display != 0
      || value.render_mode != crate::PanelRenderMode::ScreenSpaceOverlay)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  for color in [
    value.color_clear_value.r,
    value.color_clear_value.g,
    value.color_clear_value.b,
    value.color_clear_value.a,
  ] {
    if !color.is_finite() || !(0.0..=1.0).contains(&color) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  let atlas = &value.dynamic_atlas;
  let atlas_sizes_are_powers = atlas.min_atlas_size.is_power_of_two()
    && atlas.max_atlas_size.is_power_of_two()
    && atlas.max_sub_texture_size.is_power_of_two();
  let atlas_sizes_are_ordered = atlas.min_atlas_size <= atlas.max_atlas_size
    && atlas.max_sub_texture_size <= atlas.max_atlas_size;
  if !atlas_sizes_are_powers || !atlas_sizes_are_ordered {
    return Err(UiValidationError::InvalidProperty);
  }
  if atlas.filters.iter().collect::<HashSet<_>>().len() != atlas.filters.len() {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}
