use battlement::{
  FilterFunction, Gradient, InertiaTarget, MotionDiscreteValue, MotionEasing,
  MotionPlaybackCommand, MotionProperty, MotionPropertyTarget, MotionRepeat, MotionRepeatType,
  MotionTargetDescriptor, MotionValue, SpringConfiguration, StepPosition, TransformOperation,
  TransitionDefinition, TransitionGenerator,
};
use flatbuffers::{Allocator, FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use crate::{ProtocolError, common_generated as common, motion_generated as wire};

pub(crate) struct ValueOffset {
  pub(crate) kind: wire::MotionValue,
  pub(crate) value: WIPOffset<UnionWIPOffset>,
}

pub(crate) fn write_value<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &MotionValue,
) -> Result<ValueOffset, ProtocolError> {
  let (kind, value) = match value {
    MotionValue::Scalar(value) => {
      let value =
        wire::ScalarMotionValue::create(builder, &wire::ScalarMotionValueArgs { value: *value });
      (wire::MotionValue::ScalarMotionValue, value.as_union_value())
    }
    MotionValue::Length(value) => {
      let value = length(*value);
      let value = wire::LengthMotionValue::create(
        builder,
        &wire::LengthMotionValueArgs {
          value: Some(&value),
        },
      );
      (wire::MotionValue::LengthMotionValue, value.as_union_value())
    }
    MotionValue::Color(value) => {
      let value = motion_color(*value);
      let value = wire::ColorMotionValue::create(
        builder,
        &wire::ColorMotionValueArgs {
          value: Some(&value),
        },
      );
      (wire::MotionValue::ColorMotionValue, value.as_union_value())
    }
    MotionValue::Vector2(value) => {
      let value = wire::MotionVector2::new(value[0], value[1]);
      let value = wire::Vector2MotionValue::create(
        builder,
        &wire::Vector2MotionValueArgs {
          value: Some(&value),
        },
      );
      (
        wire::MotionValue::Vector2MotionValue,
        value.as_union_value(),
      )
    }
    MotionValue::Vector3(value) => {
      let value = wire::MotionVector3::new(value[0], value[1], value[2]);
      let value = wire::Vector3MotionValue::create(
        builder,
        &wire::Vector3MotionValueArgs {
          value: Some(&value),
        },
      );
      (
        wire::MotionValue::Vector3MotionValue,
        value.as_union_value(),
      )
    }
    MotionValue::Angle(value) => {
      let value =
        wire::AngleMotionValue::create(builder, &wire::AngleMotionValueArgs { value: *value });
      (wire::MotionValue::AngleMotionValue, value.as_union_value())
    }
    MotionValue::TransformList(values) => {
      let values = values
        .iter()
        .map(|value| write_transform(builder, value))
        .collect::<Vec<_>>();
      let values = builder.create_vector(&values);
      let value = wire::TransformListMotionValue::create(
        builder,
        &wire::TransformListMotionValueArgs {
          values: Some(values),
        },
      );
      (
        wire::MotionValue::TransformListMotionValue,
        value.as_union_value(),
      )
    }
    MotionValue::FilterList(values) => {
      let values = values
        .as_slice()
        .iter()
        .map(|value| write_filter(builder, *value))
        .collect::<Vec<_>>();
      let values = builder.create_vector(&values);
      let value = wire::FilterListMotionValue::create(
        builder,
        &wire::FilterListMotionValueArgs {
          values: Some(values),
        },
      );
      (
        wire::MotionValue::FilterListMotionValue,
        value.as_union_value(),
      )
    }
    MotionValue::ShadowList(values) => {
      let values = values.iter().copied().map(shadow).collect::<Vec<_>>();
      let values = builder.create_vector(&values);
      let value = wire::ShadowListMotionValue::create(
        builder,
        &wire::ShadowListMotionValueArgs {
          values: Some(values),
        },
      );
      (
        wire::MotionValue::ShadowListMotionValue,
        value.as_union_value(),
      )
    }
    MotionValue::Gradient(value) => {
      let (kind, angle, center, radius, stops) = match value {
        Gradient::Linear { angle, stops } => (
          wire::MotionGradientKind::Linear,
          *angle,
          wire::MotionVector2::new(0.0, 0.0),
          wire::MotionVector2::new(0.0, 0.0),
          stops,
        ),
        Gradient::Radial {
          center,
          radius,
          stops,
        } => (
          wire::MotionGradientKind::Radial,
          0.0,
          wire::MotionVector2::new(center[0], center[1]),
          wire::MotionVector2::new(radius[0], radius[1]),
          stops,
        ),
      };
      let stops = stops
        .iter()
        .map(|stop| wire::MotionGradientStop::new(&motion_color(stop.color), stop.position))
        .collect::<Vec<_>>();
      let stops = builder.create_vector(&stops);
      let value = wire::GradientMotionValue::create(
        builder,
        &wire::GradientMotionValueArgs {
          kind,
          angle,
          center: Some(&center),
          radius: Some(&radius),
          stops: Some(stops),
        },
      );
      (
        wire::MotionValue::GradientMotionValue,
        value.as_union_value(),
      )
    }
    MotionValue::ClipInset(values) => {
      let values = values.iter().copied().map(length).collect::<Vec<_>>();
      let values = builder.create_vector(&values);
      let value = wire::ClipInsetMotionValue::create(
        builder,
        &wire::ClipInsetMotionValueArgs {
          values: Some(values),
        },
      );
      (
        wire::MotionValue::ClipInsetMotionValue,
        value.as_union_value(),
      )
    }
    MotionValue::ClipPolygon(values) => {
      let values = values
        .iter()
        .map(|value| wire::MotionLengthPoint::new(&length(value[0]), &length(value[1])))
        .collect::<Vec<_>>();
      let values = builder.create_vector(&values);
      let value = wire::ClipPolygonMotionValue::create(
        builder,
        &wire::ClipPolygonMotionValueArgs {
          values: Some(values),
        },
      );
      (
        wire::MotionValue::ClipPolygonMotionValue,
        value.as_union_value(),
      )
    }
    MotionValue::Discrete(value) => {
      let (kind, string_value) = match value {
        MotionDiscreteValue::Null => (wire::MotionDiscreteKind::Null, None),
        MotionDiscreteValue::String(value) => (
          wire::MotionDiscreteKind::String,
          Some(builder.create_string(value)),
        ),
      };
      let value = wire::DiscreteMotionValue::create(
        builder,
        &wire::DiscreteMotionValueArgs { kind, string_value },
      );
      (
        wire::MotionValue::DiscreteMotionValue,
        value.as_union_value(),
      )
    }
  };
  Ok(ValueOffset { kind, value })
}

