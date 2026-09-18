use battlement_flatbuffers::schema_generated::motion_generated as wire;

use crate::response_reader::object_id;

pub(crate) fn value_operation(
  value: wire::MotionValueOperation<'_>,
) -> Result<battlement::MotionValueOperation, String> {
  let command = match value.command() {
    wire::MotionValueCommandKind::Set => {
      battlement::MotionValueCommand::Set(motion_value(value.value_type(), value)?)
    }
    wire::MotionValueCommandKind::Jump => {
      battlement::MotionValueCommand::Jump(motion_value(value.value_type(), value)?)
    }
    wire::MotionValueCommandKind::Stop => battlement::MotionValueCommand::Stop,
    wire::MotionValueCommandKind::Animate => battlement::MotionValueCommand::Animate {
      playback_id: object_id(
        value
          .playback_id()
          .ok_or_else(|| "Motion value playback identity is missing".to_owned())?,
      )?,
      generation: value.generation(),
      target: Box::new(motion_value(value.value_type(), value)?),
      transition: transition(
        value
          .transition()
          .ok_or_else(|| "Motion value transition is missing".to_owned())?,
      )?,
    },
    _ => return Err("Motion value command is unknown".to_owned()),
  };
  Ok(battlement::MotionValueOperation {
    value_id: object_id(value.value_id())?,
    command,
  })
}

pub(crate) fn value_playback_operation(
  value: wire::MotionValuePlaybackOperation<'_>,
) -> Result<battlement::MotionValuePlaybackOperation, String> {
  Ok(battlement::MotionValuePlaybackOperation {
    playback_id: object_id(value.playback_id())?,
    generation: value.generation(),
    command: playback_command(value.command())?,
  })
}

pub(crate) fn playback_operation(
  value: wire::MotionPlaybackOperation<'_>,
) -> Result<battlement::MotionPlaybackOperation, String> {
  Ok(battlement::MotionPlaybackOperation {
    descriptor_id: object_id(value.descriptor_id())?,
    slot: battlement::MotionSlotId(value.slot()),
    generation: battlement::MotionGeneration(value.generation()),
    command: playback_command(value.command())?,
  })
}

pub(crate) fn controlled_clock_operation(
  value: wire::MotionControlledClockOperation<'_>,
) -> Result<battlement::MotionControlledClockOperation, String> {
  let command = match value.command() {
    wire::MotionControlledClockCommandKind::Set => battlement::MotionControlledClockCommand::Set {
      elapsed_micros: value.micros(),
    },
    wire::MotionControlledClockCommandKind::Advance => {
      battlement::MotionControlledClockCommand::Advance {
        delta_micros: value.micros(),
      }
    }
    _ => return Err("Motion controlled-clock command is unknown".to_owned()),
  };
  Ok(battlement::MotionControlledClockOperation {
    clock_id: object_id(value.clock_id())?,
    command,
  })
}

pub(crate) fn control_operation(
  value: wire::MotionControlOperation<'_>,
) -> Result<battlement::MotionControlOperation, String> {
  let command = match value.command() {
    wire::MotionControlCommandKind::Start => battlement::MotionControlCommand::Start {
      playback_id: object_id(
        value
          .playback_id()
          .ok_or_else(|| "Motion control playback identity is missing".to_owned())?,
      )?,
      generation: value.generation(),
      target: control_target(
        value
          .target()
          .ok_or_else(|| "Motion control target is missing".to_owned())?,
      )?,
    },
    wire::MotionControlCommandKind::Set => battlement::MotionControlCommand::Set(control_target(
      value
        .target()
        .ok_or_else(|| "Motion control target is missing".to_owned())?,
    )?),
    wire::MotionControlCommandKind::Stop => battlement::MotionControlCommand::Stop,
    wire::MotionControlCommandKind::Clear => battlement::MotionControlCommand::Clear,
    _ => return Err("Motion control command is unknown".to_owned()),
  };
  Ok(battlement::MotionControlOperation {
    control_id: object_id(value.control_id())?,
    command,
  })
}

