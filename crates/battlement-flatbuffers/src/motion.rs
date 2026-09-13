use std::collections::HashSet;

use crate::{
  ProtocolError, motion_generated::battlement::flat_buffers::generated as wire,
  ui_event::uuid_bytes,
};

const MAXIMUM_RECORDS: usize = 262_144;

/// A semantically validated, borrowed Motion event batch.
#[derive(Clone, Copy)]
pub struct MotionEventBatchView<'a> {
  value: wire::MotionEventBatch<'a>,
}

impl<'a> MotionEventBatchView<'a> {
  pub(crate) fn new(value: wire::MotionEventBatch<'a>) -> Result<Self, ProtocolError> {
    validate_batch(value)?;
    Ok(Self { value })
  }

  /// Returns the inclusive first lifecycle sequence.
  #[must_use]
  pub fn first_sequence(self) -> u64 {
    self.value.first_sequence()
  }

  /// Returns the inclusive last lifecycle sequence.
  #[must_use]
  pub fn last_sequence(self) -> u64 {
    self.value.last_sequence()
  }

  /// Returns the number of lifecycle records.
  #[must_use]
  pub fn lifecycle_count(self) -> usize {
    self.value.events().len()
  }

  /// Returns the number of presentation samples.
  #[must_use]
  pub fn presentation_sample_count(self) -> usize {
    self.value.samples().len()
  }

  /// Returns the number of explicit value samples.
  #[must_use]
  pub fn value_sample_count(self) -> usize {
    self.value.value_samples().len()
  }

  /// Returns the number of playback records.
  #[must_use]
  pub fn playback_count(self) -> usize {
    self.value.playback_events().len()
  }

  /// Returns the number of gesture records.
  #[must_use]
  pub fn gesture_count(self) -> usize {
    self.value.gesture_events().len()
  }

  /// Iterates lifecycle records, allocating no batch collection.
  pub fn lifecycle_events(
    self,
  ) -> impl ExactSizeIterator<Item = battlement::MotionLifecycleEvent> + 'a {
    self.value.events().iter().map(lifecycle_owned)
  }

  /// Iterates presentation samples, owning only each sample's nested values.
  pub fn presentation_samples(
    self,
  ) -> impl ExactSizeIterator<Item = battlement::MotionPresentationSample> + 'a {
    self.value.samples().iter().map(presentation_owned)
  }

  /// Iterates values that the Motion registry deliberately retains.
  pub fn value_samples(self) -> impl ExactSizeIterator<Item = battlement::MotionValueSample> + 'a {
    self.value.value_samples().iter().map(value_sample_owned)
  }

  /// Iterates playback completion records without a batch allocation.
  pub fn playback_events(
    self,
  ) -> impl ExactSizeIterator<Item = battlement::MotionPlaybackEvent> + 'a {
    self.value.playback_events().iter().map(playback_owned)
  }

  /// Iterates gesture records without a batch allocation.
  pub fn gesture_events(
    self,
  ) -> impl ExactSizeIterator<Item = battlement::MotionGestureEvent> + 'a {
    self.value.gesture_events().iter().map(gesture_owned)
  }

  /// Copies the validated records into the retained Motion runtime model.
  ///
  /// Native engines should prefer the borrowed accessors. This conversion is
  /// intended for registries that deliberately retain every value after the
  /// request pin is released.
  pub fn to_owned(self) -> battlement::MotionEventBatch {
    battlement::MotionEventBatch {
      first_sequence: battlement::MotionSequence(self.first_sequence()),
      last_sequence: battlement::MotionSequence(self.last_sequence()),
      events: self.lifecycle_events().collect(),
      samples: self.presentation_samples().collect(),
      value_samples: self.value_samples().collect(),
      playback_events: self.playback_events().collect(),
      gesture_events: self.gesture_events().collect(),
    }
  }
}