pub(crate) fn write_target<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &MotionTargetDescriptor,
) -> Result<WIPOffset<wire::MotionTargetDescriptor<'a>>, ProtocolError> {
  value.validate().map_err(ProtocolError::new)?;
  let tracks = value
    .tracks
    .iter()
    .map(|track| {
      let material_parameter = match &track.target {
        MotionPropertyTarget::MaterialScalar { parameter, .. } => {
          Some(builder.create_string(parameter))
        }
        _ => None,
      };
      let playback_id = match track.target {
        MotionPropertyTarget::AudioVolume { playback_id } => Some(uuid(playback_id.as_uuid())),
        _ => None,
      };
      let target = wire::MotionPropertyTarget::create(
        builder,
        &wire::MotionPropertyTargetArgs {
          kind: match track.target {
            MotionPropertyTarget::Host => wire::MotionPropertyTargetKind::Host,
            MotionPropertyTarget::MaterialScalar { .. } => {
              wire::MotionPropertyTargetKind::MaterialScalar
            }
            MotionPropertyTarget::AudioVolume { .. } => wire::MotionPropertyTargetKind::AudioVolume,
          },
          material_slot: match track.target {
            MotionPropertyTarget::MaterialScalar { slot, .. } => slot,
            _ => 0,
          },
          material_parameter,
          playback_id: playback_id.as_ref(),
        },
      );
      let values = track
        .values
        .iter()
        .map(|value| {
          let value = write_value(builder, value)?;
          Ok(wire::MotionValueEntry::create(
            builder,
            &wire::MotionValueEntryArgs {
              value_type: value.kind,
              value: Some(value.value),
            },
          ))
        })
        .collect::<Result<Vec<_>, ProtocolError>>()?;
      let values = builder.create_vector(&values);
      let times = track
        .times
        .as_ref()
        .map(|values| builder.create_vector(values));
      let transition = write_transition(builder, &track.transition)?;
      Ok(wire::MotionPropertyTrack::create(
        builder,
        &wire::MotionPropertyTrackArgs {
          property: property(track.property),
          target: Some(target),
          values: Some(values),
          times,
          transition: Some(transition),
        },
      ))
    })
    .collect::<Result<Vec<_>, ProtocolError>>()?;
  let transition_end = value
    .transition_end
    .iter()
    .map(|value| write_property_value(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let tracks = builder.create_vector(&tracks);
  let transition_end = builder.create_vector(&transition_end);
  Ok(wire::MotionTargetDescriptor::create(
    builder,
    &wire::MotionTargetDescriptorArgs {
      tracks: Some(tracks),
      transition_end: Some(transition_end),
    },
  ))
}

pub(crate) fn write_transition<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &TransitionDefinition,
) -> Result<WIPOffset<wire::TransitionDefinition<'a>>, ProtocolError> {
  let mut args = wire::TransitionDefinitionArgs {
    delay_micros: value.delay_micros,
    repeat_delay_micros: value.repeat_delay_micros,
    repeat_type: match value.repeat_type {
      MotionRepeatType::Loop => wire::MotionRepeatType::Loop,
      MotionRepeatType::Reverse => wire::MotionRepeatType::Reverse,
      MotionRepeatType::Mirror => wire::MotionRepeatType::Mirror,
    },
    ..Default::default()
  };
  (args.repeat, args.repeat_count) = match value.repeat {
    MotionRepeat::None => (wire::MotionRepeatKind::None, 0),
    MotionRepeat::Count(value) => (wire::MotionRepeatKind::Count, value),
    MotionRepeat::Forever => (wire::MotionRepeatKind::Forever, 0),
  };
  let mut easing_offsets = Vec::new();
  let mut times = None;
  match &value.generator {
    TransitionGenerator::Immediate => args.generator = wire::TransitionGeneratorKind::Immediate,
    TransitionGenerator::Tween {
      duration_micros,
      easings,
      times: values,
    } => {
      args.generator = wire::TransitionGeneratorKind::Tween;
      args.duration_micros = *duration_micros;
      easing_offsets = easings
        .iter()
        .map(|value| write_easing(builder, *value))
        .collect();
      times = values.as_ref().map(|values| builder.create_vector(values));
    }
    TransitionGenerator::Spring(value) => match value {
      SpringConfiguration::Physical {
        stiffness,
        damping,
        mass,
        initial_velocity,
        rest_speed,
        rest_delta,
      } => {
        args.generator = wire::TransitionGeneratorKind::SpringPhysical;
        args.stiffness = *stiffness;
        args.damping = *damping;
        args.mass = *mass;
        args.initial_velocity = *initial_velocity;
        args.rest_speed = *rest_speed;
        args.rest_delta = *rest_delta;
      }
      SpringConfiguration::Duration {
        duration_micros,
        bounce,
        mass,
      } => {
        args.generator = wire::TransitionGeneratorKind::SpringDuration;
        args.duration_micros = *duration_micros;
        args.bounce = *bounce;
        args.mass = *mass;
      }
      SpringConfiguration::VisualDuration {
        duration_micros,
        bounce,
        mass,
      } => {
        args.generator = wire::TransitionGeneratorKind::SpringVisualDuration;
        args.duration_micros = *duration_micros;
        args.bounce = *bounce;
        args.mass = *mass;
      }
    },
    TransitionGenerator::Inertia {
      initial_velocity,
      power,
      time_constant_micros,
      minimum,
      maximum,
      rest_delta,
      bounce_stiffness,
      bounce_damping,
      target,
    } => {
      args.generator = wire::TransitionGeneratorKind::Inertia;
      args.initial_velocity = Some(*initial_velocity);
      args.power = *power;
      args.time_constant_micros = *time_constant_micros;
      args.minimum = *minimum;
      args.maximum = *maximum;
      args.rest_delta = Some(*rest_delta);
      args.bounce_stiffness = *bounce_stiffness;
      args.bounce_damping = *bounce_damping;
      (
        args.inertia_target,
        args.inertia_target_value,
        args.inertia_target_maximum,
      ) = match target {
        InertiaTarget::Identity => (wire::InertiaTargetKind::Identity, 0.0, 0.0),
        InertiaTarget::NearestMultiple(value) => {
          (wire::InertiaTargetKind::NearestMultiple, *value, 0.0)
        }
        InertiaTarget::FloorMultiple(value) => {
          (wire::InertiaTargetKind::FloorMultiple, *value, 0.0)
        }
        InertiaTarget::CeilingMultiple(value) => {
          (wire::InertiaTargetKind::CeilingMultiple, *value, 0.0)
        }
        InertiaTarget::Clamp { min, max } => (wire::InertiaTargetKind::Clamp, *min, *max),
      };
    }
  }
  let easings = builder.create_vector(&easing_offsets);
  args.easings = Some(easings);
  args.times = times;
  Ok(wire::TransitionDefinition::create(builder, &args))
}