pub(crate) fn scope_operation(
  value: wire::MotionScopeOperation<'_>,
) -> Result<battlement::MotionScopeOperation, String> {
  let command = match value.command() {
    wire::MotionScopeCommandKind::Start => battlement::MotionScopeCommand::Start {
      playback_id: object_id(
        value
          .playback_id()
          .ok_or_else(|| "Motion scope playback identity is missing".to_owned())?,
      )?,
      generation: value.generation(),
      entries: value
        .entries()
        .ok_or_else(|| "Motion scope entries are missing".to_owned())?
        .iter()
        .map(sequence_entry)
        .collect::<Result<_, String>>()?,
    },
    wire::MotionScopeCommandKind::Set => battlement::MotionScopeCommand::Set {
      selector: selector(
        value
          .selector()
          .ok_or_else(|| "Motion scope selector is missing".to_owned())?,
      )?,
      target: target(
        value
          .target()
          .ok_or_else(|| "Motion scope target is missing".to_owned())?,
      )?,
    },
    wire::MotionScopeCommandKind::Stop => battlement::MotionScopeCommand::Stop(selector(
      value
        .selector()
        .ok_or_else(|| "Motion scope selector is missing".to_owned())?,
    )?),
    _ => return Err("Motion scope command is unknown".to_owned()),
  };
  Ok(battlement::MotionScopeOperation {
    scope_id: object_id(value.scope_id())?,
    command,
  })
}

fn sequence_entry(
  value: wire::MotionSequenceEntry<'_>,
) -> Result<battlement::MotionSequenceEntry, String> {
  let schedule = sequence_schedule(value.schedule())?;
  match value.kind() {
    wire::MotionSequenceEntryKind::Animate => Ok(battlement::MotionSequenceEntry::Animate {
      selector: selector(
        value
          .selector()
          .ok_or_else(|| "Motion sequence selector is missing".to_owned())?,
      )?,
      target: target(
        value
          .target()
          .ok_or_else(|| "Motion sequence target is missing".to_owned())?,
      )?,
      position: value.position().map(position_reference).transpose()?,
      position_transition: Box::new(transition(
        value
          .position_transition()
          .ok_or_else(|| "Motion sequence position transition is missing".to_owned())?,
      )?),
      schedule,
      conflict: match value.conflict() {
        wire::MotionSequenceConflict::Reject => battlement::MotionSequenceConflict::Reject,
        wire::MotionSequenceConflict::Replace => battlement::MotionSequenceConflict::Replace,
        _ => return Err("Motion sequence conflict behavior is unknown".to_owned()),
      },
    }),
    wire::MotionSequenceEntryKind::Label => Ok(battlement::MotionSequenceEntry::Label {
      name: value
        .label()
        .ok_or_else(|| "Motion sequence label is missing".to_owned())?
        .to_owned(),
      schedule,
    }),
    wire::MotionSequenceEntryKind::Sound => Ok(battlement::MotionSequenceEntry::Sound {
      sound: battlement::MotionSoundOccurrence {
        address: value
          .effect_address()
          .ok_or_else(|| "Motion sequence sound address is missing".to_owned())?
          .to_owned(),
        volume: value.effect_volume(),
        pitch: value.effect_pitch(),
        looping: value.effect_loop(),
        fade_in_ms: value.effect_fade_in_millis(),
      },
      schedule,
    }),
    wire::MotionSequenceEntryKind::Particle => Ok(battlement::MotionSequenceEntry::Particle {
      particle: battlement::MotionParticleOccurrence {
        address: value
          .effect_address()
          .ok_or_else(|| "Motion sequence particle address is missing".to_owned())?
          .to_owned(),
        position: position_reference(
          value
            .effect_position()
            .ok_or_else(|| "Motion sequence particle position is missing".to_owned())?,
        )?,
        lifetime_ms: value.effect_lifetime_millis(),
      },
      schedule,
    }),
    _ => Err("Motion sequence entry kind is unknown".to_owned()),
  }
}