fn lifecycle_owned(event: wire::MotionLifecycleEvent<'_>) -> battlement::MotionLifecycleEvent {
  use battlement::{MotionEventKind, MotionGeneration, MotionSequence, MotionSlotId};
  battlement::MotionLifecycleEvent {
    sequence: MotionSequence(event.sequence()),
    descriptor_id: object_id(event.descriptor_id()),
    slot: MotionSlotId(event.slot()),
    generation: MotionGeneration(event.generation()),
    elapsed_micros: event.elapsed_micros(),
    kind: match event.kind() {
      wire::MotionLifecycleKind::Activated => MotionEventKind::Activated,
      wire::MotionLifecycleKind::Started => MotionEventKind::Started,
      wire::MotionLifecycleKind::Repeated => MotionEventKind::Repeated {
        first: event.repeat_first(),
        last: event.repeat_last(),
      },
      wire::MotionLifecycleKind::Completed => MotionEventKind::Completed,
      wire::MotionLifecycleKind::Stopped => MotionEventKind::Stopped,
      wire::MotionLifecycleKind::Cancelled => MotionEventKind::Cancelled,
      _ => unreachable!("Motion view validates lifecycle kinds"),
    },
  }
}

fn presentation_owned(
  sample: wire::MotionPresentationSample<'_>,
) -> battlement::MotionPresentationSample {
  battlement::MotionPresentationSample {
    descriptor_id: object_id(sample.descriptor_id()),
    slot: battlement::MotionSlotId(sample.slot()),
    generation: battlement::MotionGeneration(sample.generation()),
    elapsed_micros: sample.elapsed_micros(),
    values: sample.values().iter().map(property_owned).collect(),
  }
}

fn value_sample_owned(sample: wire::MotionValueSample<'_>) -> battlement::MotionValueSample {
  battlement::MotionValueSample {
    subscription_id: object_id(sample.subscription_id()),
    value_id: object_id(sample.value_id()),
    frame: sample.frame(),
    value: sampled_owned(sample, false),
    velocity: sampled_owned(sample, true),
    discontinuity: sample.discontinuity(),
  }
}

fn playback_owned(event: wire::MotionPlaybackEvent<'_>) -> battlement::MotionPlaybackEvent {
  battlement::MotionPlaybackEvent {
    playback_id: object_id(event.playback_id()),
    generation: event.generation(),
    outcome: match event.outcome() {
      wire::MotionPlaybackOutcome::Completed => battlement::MotionPlaybackOutcome::Completed,
      wire::MotionPlaybackOutcome::Stopped => battlement::MotionPlaybackOutcome::Stopped,
      wire::MotionPlaybackOutcome::Cancelled => battlement::MotionPlaybackOutcome::Cancelled,
      _ => unreachable!("Motion view validates playback outcomes"),
    },
  }
}

fn gesture_owned(event: wire::MotionGestureEvent<'_>) -> battlement::MotionGestureEvent {
  use battlement::{MotionGestureAxis, MotionGestureEventKind, MotionPointerDevice};
  battlement::MotionGestureEvent {
    descriptor_id: object_id(event.descriptor_id()),
    generation: battlement::MotionGeneration(event.generation()),
    kind: match event.kind().0 {
      0 => MotionGestureEventKind::HoverStart,
      1 => MotionGestureEventKind::HoverEnd,
      2 => MotionGestureEventKind::TapStart,
      3 => MotionGestureEventKind::Tap,
      4 => MotionGestureEventKind::TapCancel,
      5 => MotionGestureEventKind::FocusStart,
      6 => MotionGestureEventKind::FocusEnd,
      7 => MotionGestureEventKind::FocusVisibleStart,
      8 => MotionGestureEventKind::FocusVisibleEnd,
      9 => MotionGestureEventKind::PanSessionStart,
      10 => MotionGestureEventKind::PanStart,
      11 => MotionGestureEventKind::Pan,
      12 => MotionGestureEventKind::PanEnd,
      13 => MotionGestureEventKind::PanCancel,
      14 => MotionGestureEventKind::DragStart,
      15 => MotionGestureEventKind::DragDirectionLock,
      16 => MotionGestureEventKind::Drag,
      17 => MotionGestureEventKind::DragEnd,
      18 => MotionGestureEventKind::DragCancel,
      19 => MotionGestureEventKind::DragMomentumComplete,
      20 => MotionGestureEventKind::DragConstraintsMeasured,
      21 => MotionGestureEventKind::Scroll,
      22 => MotionGestureEventKind::InViewEnter,
      23 => MotionGestureEventKind::InViewLeave,
      _ => unreachable!("Motion view validates gesture kinds"),
    },
    pointer_id: event.pointer_id(),
    device: match event.device() {
      wire::MotionPointerDevice::Mouse => MotionPointerDevice::Mouse,
      wire::MotionPointerDevice::Pen => MotionPointerDevice::Pen,
      wire::MotionPointerDevice::Touch => MotionPointerDevice::Touch,
      wire::MotionPointerDevice::Keyboard => MotionPointerDevice::Keyboard,
      wire::MotionPointerDevice::Gamepad => MotionPointerDevice::Gamepad,
      _ => unreachable!("Motion view validates pointer devices"),
    },
    point: gesture_vector(event.point()),
    delta: gesture_vector(event.delta()),
    offset: gesture_vector(event.offset()),
    velocity: gesture_vector(event.velocity()),
    axis: event.has_axis().then(|| match event.axis() {
      wire::MotionGestureAxis::X => MotionGestureAxis::X,
      wire::MotionGestureAxis::Y => MotionGestureAxis::Y,
      wire::MotionGestureAxis::Both => MotionGestureAxis::Both,
      _ => unreachable!("Motion view validates gesture axes"),
    }),
    momentum_generation: event.momentum_generation(),
    constrained: event.constrained(),
  }
}