pub(crate) fn write_playback_command<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: MotionPlaybackCommand,
) -> WIPOffset<wire::MotionPlaybackCommand<'a>> {
  let (kind, elapsed_micros, speed, direction) = match value {
    MotionPlaybackCommand::Play => (
      wire::MotionPlaybackCommandKind::Play,
      0,
      0.0,
      wire::MotionPlaybackDirection::Forward,
    ),
    MotionPlaybackCommand::Pause => (
      wire::MotionPlaybackCommandKind::Pause,
      0,
      0.0,
      wire::MotionPlaybackDirection::Forward,
    ),
    MotionPlaybackCommand::Replay => (
      wire::MotionPlaybackCommandKind::Replay,
      0,
      0.0,
      wire::MotionPlaybackDirection::Forward,
    ),
    MotionPlaybackCommand::Stop => (
      wire::MotionPlaybackCommandKind::Stop,
      0,
      0.0,
      wire::MotionPlaybackDirection::Forward,
    ),
    MotionPlaybackCommand::Cancel => (
      wire::MotionPlaybackCommandKind::Cancel,
      0,
      0.0,
      wire::MotionPlaybackDirection::Forward,
    ),
    MotionPlaybackCommand::Complete => (
      wire::MotionPlaybackCommandKind::Complete,
      0,
      0.0,
      wire::MotionPlaybackDirection::Forward,
    ),
    MotionPlaybackCommand::Seek { elapsed_micros } => (
      wire::MotionPlaybackCommandKind::Seek,
      elapsed_micros,
      0.0,
      wire::MotionPlaybackDirection::Forward,
    ),
    MotionPlaybackCommand::SetSpeed { value } => (
      wire::MotionPlaybackCommandKind::SetSpeed,
      0,
      value,
      wire::MotionPlaybackDirection::Forward,
    ),
    MotionPlaybackCommand::SetDirection { value } => (
      wire::MotionPlaybackCommandKind::SetDirection,
      0,
      0.0,
      playback_direction(value),
    ),
  };
  wire::MotionPlaybackCommand::create(
    builder,
    &wire::MotionPlaybackCommandArgs {
      kind,
      elapsed_micros,
      speed,
      direction,
    },
  )
}