fn sequence_schedule(
  value: wire::MotionSequenceSchedule<'_>,
) -> Result<battlement::MotionSequenceSchedule, String> {
  Ok(match value.kind() {
    wire::MotionSequenceScheduleKind::Absolute => {
      battlement::MotionSequenceSchedule::Absolute(value.absolute_micros())
    }
    wire::MotionSequenceScheduleKind::RelativeStart => {
      battlement::MotionSequenceSchedule::RelativeStart {
        entry: value.entry(),
        offset_micros: value.offset_micros(),
      }
    }
    wire::MotionSequenceScheduleKind::AfterCompletion => {
      battlement::MotionSequenceSchedule::AfterCompletion {
        entry: value.entry(),
        offset_micros: value.offset_micros(),
      }
    }
    wire::MotionSequenceScheduleKind::Label => battlement::MotionSequenceSchedule::Label {
      name: value
        .label()
        .ok_or_else(|| "Motion sequence schedule label is missing".to_owned())?
        .to_owned(),
      offset_micros: value.offset_micros(),
    },
    _ => return Err("Motion sequence schedule kind is unknown".to_owned()),
  })
}

fn position_reference(
  value: wire::MotionPositionReference<'_>,
) -> Result<battlement::MotionPositionReference, String> {
  let offset = value.offset();
  Ok(battlement::MotionPositionReference {
    object_id: object_id(value.object_id())?,
    anchor: value.anchor().map(str::to_owned),
    offset: battlement::Vector3::new(
      f64::from(offset.x()),
      f64::from(offset.y()),
      f64::from(offset.z()),
    ),
    resolution: match value.resolution() {
      wire::MotionReferenceResolution::CaptureAtStart => {
        battlement::MotionReferenceResolution::CaptureAtStart
      }
      wire::MotionReferenceResolution::Follow => battlement::MotionReferenceResolution::Follow,
      _ => return Err("Motion position reference behavior is unknown".to_owned()),
    },
  })
}

pub(crate) fn drag_control_operation(
  value: wire::MotionDragControlOperation<'_>,
) -> Result<battlement::MotionDragControlOperation, String> {
  Ok(battlement::MotionDragControlOperation {
    control_id: object_id(value.control_id())?,
    pointer_id: value.pointer_id(),
    device: pointer_device(value.device())?,
    point: battlement::MotionGestureVector {
      x: value.point().x(),
      y: value.point().y(),
    },
    snap_to_cursor: value.snap_to_cursor(),
  })
}

fn playback_command(
  value: wire::MotionPlaybackCommand<'_>,
) -> Result<battlement::MotionPlaybackCommand, String> {
  Ok(match value.kind() {
    wire::MotionPlaybackCommandKind::Play => battlement::MotionPlaybackCommand::Play,
    wire::MotionPlaybackCommandKind::Pause => battlement::MotionPlaybackCommand::Pause,
    wire::MotionPlaybackCommandKind::Replay => battlement::MotionPlaybackCommand::Replay,
    wire::MotionPlaybackCommandKind::Stop => battlement::MotionPlaybackCommand::Stop,
    wire::MotionPlaybackCommandKind::Cancel => battlement::MotionPlaybackCommand::Cancel,
    wire::MotionPlaybackCommandKind::Complete => battlement::MotionPlaybackCommand::Complete,
    wire::MotionPlaybackCommandKind::Seek => battlement::MotionPlaybackCommand::Seek {
      elapsed_micros: value.elapsed_micros(),
    },
    wire::MotionPlaybackCommandKind::SetSpeed => battlement::MotionPlaybackCommand::SetSpeed {
      value: value.speed(),
    },
    wire::MotionPlaybackCommandKind::SetDirection => {
      battlement::MotionPlaybackCommand::SetDirection {
        value: playback_direction(value.direction())?,
      }
    }
    _ => return Err("Motion playback command is unknown".to_owned()),
  })
}

fn control_target(
  value: wire::MotionControlTarget<'_>,
) -> Result<battlement::MotionControlTarget, String> {
  Ok(match value.kind() {
    wire::MotionControlTargetKind::Target => battlement::MotionControlTarget::Target(target(
      value
        .target()
        .ok_or_else(|| "Motion target is missing".to_owned())?,
    )?),
    wire::MotionControlTargetKind::Variant => battlement::MotionControlTarget::Variant(
      value
        .variant()
        .ok_or_else(|| "Motion variant is missing".to_owned())?
        .to_owned(),
    ),
    _ => return Err("Motion control target kind is unknown".to_owned()),
  })
}