fn property_owned(value: wire::MotionPropertyValue<'_>) -> battlement::MotionPropertyValue {
  battlement::MotionPropertyValue {
    property: battlement::MotionProperty::ALL[usize::from(value.property().0)],
    value: property_value_owned(value),
  }
}

fn property_value_owned(value: wire::MotionPropertyValue<'_>) -> battlement::MotionValue {
  match value.value_type() {
    wire::MotionValue::ScalarMotionValue => {
      battlement::MotionValue::Scalar(value.value_as_scalar_motion_value().unwrap().value())
    }
    wire::MotionValue::LengthMotionValue => battlement::MotionValue::Length(owned_length(
      value.value_as_length_motion_value().unwrap().value(),
    )),
    wire::MotionValue::ColorMotionValue => battlement::MotionValue::Color(owned_color(
      value.value_as_color_motion_value().unwrap().value(),
    )),
    wire::MotionValue::Vector2MotionValue => battlement::MotionValue::Vector2(owned_vector2(
      value.value_as_vector_2_motion_value().unwrap().value(),
    )),
    wire::MotionValue::Vector3MotionValue => battlement::MotionValue::Vector3(owned_vector3(
      value.value_as_vector_3_motion_value().unwrap().value(),
    )),
    wire::MotionValue::AngleMotionValue => {
      battlement::MotionValue::Angle(value.value_as_angle_motion_value().unwrap().value())
    }
    wire::MotionValue::TransformListMotionValue => {
      owned_transforms(value.value_as_transform_list_motion_value().unwrap())
    }
    wire::MotionValue::FilterListMotionValue => {
      owned_filters(value.value_as_filter_list_motion_value().unwrap())
    }
    wire::MotionValue::ShadowListMotionValue => {
      owned_shadows(value.value_as_shadow_list_motion_value().unwrap())
    }
    wire::MotionValue::GradientMotionValue => {
      owned_gradient(value.value_as_gradient_motion_value().unwrap())
    }
    wire::MotionValue::ClipInsetMotionValue => {
      owned_inset(value.value_as_clip_inset_motion_value().unwrap())
    }
    wire::MotionValue::ClipPolygonMotionValue => {
      owned_polygon(value.value_as_clip_polygon_motion_value().unwrap())
    }
    wire::MotionValue::DiscreteMotionValue => {
      owned_discrete(value.value_as_discrete_motion_value().unwrap())
    }
    _ => unreachable!("Motion view validates value unions"),
  }
}

