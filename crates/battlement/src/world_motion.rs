//! Host capability validation for shared transform Motion descriptors.

use crate::{MotionDescriptor, MotionTargetDescriptor, ObjectId, ValidationError};

pub(crate) fn validate(
  host: ObjectId,
  descriptor: Option<&MotionDescriptor>,
) -> Result<(), ValidationError> {
  let Some(descriptor) = descriptor else {
    return Ok(());
  };
  descriptor
    .validate()
    .map_err(|_| ValidationError::InvalidReference)?;
  if descriptor.host_id != host || descriptor.layout.is_some() {
    return Err(ValidationError::InvalidReference);
  }
  if !descriptor.animations.is_empty() || !descriptor.decorations.is_empty() {
    return Err(ValidationError::InvalidReference);
  }
  if !descriptor.pseudo_styles.is_empty() || descriptor.style_transition.all.is_some() {
    return Err(ValidationError::InvalidReference);
  }
  if !descriptor.style_transition.properties.is_empty() {
    return Err(ValidationError::InvalidReference);
  }
  if let Some(gestures) = descriptor.gestures {
    if gestures.pan || gestures.drag.is_some() {
      return Err(ValidationError::InvalidReference);
    }
    if gestures.in_view || gestures.scroll {
      return Err(ValidationError::InvalidReference);
    }
    if [
      gestures.scroll_x_value,
      gestures.scroll_y_value,
      gestures.in_view_value,
    ]
    .iter()
    .any(Option::is_some)
    {
      return Err(ValidationError::InvalidReference);
    }
    if gestures.subscriptions != crate::MotionGestureSubscriptions::default() {
      return Err(ValidationError::InvalidReference);
    }
  }
  for target in descriptor
    .initial
    .iter()
    .chain(descriptor.slots.iter().map(|slot| &slot.target))
    .chain(descriptor.named_targets.iter().map(|value| &value.target))
  {
    validate_target(target)?;
  }
  if descriptor.value_bindings.iter().any(|binding| {
    !binding.property.is_world_transform()
      || binding.composition == crate::MotionBindingComposition::Compose
  }) {
    return Err(ValidationError::InvalidReference);
  }
  Ok(())
}

fn validate_target(target: &MotionTargetDescriptor) -> Result<(), ValidationError> {
  let tracks = target.tracks.iter().map(|track| track.property);
  let end = target.transition_end.iter().map(|value| value.property);
  if tracks
    .chain(end)
    .any(|property| !property.is_world_transform())
  {
    return Err(ValidationError::InvalidReference);
  }
  Ok(())
}