fn selector(value: wire::MotionSelector<'_>) -> Result<battlement::MotionSelector, String> {
  Ok(match value.kind() {
    wire::MotionSelectorKind::Element => battlement::MotionSelector::Element(object_id(
      value
        .object_id()
        .ok_or_else(|| "Motion selector identity is missing".to_owned())?,
    )?),
    wire::MotionSelectorKind::Name => battlement::MotionSelector::Name(
      value
        .name()
        .ok_or_else(|| "Motion selector name is missing".to_owned())?
        .to_owned(),
    ),
    wire::MotionSelectorKind::ScopeRoot => battlement::MotionSelector::ScopeRoot,
    wire::MotionSelectorKind::Children => battlement::MotionSelector::Children,
    wire::MotionSelectorKind::Descendants => battlement::MotionSelector::Descendants,
    _ => return Err("Motion selector kind is unknown".to_owned()),
  })
}

pub(crate) fn target(
  value: wire::MotionTargetDescriptor<'_>,
) -> Result<battlement::MotionTargetDescriptor, String> {
  Ok(battlement::MotionTargetDescriptor {
    tracks: value
      .tracks()
      .iter()
      .map(|track| {
        Ok(battlement::MotionPropertyTrack {
          property: property(track.property())?,
          target: property_target(track.target())?,
          values: track
            .values()
            .iter()
            .map(|entry| motion_value(entry.value_type(), entry))
            .collect::<Result<_, _>>()?,
          times: track.times().map(|values| values.iter().collect()),
          transition: transition(track.transition())?,
        })
      })
      .collect::<Result<_, String>>()?,
    transition_end: value
      .transition_end()
      .iter()
      .map(|entry| {
        Ok(battlement::MotionPropertyValue {
          property: property(entry.property())?,
          value: motion_value(entry.value_type(), entry)?,
        })
      })
      .collect::<Result<_, String>>()?,
  })
}

fn property_target(
  value: wire::MotionPropertyTarget<'_>,
) -> Result<battlement::MotionPropertyTarget, String> {
  Ok(match value.kind() {
    wire::MotionPropertyTargetKind::Host => battlement::MotionPropertyTarget::Host,
    wire::MotionPropertyTargetKind::MaterialScalar => {
      battlement::MotionPropertyTarget::MaterialScalar {
        slot: value.material_slot(),
        parameter: value
          .material_parameter()
          .ok_or_else(|| "material scalar Motion target has no parameter".to_owned())?
          .to_owned(),
      }
    }
    wire::MotionPropertyTargetKind::AudioVolume => battlement::MotionPropertyTarget::AudioVolume {
      playback_id: object_id(
        value
          .playback_id()
          .ok_or_else(|| "audio volume Motion target has no playback".to_owned())?,
      )?,
    },
    _ => return Err("Motion property target kind is unknown".to_owned()),
  })
}

pub(crate) trait MotionValueSource<'a> {
  fn scalar(self) -> Option<wire::ScalarMotionValue<'a>>;
  fn length(self) -> Option<wire::LengthMotionValue<'a>>;
  fn color(self) -> Option<wire::ColorMotionValue<'a>>;
  fn vector2(self) -> Option<wire::Vector2MotionValue<'a>>;
  fn vector3(self) -> Option<wire::Vector3MotionValue<'a>>;
  fn angle(self) -> Option<wire::AngleMotionValue<'a>>;
  fn transforms(self) -> Option<wire::TransformListMotionValue<'a>>;
  fn filters(self) -> Option<wire::FilterListMotionValue<'a>>;
  fn shadows(self) -> Option<wire::ShadowListMotionValue<'a>>;
  fn gradient(self) -> Option<wire::GradientMotionValue<'a>>;
  fn inset(self) -> Option<wire::ClipInsetMotionValue<'a>>;
  fn polygon(self) -> Option<wire::ClipPolygonMotionValue<'a>>;
  fn discrete(self) -> Option<wire::DiscreteMotionValue<'a>>;
}