fn sampled_owned(value: wire::MotionValueSample<'_>, velocity: bool) -> battlement::MotionValue {
  let kind = if velocity {
    value.velocity_type()
  } else {
    value.value_type()
  };
  macro_rules! selected {
    ($value_method:ident, $velocity_method:ident) => {
      if velocity {
        value.$velocity_method().unwrap()
      } else {
        value.$value_method().unwrap()
      }
    };
  }
  match kind {
    wire::MotionValue::ScalarMotionValue => battlement::MotionValue::Scalar(
      selected!(
        value_as_scalar_motion_value,
        velocity_as_scalar_motion_value
      )
      .value(),
    ),
    wire::MotionValue::LengthMotionValue => battlement::MotionValue::Length(owned_length(
      selected!(
        value_as_length_motion_value,
        velocity_as_length_motion_value
      )
      .value(),
    )),
    wire::MotionValue::ColorMotionValue => battlement::MotionValue::Color(owned_color(
      selected!(value_as_color_motion_value, velocity_as_color_motion_value).value(),
    )),
    wire::MotionValue::Vector2MotionValue => battlement::MotionValue::Vector2(owned_vector2(
      selected!(
        value_as_vector_2_motion_value,
        velocity_as_vector_2_motion_value
      )
      .value(),
    )),
    wire::MotionValue::Vector3MotionValue => battlement::MotionValue::Vector3(owned_vector3(
      selected!(
        value_as_vector_3_motion_value,
        velocity_as_vector_3_motion_value
      )
      .value(),
    )),
    wire::MotionValue::AngleMotionValue => battlement::MotionValue::Angle(
      selected!(value_as_angle_motion_value, velocity_as_angle_motion_value).value(),
    ),
    wire::MotionValue::TransformListMotionValue => owned_transforms(selected!(
      value_as_transform_list_motion_value,
      velocity_as_transform_list_motion_value
    )),
    wire::MotionValue::FilterListMotionValue => owned_filters(selected!(
      value_as_filter_list_motion_value,
      velocity_as_filter_list_motion_value
    )),
    wire::MotionValue::ShadowListMotionValue => owned_shadows(selected!(
      value_as_shadow_list_motion_value,
      velocity_as_shadow_list_motion_value
    )),
    wire::MotionValue::GradientMotionValue => owned_gradient(selected!(
      value_as_gradient_motion_value,
      velocity_as_gradient_motion_value
    )),
    wire::MotionValue::ClipInsetMotionValue => owned_inset(selected!(
      value_as_clip_inset_motion_value,
      velocity_as_clip_inset_motion_value
    )),
    wire::MotionValue::ClipPolygonMotionValue => owned_polygon(selected!(
      value_as_clip_polygon_motion_value,
      velocity_as_clip_polygon_motion_value
    )),
    wire::MotionValue::DiscreteMotionValue => owned_discrete(selected!(
      value_as_discrete_motion_value,
      velocity_as_discrete_motion_value
    )),
    _ => unreachable!("Motion view validates sampled value unions"),
  }
}

fn owned_length(value: &wire::MotionLength) -> battlement::Length {
  battlement::Length::calc(value.pixels(), value.percentage())
}

fn owned_color(value: &wire::MotionColor) -> battlement::Color {
  battlement::Color {
    r: value.red(),
    g: value.green(),
    b: value.blue(),
    a: value.alpha(),
  }
}

fn owned_vector2(value: &wire::MotionVector2) -> [f32; 2] {
  [value.x(), value.y()]
}

fn owned_vector3(value: &wire::MotionVector3) -> [f32; 3] {
  [value.x(), value.y(), value.z()]
}

fn owned_shadow(value: &wire::MotionShadow) -> battlement::Shadow {
  battlement::Shadow {
    x: value.x(),
    y: value.y(),
    blur: value.blur(),
    spread: value.spread(),
    color: owned_color(value.color()),
    inset: value.inset(),
  }
}

fn owned_transforms(value: wire::TransformListMotionValue<'_>) -> battlement::MotionValue {
  battlement::MotionValue::TransformList(
    value
      .values()
      .iter()
      .map(|item| match item.kind() {
        wire::MotionTransformKind::Translate => battlement::TransformOperation::Translate(
          item
            .lengths()
            .unwrap()
            .iter()
            .map(owned_length)
            .collect::<Vec<_>>()
            .try_into()
            .expect("writer emits three translation channels"),
        ),
        wire::MotionTransformKind::Rotate => battlement::TransformOperation::Rotate(
          item
            .scalars()
            .unwrap()
            .iter()
            .collect::<Vec<_>>()
            .try_into()
            .expect("writer emits three rotation channels"),
        ),
        wire::MotionTransformKind::Skew => battlement::TransformOperation::Skew(
          item
            .scalars()
            .unwrap()
            .iter()
            .collect::<Vec<_>>()
            .try_into()
            .expect("writer emits two skew channels"),
        ),
        wire::MotionTransformKind::Scale => battlement::TransformOperation::Scale(
          item
            .scalars()
            .unwrap()
            .iter()
            .collect::<Vec<_>>()
            .try_into()
            .expect("writer emits three scale channels"),
        ),
        _ => unreachable!("Motion view validates transform kinds"),
      })
      .collect(),
  )
}