pub(crate) fn write_playback_operation<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionPlaybackOperation,
) -> WIPOffset<wire::MotionPlaybackOperation<'a>> {
  let descriptor_id = uuid(value.descriptor_id.as_uuid());
  let command = write_playback_command(builder, value.command);
  wire::MotionPlaybackOperation::create(
    builder,
    &wire::MotionPlaybackOperationArgs {
      descriptor_id: Some(&descriptor_id),
      slot: value.slot.0,
      generation: value.generation.0,
      command: Some(command),
    },
  )
}

pub(crate) fn write_controlled_clock_operation<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionControlledClockOperation,
) -> WIPOffset<wire::MotionControlledClockOperation<'a>> {
  let clock_id = uuid(value.clock_id.as_uuid());
  let (command, micros) = match value.command {
    battlement::MotionControlledClockCommand::Set { elapsed_micros } => {
      (wire::MotionControlledClockCommandKind::Set, elapsed_micros)
    }
    battlement::MotionControlledClockCommand::Advance { delta_micros } => (
      wire::MotionControlledClockCommandKind::Advance,
      delta_micros,
    ),
  };
  wire::MotionControlledClockOperation::create(
    builder,
    &wire::MotionControlledClockOperationArgs {
      clock_id: Some(&clock_id),
      command,
      micros,
    },
  )
}

