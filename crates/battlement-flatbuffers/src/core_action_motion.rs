use battlement::{
  MotionEventBatch, MotionEventKind, MotionGestureAxis, MotionGestureEventKind,
  MotionPlaybackOutcome, MotionPointerDevice,
};
use flatbuffers::{FlatBufferBuilder, WIPOffset};

use crate::{
  ProtocolError, client_message_generated::battlement::flat_buffers::generated as client,
  common_generated as common, motion_generated as wire, response_motion,
};

const MAXIMUM_RECORDS: usize = 262_144;

pub(crate) fn write<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  batch: &MotionEventBatch,
) -> Result<WIPOffset<client::MotionAction<'a>>, ProtocolError> {
  validate_sequence(batch)?;
  for count in [
    batch.events.len(),
    batch.samples.len(),
    batch.value_samples.len(),
    batch.playback_events.len(),
    batch.gesture_events.len(),
  ] {
    if count > MAXIMUM_RECORDS {
      return Err(error("a Motion batch has too many records"));
    }
  }

  let events = batch
    .events
    .iter()
    .map(|value| {
      let (kind, repeat_first, repeat_last) = match value.kind {
        MotionEventKind::Activated => (wire::MotionLifecycleKind::Activated, 0, 0),
        MotionEventKind::Started => (wire::MotionLifecycleKind::Started, 0, 0),
        MotionEventKind::Repeated { first, last } => {
          if first > last {
            return Err(error("Motion repeat boundaries must be ordered"));
          }
          (wire::MotionLifecycleKind::Repeated, first, last)
        }
        MotionEventKind::Completed => (wire::MotionLifecycleKind::Completed, 0, 0),
        MotionEventKind::Stopped => (wire::MotionLifecycleKind::Stopped, 0, 0),
        MotionEventKind::Cancelled => (wire::MotionLifecycleKind::Cancelled, 0, 0),
      };
      let descriptor_id = common::Uuid(*value.descriptor_id.as_uuid().as_bytes());
      Ok(wire::MotionLifecycleEvent::create(
        builder,
        &wire::MotionLifecycleEventArgs {
          sequence: value.sequence.0,
          descriptor_id: Some(&descriptor_id),
          slot: value.slot.0,
          generation: value.generation.0,
          elapsed_micros: value.elapsed_micros,
          kind,
          repeat_first,
          repeat_last,
        },
      ))
    })
    .collect::<Result<Vec<_>, ProtocolError>>()?;

  let samples = batch
    .samples
    .iter()
    .map(|sample| {
      let values = sample
        .values
        .iter()
        .map(|value| response_motion::write_property_value(builder, value))
        .collect::<Result<Vec<_>, _>>()?;
      let values = builder.create_vector(&values);
      let descriptor_id = common::Uuid(*sample.descriptor_id.as_uuid().as_bytes());
      Ok(wire::MotionPresentationSample::create(
        builder,
        &wire::MotionPresentationSampleArgs {
          descriptor_id: Some(&descriptor_id),
          slot: sample.slot.0,
          generation: sample.generation.0,
          elapsed_micros: sample.elapsed_micros,
          values: Some(values),
        },
      ))
    })
    .collect::<Result<Vec<_>, ProtocolError>>()?;

  let value_samples = batch
    .value_samples
    .iter()
    .map(|sample| {
      let value = response_motion::write_value(builder, &sample.value)?;
      let velocity = response_motion::write_value(builder, &sample.velocity)?;
      let subscription_id = common::Uuid(*sample.subscription_id.as_uuid().as_bytes());
      let value_id = common::Uuid(*sample.value_id.as_uuid().as_bytes());
      Ok(wire::MotionValueSample::create(
        builder,
        &wire::MotionValueSampleArgs {
          subscription_id: Some(&subscription_id),
          value_id: Some(&value_id),
          frame: sample.frame,
          value_type: value.kind,
          value: Some(value.value),
          velocity_type: velocity.kind,
          velocity: Some(velocity.value),
          discontinuity: sample.discontinuity,
        },
      ))
    })
    .collect::<Result<Vec<_>, ProtocolError>>()?;

  let playback_events = batch
    .playback_events
    .iter()
    .map(|event| {
      let playback_id = common::Uuid(*event.playback_id.as_uuid().as_bytes());
      wire::MotionPlaybackEvent::create(
        builder,
        &wire::MotionPlaybackEventArgs {
          playback_id: Some(&playback_id),
          generation: event.generation,
          outcome: match event.outcome {
            MotionPlaybackOutcome::Completed => wire::MotionPlaybackOutcome::Completed,
            MotionPlaybackOutcome::Stopped => wire::MotionPlaybackOutcome::Stopped,
            MotionPlaybackOutcome::Cancelled => wire::MotionPlaybackOutcome::Cancelled,
            MotionPlaybackOutcome::Failed => wire::MotionPlaybackOutcome::Failed,
          },
        },
      )
    })
    .collect::<Vec<_>>();

  let gesture_events = batch
    .gesture_events
    .iter()
    .map(|event| {
      for vector in [event.point, event.delta, event.offset, event.velocity] {
        if !vector.x.is_finite() || !vector.y.is_finite() {
          return Err(error("Motion gesture vectors must be finite"));
        }
      }
      let descriptor_id = common::Uuid(*event.descriptor_id.as_uuid().as_bytes());
      let point = wire::MotionVector2::new(event.point.x, event.point.y);
      let delta = wire::MotionVector2::new(event.delta.x, event.delta.y);
      let offset = wire::MotionVector2::new(event.offset.x, event.offset.y);
      let velocity = wire::MotionVector2::new(event.velocity.x, event.velocity.y);
      let (has_axis, axis) = match event.axis {
        None => (false, wire::MotionGestureAxis::X),
        Some(MotionGestureAxis::X) => (true, wire::MotionGestureAxis::X),
        Some(MotionGestureAxis::Y) => (true, wire::MotionGestureAxis::Y),
        Some(MotionGestureAxis::Both) => (true, wire::MotionGestureAxis::Both),
      };
      Ok(wire::MotionGestureEvent::create(
        builder,
        &wire::MotionGestureEventArgs {
          descriptor_id: Some(&descriptor_id),
          generation: event.generation.0,
          kind: gesture_kind(event.kind),
          pointer_id: event.pointer_id,
          device: pointer_device(event.device),
          point: Some(&point),
          delta: Some(&delta),
          offset: Some(&offset),
          velocity: Some(&velocity),
          has_axis,
          axis,
          momentum_generation: event.momentum_generation,
          constrained: event.constrained,
        },
      ))
    })
    .collect::<Result<Vec<_>, ProtocolError>>()?;

  let events = builder.create_vector(&events);
  let samples = builder.create_vector(&samples);
  let value_samples = builder.create_vector(&value_samples);
  let playback_events = builder.create_vector(&playback_events);
  let gesture_events = builder.create_vector(&gesture_events);
  let batch = wire::MotionEventBatch::create(
    builder,
    &wire::MotionEventBatchArgs {
      first_sequence: batch.first_sequence.0,
      last_sequence: batch.last_sequence.0,
      events: Some(events),
      samples: Some(samples),
      value_samples: Some(value_samples),
      playback_events: Some(playback_events),
      gesture_events: Some(gesture_events),
    },
  );
  Ok(client::MotionAction::create(
    builder,
    &client::MotionActionArgs { value: Some(batch) },
  ))
}