fn owned_filters(value: wire::FilterListMotionValue<'_>) -> battlement::MotionValue {
  battlement::MotionValue::FilterList(battlement::FilterList::new(value.values().iter().map(
    |item| match item.kind() {
      wire::MotionFilterKind::Brightness => {
        battlement::FilterFunction::Brightness(item.brightness())
      }
      wire::MotionFilterKind::DropShadow => {
        battlement::FilterFunction::DropShadow(owned_shadow(item.shadow().unwrap()))
      }
      _ => unreachable!("Motion view validates filter kinds"),
    },
  )))
}

fn owned_shadows(value: wire::ShadowListMotionValue<'_>) -> battlement::MotionValue {
  battlement::MotionValue::ShadowList(value.values().iter().map(owned_shadow).collect())
}

fn owned_gradient(value: wire::GradientMotionValue<'_>) -> battlement::MotionValue {
  let stops = value
    .stops()
    .iter()
    .map(|stop| battlement::GradientStop {
      color: owned_color(stop.color()),
      position: stop.position(),
    })
    .collect();
  battlement::MotionValue::Gradient(match value.kind() {
    wire::MotionGradientKind::Linear => battlement::Gradient::Linear {
      angle: value.angle(),
      stops,
    },
    wire::MotionGradientKind::Radial => battlement::Gradient::Radial {
      center: owned_vector2(value.center().unwrap()),
      radius: owned_vector2(value.radius().unwrap()),
      stops,
    },
    _ => unreachable!("Motion view validates gradient kinds"),
  })
}

fn owned_inset(value: wire::ClipInsetMotionValue<'_>) -> battlement::MotionValue {
  battlement::MotionValue::ClipInset(
    value
      .values()
      .iter()
      .map(owned_length)
      .collect::<Vec<_>>()
      .try_into()
      .expect("Motion inset has four channels"),
  )
}

fn owned_polygon(value: wire::ClipPolygonMotionValue<'_>) -> battlement::MotionValue {
  battlement::MotionValue::ClipPolygon(
    value
      .values()
      .iter()
      .map(|point| [owned_length(point.x()), owned_length(point.y())])
      .collect(),
  )
}

fn owned_discrete(value: wire::DiscreteMotionValue<'_>) -> battlement::MotionValue {
  battlement::MotionValue::Discrete(match value.kind() {
    wire::MotionDiscreteKind::Null => battlement::MotionDiscreteValue::Null,
    wire::MotionDiscreteKind::String => {
      battlement::MotionDiscreteValue::String(value.string_value().unwrap().to_owned())
    }
    _ => unreachable!("Motion view validates discrete kinds"),
  })
}

fn gesture_vector(value: &wire::MotionVector2) -> battlement::MotionGestureVector {
  battlement::MotionGestureVector {
    x: value.x(),
    y: value.y(),
  }
}

fn object_id(value: &crate::common_generated::Uuid) -> battlement::ObjectId {
  battlement::ObjectId::from_uuid(uuid::Uuid::from_bytes(uuid_bytes(value)))
    .expect("Motion view validates nonzero UUIDs")
}