macro_rules! motion_value_source {
  ($type:ty) => {
    impl<'a> MotionValueSource<'a> for $type {
      fn scalar(self) -> Option<wire::ScalarMotionValue<'a>> {
        self.value_as_scalar_motion_value()
      }
      fn length(self) -> Option<wire::LengthMotionValue<'a>> {
        self.value_as_length_motion_value()
      }
      fn color(self) -> Option<wire::ColorMotionValue<'a>> {
        self.value_as_color_motion_value()
      }
      fn vector2(self) -> Option<wire::Vector2MotionValue<'a>> {
        self.value_as_vector_2_motion_value()
      }
      fn vector3(self) -> Option<wire::Vector3MotionValue<'a>> {
        self.value_as_vector_3_motion_value()
      }
      fn angle(self) -> Option<wire::AngleMotionValue<'a>> {
        self.value_as_angle_motion_value()
      }
      fn transforms(self) -> Option<wire::TransformListMotionValue<'a>> {
        self.value_as_transform_list_motion_value()
      }
      fn filters(self) -> Option<wire::FilterListMotionValue<'a>> {
        self.value_as_filter_list_motion_value()
      }
      fn shadows(self) -> Option<wire::ShadowListMotionValue<'a>> {
        self.value_as_shadow_list_motion_value()
      }
      fn gradient(self) -> Option<wire::GradientMotionValue<'a>> {
        self.value_as_gradient_motion_value()
      }
      fn inset(self) -> Option<wire::ClipInsetMotionValue<'a>> {
        self.value_as_clip_inset_motion_value()
      }
      fn polygon(self) -> Option<wire::ClipPolygonMotionValue<'a>> {
        self.value_as_clip_polygon_motion_value()
      }
      fn discrete(self) -> Option<wire::DiscreteMotionValue<'a>> {
        self.value_as_discrete_motion_value()
      }
    }
  };
}