pub(crate) fn write_value_operation<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  operation: &battlement::MotionValueOperation,
) -> Result<WIPOffset<wire::MotionValueOperation<'a>>, ProtocolError> {
  let mut value = None;
  let mut playback_id = None;
  let mut generation = 0;
  let mut transition = None;
  let command = match &operation.command {
    battlement::MotionValueCommand::Set(body) => {
      value = Some(write_value(builder, body)?);
      wire::MotionValueCommandKind::Set
    }
    battlement::MotionValueCommand::Jump(body) => {
      value = Some(write_value(builder, body)?);
      wire::MotionValueCommandKind::Jump
    }
    battlement::MotionValueCommand::Stop => wire::MotionValueCommandKind::Stop,
    battlement::MotionValueCommand::Animate {
      playback_id: id,
      generation: value_generation,
      target,
      transition: value_transition,
    } => {
      value = Some(write_value(builder, target)?);
      playback_id = Some(uuid(id.as_uuid()));
      generation = *value_generation;
      transition = Some(write_transition(builder, value_transition)?);
      wire::MotionValueCommandKind::Animate
    }
  };
  let value_id = uuid(operation.value_id.as_uuid());
  Ok(wire::MotionValueOperation::create(
    builder,
    &wire::MotionValueOperationArgs {
      value_id: Some(&value_id),
      command,
      value_type: value
        .as_ref()
        .map_or(wire::MotionValue::NONE, |value| value.kind),
      value: value.map(|value| value.value),
      playback_id: playback_id.as_ref(),
      generation,
      transition,
    },
  ))
}

pub(crate) fn write_value_playback_operation<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionValuePlaybackOperation,
) -> WIPOffset<wire::MotionValuePlaybackOperation<'a>> {
  let playback_id = uuid(value.playback_id.as_uuid());
  let command = write_playback_command(builder, value.command);
  wire::MotionValuePlaybackOperation::create(
    builder,
    &wire::MotionValuePlaybackOperationArgs {
      playback_id: Some(&playback_id),
      generation: value.generation,
      command: Some(command),
    },
  )
}

pub(crate) fn write_control_operation<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  operation: &battlement::MotionControlOperation,
) -> Result<WIPOffset<wire::MotionControlOperation<'a>>, ProtocolError> {
  let mut playback_id = None;
  let mut generation = 0;
  let mut target = None;
  let command = match &operation.command {
    battlement::MotionControlCommand::Start {
      playback_id: id,
      generation: value_generation,
      target: value,
    } => {
      playback_id = Some(uuid(id.as_uuid()));
      generation = *value_generation;
      target = Some(write_control_target(builder, value)?);
      wire::MotionControlCommandKind::Start
    }
    battlement::MotionControlCommand::Set(value) => {
      target = Some(write_control_target(builder, value)?);
      wire::MotionControlCommandKind::Set
    }
    battlement::MotionControlCommand::Stop => wire::MotionControlCommandKind::Stop,
    battlement::MotionControlCommand::Clear => wire::MotionControlCommandKind::Clear,
  };
  let control_id = uuid(operation.control_id.as_uuid());
  Ok(wire::MotionControlOperation::create(
    builder,
    &wire::MotionControlOperationArgs {
      control_id: Some(&control_id),
      command,
      playback_id: playback_id.as_ref(),
      generation,
      target,
    },
  ))
}

pub(crate) fn write_scope_operation<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  operation: &battlement::MotionScopeOperation,
) -> Result<WIPOffset<wire::MotionScopeOperation<'a>>, ProtocolError> {
  let mut playback_id = None;
  let mut generation = 0;
  let mut entries = None;
  let mut selector = None;
  let mut target = None;
  let command = match &operation.command {
    battlement::MotionScopeCommand::Start {
      playback_id: id,
      generation: value_generation,
      entries: values,
    } => {
      playback_id = Some(uuid(id.as_uuid()));
      generation = *value_generation;
      let values = values
        .iter()
        .map(|value| write_sequence_entry(builder, value))
        .collect::<Result<Vec<_>, ProtocolError>>()?;
      entries = Some(builder.create_vector(&values));
      wire::MotionScopeCommandKind::Start
    }
    battlement::MotionScopeCommand::Set {
      selector: value_selector,
      target: value_target,
    } => {
      selector = Some(write_selector(builder, value_selector));
      target = Some(write_target(builder, value_target)?);
      wire::MotionScopeCommandKind::Set
    }
    battlement::MotionScopeCommand::Stop(value) => {
      selector = Some(write_selector(builder, value));
      wire::MotionScopeCommandKind::Stop
    }
  };
  let scope_id = uuid(operation.scope_id.as_uuid());
  Ok(wire::MotionScopeOperation::create(
    builder,
    &wire::MotionScopeOperationArgs {
      scope_id: Some(&scope_id),
      command,
      playback_id: playback_id.as_ref(),
      generation,
      entries,
      selector,
      target,
    },
  ))
}