fn validate_batch(value: wire::MotionEventBatch<'_>) -> Result<(), ProtocolError> {
  if value.first_sequence() > value.last_sequence() {
    return Err(error("Motion sequence range is reversed"));
  }
  for count in [
    value.events().len(),
    value.samples().len(),
    value.value_samples().len(),
    value.playback_events().len(),
    value.gesture_events().len(),
  ] {
    if count > MAXIMUM_RECORDS {
      return Err(error("Motion batch has too many records"));
    }
  }

  let events = value.events();
  if !events.is_empty() {
    if events.get(0).sequence() != value.first_sequence()
      || events.get(events.len() - 1).sequence() != value.last_sequence()
    {
      return Err(error(
        "Motion sequence bounds do not match lifecycle events",
      ));
    }
    for index in 1..events.len() {
      if events.get(index - 1).sequence().checked_add(1) != Some(events.get(index).sequence()) {
        return Err(error("Motion lifecycle sequences are not contiguous"));
      }
    }
  }
  for event in events {
    if nil(event.descriptor_id()) || event.kind().0 > wire::MotionLifecycleKind::Cancelled.0 {
      return Err(error("invalid Motion lifecycle identity or kind"));
    }
    if event.kind() == wire::MotionLifecycleKind::Repeated {
      if event.repeat_first() > event.repeat_last() {
        return Err(error("Motion repeat range is reversed"));
      }
    } else if event.repeat_first() != 0 || event.repeat_last() != 0 {
      return Err(error("non-repeat Motion event carries repeat bounds"));
    }
  }

  for sample in value.samples() {
    if nil(sample.descriptor_id()) || sample.values().len() > MAXIMUM_RECORDS {
      return Err(error("invalid Motion presentation sample"));
    }
    let mut properties = HashSet::with_capacity(sample.values().len());
    for property in sample.values() {
      if !properties.insert(property.property().0) {
        return Err(error("Motion presentation properties must be unique"));
      }
      validate_property_value(property)?;
    }
  }

  for sample in value.value_samples() {
    if nil(sample.subscription_id()) || nil(sample.value_id()) {
      return Err(error("Motion value-sample UUIDs must be nonzero"));
    }
    validate_sample_value(sample, false)?;
    validate_sample_value(sample, true)?;
  }

  for event in value.playback_events() {
    if nil(event.playback_id()) || event.outcome().0 > wire::MotionPlaybackOutcome::Cancelled.0 {
      return Err(error("invalid Motion playback event"));
    }
  }

  for event in value.gesture_events() {
    if nil(event.descriptor_id())
      || event.kind().0 > wire::MotionGestureEventKind::InViewLeave.0
      || event.device().0 > wire::MotionPointerDevice::Gamepad.0
      || event.axis().0 > wire::MotionGestureAxis::Both.0
      || !vector2(event.point())
      || !vector2(event.delta())
      || !vector2(event.offset())
      || !vector2(event.velocity())
    {
      return Err(error("invalid Motion gesture event"));
    }
    if !event.has_axis() && event.axis() != wire::MotionGestureAxis::X {
      return Err(error(
        "Motion gesture without an axis has a noncanonical axis value",
      ));
    }
  }
  Ok(())
}

fn validate_property_value(value: wire::MotionPropertyValue<'_>) -> Result<(), ProtocolError> {
  if value.property().0 > wire::MotionProperty::Layout.0 {
    return Err(error("unknown Motion property"));
  }
  let valid = match value.value_type() {
    wire::MotionValue::ScalarMotionValue => value
      .value_as_scalar_motion_value()
      .is_some_and(|item| item.value().is_finite()),
    wire::MotionValue::LengthMotionValue => value
      .value_as_length_motion_value()
      .is_some_and(|item| length(item.value())),
    wire::MotionValue::ColorMotionValue => value
      .value_as_color_motion_value()
      .is_some_and(|item| color(item.value())),
    wire::MotionValue::Vector2MotionValue => value
      .value_as_vector_2_motion_value()
      .is_some_and(|item| vector2(item.value())),
    wire::MotionValue::Vector3MotionValue => value
      .value_as_vector_3_motion_value()
      .is_some_and(|item| vector3(item.value())),
    wire::MotionValue::AngleMotionValue => value
      .value_as_angle_motion_value()
      .is_some_and(|item| item.value().is_finite()),
    wire::MotionValue::TransformListMotionValue => value
      .value_as_transform_list_motion_value()
      .is_some_and(validate_transforms),
    wire::MotionValue::FilterListMotionValue => value
      .value_as_filter_list_motion_value()
      .is_some_and(validate_filters),
    wire::MotionValue::ShadowListMotionValue => value
      .value_as_shadow_list_motion_value()
      .is_some_and(|item| {
        item.values().len() <= MAXIMUM_RECORDS && item.values().iter().all(shadow)
      }),
    wire::MotionValue::GradientMotionValue => value
      .value_as_gradient_motion_value()
      .is_some_and(validate_gradient),
    wire::MotionValue::ClipInsetMotionValue => value
      .value_as_clip_inset_motion_value()
      .is_some_and(|item| item.values().len() == 4 && item.values().iter().all(length)),
    wire::MotionValue::ClipPolygonMotionValue => value
      .value_as_clip_polygon_motion_value()
      .is_some_and(|item| {
        !item.values().is_empty()
          && item.values().len() <= MAXIMUM_RECORDS
          && item
            .values()
            .iter()
            .all(|point| length(point.x()) && length(point.y()))
      }),
    wire::MotionValue::DiscreteMotionValue => value
      .value_as_discrete_motion_value()
      .is_some_and(validate_discrete),
    _ => false,
  };
  if valid {
    Ok(())
  } else {
    Err(error("invalid Motion property value"))
  }
}