motion_value_source!(wire::MotionValueOperation<'a>);
motion_value_source!(wire::MotionValueEntry<'a>);
motion_value_source!(wire::MotionPropertyValue<'a>);

pub(crate) fn motion_value<'a>(
  kind: wire::MotionValue,
  source: impl MotionValueSource<'a> + Copy,
) -> Result<battlement::MotionValue, String> {
  let missing = || "Motion value payload is missing".to_owned();
  Ok(match kind {
    wire::MotionValue::ScalarMotionValue => {
      battlement::MotionValue::Scalar(source.scalar().ok_or_else(missing)?.value())
    }
    wire::MotionValue::LengthMotionValue => {
      battlement::MotionValue::Length(length(source.length().ok_or_else(missing)?.value()))
    }
    wire::MotionValue::ColorMotionValue => {
      battlement::MotionValue::Color(color(source.color().ok_or_else(missing)?.value()))
    }
    wire::MotionValue::Vector2MotionValue => {
      let value = source.vector2().ok_or_else(missing)?.value();
      battlement::MotionValue::Vector2([value.x(), value.y()])
    }
    wire::MotionValue::Vector3MotionValue => {
      let value = source.vector3().ok_or_else(missing)?.value();
      battlement::MotionValue::Vector3([value.x(), value.y(), value.z()])
    }
    wire::MotionValue::AngleMotionValue => {
      battlement::MotionValue::Angle(source.angle().ok_or_else(missing)?.value())
    }
    wire::MotionValue::TransformListMotionValue => battlement::MotionValue::TransformList(
      source
        .transforms()
        .ok_or_else(missing)?
        .values()
        .iter()
        .map(transform)
        .collect::<Result<_, _>>()?,
    ),
    wire::MotionValue::FilterListMotionValue => {
      battlement::MotionValue::FilterList(battlement::FilterList::new(
        source
          .filters()
          .ok_or_else(missing)?
          .values()
          .iter()
          .map(filter)
          .collect::<Result<Vec<_>, _>>()?,
      ))
    }
    wire::MotionValue::ShadowListMotionValue => battlement::MotionValue::ShadowList(
      source
        .shadows()
        .ok_or_else(missing)?
        .values()
        .iter()
        .map(shadow)
        .collect(),
    ),
    wire::MotionValue::GradientMotionValue => {
      battlement::MotionValue::Gradient(gradient(source.gradient().ok_or_else(missing)?)?)
    }
    wire::MotionValue::ClipInsetMotionValue => battlement::MotionValue::ClipInset(
      source
        .inset()
        .ok_or_else(missing)?
        .values()
        .iter()
        .map(length)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "Motion clip inset does not have four values".to_owned())?,
    ),
    wire::MotionValue::ClipPolygonMotionValue => battlement::MotionValue::ClipPolygon(
      source
        .polygon()
        .ok_or_else(missing)?
        .values()
        .iter()
        .map(|point| [length(point.x()), length(point.y())])
        .collect(),
    ),
    wire::MotionValue::DiscreteMotionValue => {
      let value = source.discrete().ok_or_else(missing)?;
      battlement::MotionValue::Discrete(match value.kind() {
        wire::MotionDiscreteKind::Null => battlement::MotionDiscreteValue::Null,
        wire::MotionDiscreteKind::String => battlement::MotionDiscreteValue::String(
          value
            .string_value()
            .ok_or_else(|| "Motion discrete string is missing".to_owned())?
            .to_owned(),
        ),
        _ => return Err("Motion discrete kind is unknown".to_owned()),
      })
    }
    _ => return Err("Motion value kind is unknown".to_owned()),
  })
}

pub(crate) fn transition(
  value: wire::TransitionDefinition<'_>,
) -> Result<battlement::TransitionDefinition, String> {
  let generator = match value.generator() {
    wire::TransitionGeneratorKind::Immediate => battlement::TransitionGenerator::Immediate,
    wire::TransitionGeneratorKind::Tween => battlement::TransitionGenerator::Tween {
      duration_micros: value.duration_micros(),
      easings: value
        .easings()
        .map(|values| values.iter().map(easing).collect())
        .transpose()?
        .unwrap_or_default(),
      times: value.times().map(|values| values.iter().collect()),
    },
    wire::TransitionGeneratorKind::SpringPhysical => {
      battlement::TransitionGenerator::Spring(battlement::SpringConfiguration::Physical {
        stiffness: value.stiffness(),
        damping: value.damping(),
        mass: value.mass(),
        initial_velocity: value.initial_velocity(),
        rest_speed: value.rest_speed(),
        rest_delta: value.rest_delta(),
      })
    }
    wire::TransitionGeneratorKind::SpringDuration => {
      battlement::TransitionGenerator::Spring(battlement::SpringConfiguration::Duration {
        duration_micros: value.duration_micros(),
        bounce: value.bounce(),
        mass: value.mass(),
      })
    }
    wire::TransitionGeneratorKind::SpringVisualDuration => {
      battlement::TransitionGenerator::Spring(battlement::SpringConfiguration::VisualDuration {
        duration_micros: value.duration_micros(),
        bounce: value.bounce(),
        mass: value.mass(),
      })
    }
    wire::TransitionGeneratorKind::Inertia => battlement::TransitionGenerator::Inertia {
      initial_velocity: value
        .initial_velocity()
        .ok_or_else(|| "Motion inertia velocity is missing".to_owned())?,
      power: value.power(),
      time_constant_micros: value.time_constant_micros(),
      minimum: value.minimum(),
      maximum: value.maximum(),
      rest_delta: value
        .rest_delta()
        .ok_or_else(|| "Motion inertia rest delta is missing".to_owned())?,
      bounce_stiffness: value.bounce_stiffness(),
      bounce_damping: value.bounce_damping(),
      target: inertia_target(value)?,
    },
    _ => return Err("Motion transition generator is unknown".to_owned()),
  };
  Ok(battlement::TransitionDefinition {
    generator,
    delay_micros: value.delay_micros(),
    repeat: match value.repeat() {
      wire::MotionRepeatKind::None => battlement::MotionRepeat::None,
      wire::MotionRepeatKind::Count => battlement::MotionRepeat::Count(value.repeat_count()),
      wire::MotionRepeatKind::Forever => battlement::MotionRepeat::Forever,
      _ => return Err("Motion repeat kind is unknown".to_owned()),
    },
    repeat_delay_micros: value.repeat_delay_micros(),
    repeat_type: match value.repeat_type() {
      wire::MotionRepeatType::Loop => battlement::MotionRepeatType::Loop,
      wire::MotionRepeatType::Reverse => battlement::MotionRepeatType::Reverse,
      wire::MotionRepeatType::Mirror => battlement::MotionRepeatType::Mirror,
      _ => return Err("Motion repeat type is unknown".to_owned()),
    },
  })
}

fn easing(value: wire::MotionEasingDefinition<'_>) -> Result<battlement::MotionEasing, String> {
  Ok(match value.kind() {
    wire::MotionEasingKind::Linear => battlement::MotionEasing::Linear,
    wire::MotionEasingKind::EaseIn => battlement::MotionEasing::EaseIn,
    wire::MotionEasingKind::EaseOut => battlement::MotionEasing::EaseOut,
    wire::MotionEasingKind::EaseInOut => battlement::MotionEasing::EaseInOut,
    wire::MotionEasingKind::CubicBezier => battlement::MotionEasing::CubicBezier(
      value
        .cubic_bezier()
        .ok_or_else(|| "Motion cubic Bézier values are missing".to_owned())?
        .iter()
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "Motion cubic Bézier does not have four values".to_owned())?,
    ),
    wire::MotionEasingKind::Steps => battlement::MotionEasing::Steps {
      count: value.step_count(),
      position: match value.step_position() {
        wire::StepPosition::Start => battlement::StepPosition::Start,
        wire::StepPosition::End => battlement::StepPosition::End,
        _ => return Err("Motion step position is unknown".to_owned()),
      },
    },
    _ => return Err("Motion easing kind is unknown".to_owned()),
  })
}