fn write_sequence_entry<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionSequenceEntry,
) -> Result<WIPOffset<wire::MotionSequenceEntry<'a>>, ProtocolError> {
  let mut selector = None;
  let mut target = None;
  let mut position = None;
  let mut position_transition = None;
  let mut conflict = wire::MotionSequenceConflict::Reject;
  let mut label = None;
  let mut effect_address = None;
  let mut effect_volume = 1.0;
  let mut effect_pitch = 1.0;
  let mut effect_loop = false;
  let mut effect_fade_in_millis = 0;
  let mut effect_position = None;
  let mut effect_lifetime_millis = 0;
  let (kind, schedule) = match value {
    battlement::MotionSequenceEntry::Animate {
      selector: value_selector,
      target: value_target,
      position: value_position,
      position_transition: value_transition,
      schedule,
      conflict: value_conflict,
    } => {
      selector = Some(write_selector(builder, value_selector));
      target = Some(write_target(builder, value_target)?);
      position = value_position
        .as_ref()
        .map(|value| write_position_reference(builder, value));
      position_transition = Some(write_transition(builder, value_transition)?);
      conflict = match value_conflict {
        battlement::MotionSequenceConflict::Reject => wire::MotionSequenceConflict::Reject,
        battlement::MotionSequenceConflict::Replace => wire::MotionSequenceConflict::Replace,
      };
      (wire::MotionSequenceEntryKind::Animate, schedule)
    }
    battlement::MotionSequenceEntry::Label { name, schedule } => {
      label = Some(builder.create_string(name));
      (wire::MotionSequenceEntryKind::Label, schedule)
    }
    battlement::MotionSequenceEntry::Sound { sound, schedule } => {
      effect_address = Some(builder.create_string(&sound.address));
      effect_volume = sound.volume;
      effect_pitch = sound.pitch;
      effect_loop = sound.looping;
      effect_fade_in_millis = sound.fade_in_ms;
      (wire::MotionSequenceEntryKind::Sound, schedule)
    }
    battlement::MotionSequenceEntry::Particle { particle, schedule } => {
      effect_address = Some(builder.create_string(&particle.address));
      effect_position = Some(write_position_reference(builder, &particle.position));
      effect_lifetime_millis = particle.lifetime_ms;
      (wire::MotionSequenceEntryKind::Particle, schedule)
    }
  };
  let schedule = write_sequence_schedule(builder, schedule);
  Ok(wire::MotionSequenceEntry::create(
    builder,
    &wire::MotionSequenceEntryArgs {
      kind,
      selector,
      target,
      position,
      position_transition,
      schedule: Some(schedule),
      conflict,
      label,
      effect_address,
      effect_volume,
      effect_pitch,
      effect_loop,
      effect_fade_in_millis,
      effect_position,
      effect_lifetime_millis,
    },
  ))
}

fn write_sequence_schedule<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionSequenceSchedule,
) -> WIPOffset<wire::MotionSequenceSchedule<'a>> {
  let mut entry = 0;
  let mut offset_micros = 0;
  let mut absolute_micros = 0;
  let mut label = None;
  let kind = match value {
    battlement::MotionSequenceSchedule::Absolute(value) => {
      absolute_micros = *value;
      wire::MotionSequenceScheduleKind::Absolute
    }
    battlement::MotionSequenceSchedule::RelativeStart {
      entry: value_entry,
      offset_micros: value_offset,
    } => {
      entry = *value_entry;
      offset_micros = *value_offset;
      wire::MotionSequenceScheduleKind::RelativeStart
    }
    battlement::MotionSequenceSchedule::AfterCompletion {
      entry: value_entry,
      offset_micros: value_offset,
    } => {
      entry = *value_entry;
      offset_micros = *value_offset;
      wire::MotionSequenceScheduleKind::AfterCompletion
    }
    battlement::MotionSequenceSchedule::Label {
      name,
      offset_micros: value_offset,
    } => {
      label = Some(builder.create_string(name));
      offset_micros = *value_offset;
      wire::MotionSequenceScheduleKind::Label
    }
  };
  wire::MotionSequenceSchedule::create(
    builder,
    &wire::MotionSequenceScheduleArgs {
      kind,
      entry,
      offset_micros,
      absolute_micros,
      label,
    },
  )
}