fn validate_sample_value(
  sample: wire::MotionValueSample<'_>,
  velocity: bool,
) -> Result<(), ProtocolError> {
  let valid = match (
    velocity,
    if velocity {
      sample.velocity_type()
    } else {
      sample.value_type()
    },
  ) {
    (false, wire::MotionValue::ScalarMotionValue) => sample
      .value_as_scalar_motion_value()
      .is_some_and(|v| v.value().is_finite()),
    (true, wire::MotionValue::ScalarMotionValue) => sample
      .velocity_as_scalar_motion_value()
      .is_some_and(|v| v.value().is_finite()),
    (false, wire::MotionValue::LengthMotionValue) => sample
      .value_as_length_motion_value()
      .is_some_and(|v| length(v.value())),
    (true, wire::MotionValue::LengthMotionValue) => sample
      .velocity_as_length_motion_value()
      .is_some_and(|v| length(v.value())),
    (false, wire::MotionValue::ColorMotionValue) => sample
      .value_as_color_motion_value()
      .is_some_and(|v| color(v.value())),
    (true, wire::MotionValue::ColorMotionValue) => sample
      .velocity_as_color_motion_value()
      .is_some_and(|v| color(v.value())),
    (false, wire::MotionValue::Vector2MotionValue) => sample
      .value_as_vector_2_motion_value()
      .is_some_and(|v| vector2(v.value())),
    (true, wire::MotionValue::Vector2MotionValue) => sample
      .velocity_as_vector_2_motion_value()
      .is_some_and(|v| vector2(v.value())),
    (false, wire::MotionValue::Vector3MotionValue) => sample
      .value_as_vector_3_motion_value()
      .is_some_and(|v| vector3(v.value())),
    (true, wire::MotionValue::Vector3MotionValue) => sample
      .velocity_as_vector_3_motion_value()
      .is_some_and(|v| vector3(v.value())),
    (false, wire::MotionValue::AngleMotionValue) => sample
      .value_as_angle_motion_value()
      .is_some_and(|v| v.value().is_finite()),
    (true, wire::MotionValue::AngleMotionValue) => sample
      .velocity_as_angle_motion_value()
      .is_some_and(|v| v.value().is_finite()),
    (false, wire::MotionValue::TransformListMotionValue) => sample
      .value_as_transform_list_motion_value()
      .is_some_and(validate_transforms),
    (true, wire::MotionValue::TransformListMotionValue) => sample
      .velocity_as_transform_list_motion_value()
      .is_some_and(validate_transforms),
    (false, wire::MotionValue::FilterListMotionValue) => sample
      .value_as_filter_list_motion_value()
      .is_some_and(validate_filters),
    (true, wire::MotionValue::FilterListMotionValue) => sample
      .velocity_as_filter_list_motion_value()
      .is_some_and(validate_filters),
    (false, wire::MotionValue::ShadowListMotionValue) => sample
      .value_as_shadow_list_motion_value()
      .is_some_and(validate_shadows),
    (true, wire::MotionValue::ShadowListMotionValue) => sample
      .velocity_as_shadow_list_motion_value()
      .is_some_and(validate_shadows),
    (false, wire::MotionValue::GradientMotionValue) => sample
      .value_as_gradient_motion_value()
      .is_some_and(validate_gradient),
    (true, wire::MotionValue::GradientMotionValue) => sample
      .velocity_as_gradient_motion_value()
      .is_some_and(validate_gradient),
    (false, wire::MotionValue::ClipInsetMotionValue) => sample
      .value_as_clip_inset_motion_value()
      .is_some_and(validate_inset),
    (true, wire::MotionValue::ClipInsetMotionValue) => sample
      .velocity_as_clip_inset_motion_value()
      .is_some_and(validate_inset),
    (false, wire::MotionValue::ClipPolygonMotionValue) => sample
      .value_as_clip_polygon_motion_value()
      .is_some_and(validate_polygon),
    (true, wire::MotionValue::ClipPolygonMotionValue) => sample
      .velocity_as_clip_polygon_motion_value()
      .is_some_and(validate_polygon),
    (false, wire::MotionValue::DiscreteMotionValue) => sample
      .value_as_discrete_motion_value()
      .is_some_and(validate_discrete),
    (true, wire::MotionValue::DiscreteMotionValue) => sample
      .velocity_as_discrete_motion_value()
      .is_some_and(validate_discrete),
    _ => false,
  };
  if valid {
    Ok(())
  } else {
    Err(error("invalid Motion sampled value"))
  }
}