fn validate_sequence(batch: &MotionEventBatch) -> Result<(), ProtocolError> {
  if batch.first_sequence.0 > batch.last_sequence.0 {
    return Err(error("Motion sequence range is reversed"));
  }
  if batch.events.is_empty() {
    return Ok(());
  }
  if batch.events[0].sequence != batch.first_sequence
    || batch.events.last().expect("nonempty").sequence != batch.last_sequence
  {
    return Err(error("Motion sequence bounds do not match events"));
  }
  for pair in batch.events.windows(2) {
    if pair[0].sequence.0.checked_add(1) != Some(pair[1].sequence.0) {
      return Err(error("Motion lifecycle sequences must be contiguous"));
    }
  }
  Ok(())
}

fn pointer_device(value: MotionPointerDevice) -> wire::MotionPointerDevice {
  match value {
    MotionPointerDevice::Mouse => wire::MotionPointerDevice::Mouse,
    MotionPointerDevice::Touch => wire::MotionPointerDevice::Touch,
    MotionPointerDevice::Pen => wire::MotionPointerDevice::Pen,
    MotionPointerDevice::Keyboard => wire::MotionPointerDevice::Keyboard,
    MotionPointerDevice::Gamepad => wire::MotionPointerDevice::Gamepad,
  }
}

fn gesture_kind(value: MotionGestureEventKind) -> wire::MotionGestureEventKind {
  use MotionGestureEventKind as K;
  match value {
    K::HoverStart => wire::MotionGestureEventKind::HoverStart,
    K::HoverEnd => wire::MotionGestureEventKind::HoverEnd,
    K::TapStart => wire::MotionGestureEventKind::TapStart,
    K::Tap => wire::MotionGestureEventKind::Tap,
    K::TapCancel => wire::MotionGestureEventKind::TapCancel,
    K::FocusStart => wire::MotionGestureEventKind::FocusStart,
    K::FocusEnd => wire::MotionGestureEventKind::FocusEnd,
    K::FocusVisibleStart => wire::MotionGestureEventKind::FocusVisibleStart,
    K::FocusVisibleEnd => wire::MotionGestureEventKind::FocusVisibleEnd,
    K::PanSessionStart => wire::MotionGestureEventKind::PanSessionStart,
    K::PanStart => wire::MotionGestureEventKind::PanStart,
    K::Pan => wire::MotionGestureEventKind::Pan,
    K::PanEnd => wire::MotionGestureEventKind::PanEnd,
    K::PanCancel => wire::MotionGestureEventKind::PanCancel,
    K::DragStart => wire::MotionGestureEventKind::DragStart,
    K::DragDirectionLock => wire::MotionGestureEventKind::DragDirectionLock,
    K::Drag => wire::MotionGestureEventKind::Drag,
    K::DragEnd => wire::MotionGestureEventKind::DragEnd,
    K::DragCancel => wire::MotionGestureEventKind::DragCancel,
    K::DragMomentumComplete => wire::MotionGestureEventKind::DragMomentumComplete,
    K::DragConstraintsMeasured => wire::MotionGestureEventKind::DragConstraintsMeasured,
    K::Scroll => wire::MotionGestureEventKind::Scroll,
    K::InViewEnter => wire::MotionGestureEventKind::InViewEnter,
    K::InViewLeave => wire::MotionGestureEventKind::InViewLeave,
  }
}

fn error(message: impl Into<String>) -> ProtocolError {
  ProtocolError::new(message)
}