fn inertia_target(
  value: wire::TransitionDefinition<'_>,
) -> Result<battlement::InertiaTarget, String> {
  Ok(match value.inertia_target() {
    wire::InertiaTargetKind::Identity => battlement::InertiaTarget::Identity,
    wire::InertiaTargetKind::NearestMultiple => {
      battlement::InertiaTarget::NearestMultiple(value.inertia_target_value())
    }
    wire::InertiaTargetKind::FloorMultiple => {
      battlement::InertiaTarget::FloorMultiple(value.inertia_target_value())
    }
    wire::InertiaTargetKind::CeilingMultiple => {
      battlement::InertiaTarget::CeilingMultiple(value.inertia_target_value())
    }
    wire::InertiaTargetKind::Clamp => battlement::InertiaTarget::Clamp {
      min: value.inertia_target_value(),
      max: value.inertia_target_maximum(),
    },
    _ => return Err("Motion inertia target is unknown".to_owned()),
  })
}

fn transform(value: wire::MotionTransform<'_>) -> Result<battlement::TransformOperation, String> {
  Ok(match value.kind() {
    wire::MotionTransformKind::Translate => battlement::TransformOperation::Translate(
      value
        .lengths()
        .ok_or_else(|| "Motion translation values are missing".to_owned())?
        .iter()
        .map(length)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "Motion translation does not have three values".to_owned())?,
    ),
    wire::MotionTransformKind::Rotate => battlement::TransformOperation::Rotate(
      scalar_array(value.scalars(), 3)?
        .try_into()
        .expect("length was checked"),
    ),
    wire::MotionTransformKind::Skew => battlement::TransformOperation::Skew(
      scalar_array(value.scalars(), 2)?
        .try_into()
        .expect("length was checked"),
    ),
    wire::MotionTransformKind::Scale => battlement::TransformOperation::Scale(
      scalar_array(value.scalars(), 3)?
        .try_into()
        .expect("length was checked"),
    ),
    _ => return Err("Motion transform kind is unknown".to_owned()),
  })
}

fn scalar_array(
  values: Option<flatbuffers::Vector<'_, f32>>,
  expected: usize,
) -> Result<Vec<f32>, String> {
  let values = values
    .ok_or_else(|| "Motion transform scalars are missing".to_owned())?
    .iter()
    .collect::<Vec<_>>();
  if values.len() != expected {
    return Err("Motion transform has the wrong number of values".to_owned());
  }
  Ok(values)
}