fn validate_transforms(value: wire::TransformListMotionValue<'_>) -> bool {
  value.values().len() <= MAXIMUM_RECORDS
    && value.values().iter().all(|item| match item.kind() {
      wire::MotionTransformKind::Translate => {
        item.scalars().is_none()
          && item
            .lengths()
            .is_some_and(|v| v.len() == 3 && v.iter().all(length))
      }
      wire::MotionTransformKind::Rotate
      | wire::MotionTransformKind::Skew
      | wire::MotionTransformKind::Scale => {
        item.lengths().is_none()
          && item.scalars().is_some_and(|v| {
            v.len()
              == if item.kind() == wire::MotionTransformKind::Skew {
                2
              } else {
                3
              }
              && v.iter().all(f32::is_finite)
          })
      }
      _ => false,
    })
}

fn validate_filters(value: wire::FilterListMotionValue<'_>) -> bool {
  value.values().len() <= MAXIMUM_RECORDS
    && value.values().iter().all(|item| match item.kind() {
      wire::MotionFilterKind::Brightness => {
        item.shadow().is_none() && item.brightness().is_finite() && item.brightness() >= 0.0
      }
      wire::MotionFilterKind::DropShadow => {
        item.brightness() == 0.0 && item.shadow().is_some_and(shadow)
      }
      _ => false,
    })
}

fn validate_shadows(value: wire::ShadowListMotionValue<'_>) -> bool {
  value.values().len() <= MAXIMUM_RECORDS && value.values().iter().all(shadow)
}

fn validate_gradient(value: wire::GradientMotionValue<'_>) -> bool {
  if value.stops().is_empty()
    || value.stops().len() > MAXIMUM_RECORDS
    || !value
      .stops()
      .iter()
      .all(|s| color(s.color()) && s.position().is_finite())
  {
    return false;
  }
  match value.kind() {
    wire::MotionGradientKind::Linear => {
      value.angle().is_finite() && value.center().is_none() && value.radius().is_none()
    }
    wire::MotionGradientKind::Radial => {
      value.angle() == 0.0
        && value.center().is_some_and(vector2)
        && value.radius().is_some_and(vector2)
    }
    _ => false,
  }
}

fn validate_inset(value: wire::ClipInsetMotionValue<'_>) -> bool {
  value.values().len() == 4 && value.values().iter().all(length)
}

fn validate_polygon(value: wire::ClipPolygonMotionValue<'_>) -> bool {
  !value.values().is_empty()
    && value.values().len() <= MAXIMUM_RECORDS
    && value
      .values()
      .iter()
      .all(|p| length(p.x()) && length(p.y()))
}

fn validate_discrete(value: wire::DiscreteMotionValue<'_>) -> bool {
  match value.kind() {
    wire::MotionDiscreteKind::Null => value.string_value().is_none(),
    wire::MotionDiscreteKind::String => value.string_value().is_some(),
    _ => false,
  }
}

fn length(value: &wire::MotionLength) -> bool {
  value.pixels().is_finite() && value.percentage().is_finite()
}

fn vector2(value: &wire::MotionVector2) -> bool {
  value.x().is_finite() && value.y().is_finite()
}

fn vector3(value: &wire::MotionVector3) -> bool {
  value.x().is_finite() && value.y().is_finite() && value.z().is_finite()
}

fn color(value: &wire::MotionColor) -> bool {
  value.red().is_finite()
    && value.green().is_finite()
    && value.blue().is_finite()
    && value.alpha().is_finite()
}

fn shadow(value: &wire::MotionShadow) -> bool {
  value.x().is_finite()
    && value.y().is_finite()
    && value.blur().is_finite()
    && value.spread().is_finite()
    && color(value.color())
}

fn nil(value: &crate::common_generated::Uuid) -> bool {
  uuid_bytes(value).iter().all(|byte| *byte == 0)
}

fn error(message: impl Into<String>) -> ProtocolError {
  ProtocolError::new(message)
}