fn write_position_reference<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionPositionReference,
) -> WIPOffset<wire::MotionPositionReference<'a>> {
  let object_id = uuid(value.object_id.as_uuid());
  let anchor = value
    .anchor
    .as_ref()
    .map(|value| builder.create_string(value));
  wire::MotionPositionReference::create(
    builder,
    &wire::MotionPositionReferenceArgs {
      object_id: Some(&object_id),
      anchor,
      resolution: match value.resolution {
        battlement::MotionReferenceResolution::CaptureAtStart => {
          wire::MotionReferenceResolution::CaptureAtStart
        }
        battlement::MotionReferenceResolution::Follow => wire::MotionReferenceResolution::Follow,
      },
    },
  )
}

pub(crate) fn write_drag_control_operation<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionDragControlOperation,
) -> WIPOffset<wire::MotionDragControlOperation<'a>> {
  let control_id = uuid(value.control_id.as_uuid());
  let point = wire::MotionVector2::new(value.point.x, value.point.y);
  wire::MotionDragControlOperation::create(
    builder,
    &wire::MotionDragControlOperationArgs {
      control_id: Some(&control_id),
      pointer_id: value.pointer_id,
      device: pointer_device(value.device),
      point: Some(&point),
      snap_to_cursor: value.snap_to_cursor,
    },
  )
}

fn write_control_target<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionControlTarget,
) -> Result<WIPOffset<wire::MotionControlTarget<'a>>, ProtocolError> {
  let (kind, target, variant) = match value {
    battlement::MotionControlTarget::Target(value) => (
      wire::MotionControlTargetKind::Target,
      Some(write_target(builder, value)?),
      None,
    ),
    battlement::MotionControlTarget::Variant(value) => (
      wire::MotionControlTargetKind::Variant,
      None,
      Some(builder.create_string(value)),
    ),
  };
  Ok(wire::MotionControlTarget::create(
    builder,
    &wire::MotionControlTargetArgs {
      kind,
      target,
      variant,
    },
  ))
}

fn write_selector<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionSelector,
) -> WIPOffset<wire::MotionSelector<'a>> {
  let (kind, object_id, name) = match value {
    battlement::MotionSelector::Element(value) => (
      wire::MotionSelectorKind::Element,
      Some(uuid(value.as_uuid())),
      None,
    ),
    battlement::MotionSelector::Name(value) => (
      wire::MotionSelectorKind::Name,
      None,
      Some(builder.create_string(value)),
    ),
    battlement::MotionSelector::ScopeRoot => (wire::MotionSelectorKind::ScopeRoot, None, None),
    battlement::MotionSelector::Children => (wire::MotionSelectorKind::Children, None, None),
    battlement::MotionSelector::Descendants => (wire::MotionSelectorKind::Descendants, None, None),
  };
  wire::MotionSelector::create(
    builder,
    &wire::MotionSelectorArgs {
      kind,
      object_id: object_id.as_ref(),
      name,
    },
  )
}

pub(crate) fn write_property_value<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionPropertyValue,
) -> Result<WIPOffset<wire::MotionPropertyValue<'a>>, ProtocolError> {
  value.validate().map_err(ProtocolError::new)?;
  let offset = write_value(builder, &value.value)?;
  Ok(wire::MotionPropertyValue::create(
    builder,
    &wire::MotionPropertyValueArgs {
      property: property(value.property),
      value_type: offset.kind,
      value: Some(offset.value),
    },
  ))
}

fn write_easing<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: MotionEasing,
) -> WIPOffset<wire::MotionEasingDefinition<'a>> {
  let (kind, bezier, step_count, step_position) = match value {
    MotionEasing::Linear => (
      wire::MotionEasingKind::Linear,
      None,
      0,
      wire::StepPosition::Start,
    ),
    MotionEasing::EaseIn => (
      wire::MotionEasingKind::EaseIn,
      None,
      0,
      wire::StepPosition::Start,
    ),
    MotionEasing::EaseOut => (
      wire::MotionEasingKind::EaseOut,
      None,
      0,
      wire::StepPosition::Start,
    ),
    MotionEasing::EaseInOut => (
      wire::MotionEasingKind::EaseInOut,
      None,
      0,
      wire::StepPosition::Start,
    ),
    MotionEasing::CubicBezier(values) => (
      wire::MotionEasingKind::CubicBezier,
      Some(builder.create_vector(&values)),
      0,
      wire::StepPosition::Start,
    ),
    MotionEasing::Steps { count, position } => (
      wire::MotionEasingKind::Steps,
      None,
      count,
      match position {
        StepPosition::Start => wire::StepPosition::Start,
        StepPosition::End => wire::StepPosition::End,
      },
    ),
  };
  wire::MotionEasingDefinition::create(
    builder,
    &wire::MotionEasingDefinitionArgs {
      kind,
      cubic_bezier: bezier,
      step_count,
      step_position,
    },
  )
}

