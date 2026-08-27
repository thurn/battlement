use crate::{InteractionDistance, PanelInputConfiguration, UiValidationError};

/// Validates process-wide world-space input settings.
///
/// # Errors
///
/// Returns [`UiValidationError::InvalidProperty`] when a finite inclusive
/// interaction distance is negative or nonfinite.
pub fn validate_panel_input_configuration(
  value: &PanelInputConfiguration,
) -> Result<(), UiValidationError> {
  if let InteractionDistance::Inclusive(distance) = value.maximum_interaction_distance
    && (!distance.is_finite() || distance < 0.0)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}