fn filter(value: wire::MotionFilter<'_>) -> Result<battlement::FilterFunction, String> {
  Ok(match value.kind() {
    wire::MotionFilterKind::Brightness => {
      battlement::FilterFunction::Brightness(value.brightness())
    }
    wire::MotionFilterKind::DropShadow => battlement::FilterFunction::DropShadow(shadow(
      value
        .shadow()
        .ok_or_else(|| "Motion drop shadow is missing".to_owned())?,
    )),
    _ => return Err("Motion filter kind is unknown".to_owned()),
  })
}

fn gradient(value: wire::GradientMotionValue<'_>) -> Result<battlement::Gradient, String> {
  let stops = value
    .stops()
    .iter()
    .map(|stop| battlement::GradientStop {
      color: color(stop.color()),
      position: stop.position(),
    })
    .collect();
  Ok(match value.kind() {
    wire::MotionGradientKind::Linear => battlement::Gradient::Linear {
      angle: value.angle(),
      stops,
    },
    wire::MotionGradientKind::Radial => battlement::Gradient::Radial {
      center: {
        let center = value
          .center()
          .ok_or_else(|| "Motion radial-gradient center is missing".to_owned())?;
        [center.x(), center.y()]
      },
      radius: {
        let radius = value
          .radius()
          .ok_or_else(|| "Motion radial-gradient radius is missing".to_owned())?;
        [radius.x(), radius.y()]
      },
      stops,
    },
    _ => return Err("Motion gradient kind is unknown".to_owned()),
  })
}

pub(crate) fn property(value: wire::MotionProperty) -> Result<battlement::MotionProperty, String> {
  battlement::MotionProperty::ALL
    .get(usize::from(value.0))
    .copied()
    .ok_or_else(|| "Motion property is unknown".to_owned())
}

pub(crate) fn length(value: &wire::MotionLength) -> battlement::Length {
  battlement::Length::calc(value.pixels(), value.percentage())
}

pub(crate) fn color(value: &wire::MotionColor) -> battlement::Color {
  battlement::Color {
    r: value.red(),
    g: value.green(),
    b: value.blue(),
    a: value.alpha(),
  }
}

pub(crate) fn shadow(value: &wire::MotionShadow) -> battlement::Shadow {
  battlement::Shadow {
    x: value.x(),
    y: value.y(),
    blur: value.blur(),
    spread: value.spread(),
    color: color(value.color()),
    inset: value.inset(),
  }
}

fn playback_direction(
  value: wire::MotionPlaybackDirection,
) -> Result<battlement::MotionPlaybackDirection, String> {
  match value {
    wire::MotionPlaybackDirection::Forward => Ok(battlement::MotionPlaybackDirection::Forward),
    wire::MotionPlaybackDirection::Reverse => Ok(battlement::MotionPlaybackDirection::Reverse),
    wire::MotionPlaybackDirection::Alternate => Ok(battlement::MotionPlaybackDirection::Alternate),
    wire::MotionPlaybackDirection::AlternateReverse => {
      Ok(battlement::MotionPlaybackDirection::AlternateReverse)
    }
    _ => Err("Motion playback direction is unknown".to_owned()),
  }
}

fn pointer_device(
  value: wire::MotionPointerDevice,
) -> Result<battlement::MotionPointerDevice, String> {
  match value {
    wire::MotionPointerDevice::Mouse => Ok(battlement::MotionPointerDevice::Mouse),
    wire::MotionPointerDevice::Pen => Ok(battlement::MotionPointerDevice::Pen),
    wire::MotionPointerDevice::Touch => Ok(battlement::MotionPointerDevice::Touch),
    wire::MotionPointerDevice::Keyboard => Ok(battlement::MotionPointerDevice::Keyboard),
    wire::MotionPointerDevice::Gamepad => Ok(battlement::MotionPointerDevice::Gamepad),
    _ => Err("Motion pointer device is unknown".to_owned()),
  }
}