fn write_transform<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &TransformOperation,
) -> WIPOffset<wire::MotionTransform<'a>> {
  let (kind, lengths, scalars) = match value {
    TransformOperation::Translate(values) => (
      wire::MotionTransformKind::Translate,
      Some(builder.create_vector(&values.iter().copied().map(length).collect::<Vec<_>>())),
      None,
    ),
    TransformOperation::Rotate(values) => (
      wire::MotionTransformKind::Rotate,
      None,
      Some(builder.create_vector(values)),
    ),
    TransformOperation::Skew(values) => (
      wire::MotionTransformKind::Skew,
      None,
      Some(builder.create_vector(values)),
    ),
    TransformOperation::Scale(values) => (
      wire::MotionTransformKind::Scale,
      None,
      Some(builder.create_vector(values)),
    ),
  };
  wire::MotionTransform::create(
    builder,
    &wire::MotionTransformArgs {
      kind,
      lengths,
      scalars,
    },
  )
}

fn write_filter<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: FilterFunction,
) -> WIPOffset<wire::MotionFilter<'a>> {
  let (kind, brightness, shadow_value) = match value {
    FilterFunction::Brightness(value) => (wire::MotionFilterKind::Brightness, value, None),
    FilterFunction::DropShadow(value) => {
      (wire::MotionFilterKind::DropShadow, 0.0, Some(shadow(value)))
    }
  };
  wire::MotionFilter::create(
    builder,
    &wire::MotionFilterArgs {
      kind,
      brightness,
      shadow: shadow_value.as_ref(),
    },
  )
}

pub(crate) fn property(value: MotionProperty) -> wire::MotionProperty {
  let index = MotionProperty::ALL
    .iter()
    .position(|candidate| *candidate == value)
    .expect("motion property catalog is exhaustive");
  wire::MotionProperty(u16::try_from(index).expect("motion property catalog fits u16"))
}

fn length(value: battlement::Length) -> wire::MotionLength {
  let [pixels, percentage] = value.components();
  wire::MotionLength::new(pixels, percentage)
}

fn motion_color(value: battlement::Color) -> wire::MotionColor {
  wire::MotionColor::new(value.r, value.g, value.b, value.a)
}

fn shadow(value: battlement::Shadow) -> wire::MotionShadow {
  wire::MotionShadow::new(
    value.x,
    value.y,
    value.blur,
    value.spread,
    &motion_color(value.color),
    value.inset,
  )
}

fn playback_direction(value: battlement::MotionPlaybackDirection) -> wire::MotionPlaybackDirection {
  match value {
    battlement::MotionPlaybackDirection::Forward => wire::MotionPlaybackDirection::Forward,
    battlement::MotionPlaybackDirection::Reverse => wire::MotionPlaybackDirection::Reverse,
    battlement::MotionPlaybackDirection::Alternate => wire::MotionPlaybackDirection::Alternate,
    battlement::MotionPlaybackDirection::AlternateReverse => {
      wire::MotionPlaybackDirection::AlternateReverse
    }
  }
}

fn pointer_device(value: battlement::MotionPointerDevice) -> wire::MotionPointerDevice {
  match value {
    battlement::MotionPointerDevice::Mouse => wire::MotionPointerDevice::Mouse,
    battlement::MotionPointerDevice::Pen => wire::MotionPointerDevice::Pen,
    battlement::MotionPointerDevice::Touch => wire::MotionPointerDevice::Touch,
    battlement::MotionPointerDevice::Keyboard => wire::MotionPointerDevice::Keyboard,
    battlement::MotionPointerDevice::Gamepad => wire::MotionPointerDevice::Gamepad,
  }
}

pub(crate) fn uuid(value: &uuid::Uuid) -> common::Uuid {
  common::Uuid::new(value.as_bytes())
}
