use battlement::{
  AnimationComposition, AnimationDirection, AnimationFill, AnimationPlayState, DecorationOverflow,
  DecorationPlacement, DecorationPosition, MotionClockSource, MotionExpressionOperation,
  MotionGestureAxis, MotionLayer, MotionPseudoState, MotionValueSource, ReducedMotionPolicy,
  SpringConfiguration,
};
use flatbuffers::{Allocator, FlatBufferBuilder, WIPOffset};

use crate::{
  ProtocolError, common_generated as common, motion_generated as motion_wire, response_motion,
  response_ui, ui_generated as wire,
};

pub(crate) fn write_descriptor<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionDescriptor,
) -> Result<WIPOffset<wire::MotionDescriptor<'a>>, ProtocolError> {
  value.validate().map_err(ProtocolError::new)?;
  let initial = value
    .initial
    .as_ref()
    .map(|value| response_motion::write_target(builder, value))
    .transpose()?;
  let slots = value
    .slots
    .iter()
    .map(|value| write_slot(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let clock = write_clock(builder, value.clock);
  let pseudo_styles = value
    .pseudo_styles
    .iter()
    .map(|value| write_pseudo_style(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let style_transition = write_style_transition(builder, &value.style_transition)?;
  let animations = value
    .animations
    .iter()
    .map(|value| write_animation(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let decorations = value
    .decorations
    .iter()
    .map(|value| write_decoration(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let variants = value
    .variants
    .as_ref()
    .map(|value| write_variants(builder, value));
  let values = value
    .values
    .iter()
    .map(|value| write_value_descriptor(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let value_bindings = value
    .value_bindings
    .iter()
    .map(|value| write_value_binding(builder, value))
    .collect::<Vec<_>>();
  let value_subscriptions = value
    .value_subscriptions
    .iter()
    .map(|value| write_value_subscription(builder, value))
    .collect::<Vec<_>>();
  let named_targets = value
    .named_targets
    .iter()
    .map(|value| write_named_target(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let gestures = value
    .gestures
    .as_ref()
    .map(|value| write_gestures(builder, value));
  let layout = value
    .layout
    .as_ref()
    .map(|value| write_layout(builder, value))
    .transpose()?;
  let motion_name = value
    .motion_name
    .as_ref()
    .map(|value| builder.create_string(value));
  let descriptor_id = uuid(value.descriptor_id.as_uuid());
  let host_id = uuid(value.host_id.as_uuid());
  let control_id = value.control_id.map(|value| uuid(value.as_uuid()));
  let scope_id = value.scope_id.map(|value| uuid(value.as_uuid()));
  let slots = builder.create_vector(&slots);
  let pseudo_styles = builder.create_vector(&pseudo_styles);
  let animations = builder.create_vector(&animations);
  let decorations = builder.create_vector(&decorations);
  let values = builder.create_vector(&values);
  let value_bindings = builder.create_vector(&value_bindings);
  let value_subscriptions = builder.create_vector(&value_subscriptions);
  let named_targets = builder.create_vector(&named_targets);
  Ok(wire::MotionDescriptor::create(
    builder,
    &wire::MotionDescriptorArgs {
      descriptor_id: Some(&descriptor_id),
      host_id: Some(&host_id),
      generation: value.generation.0,
      initial,
      initial_disabled: value.initial_disabled,
      slots: Some(slots),
      clock: Some(clock),
      reduced_motion: reduced_motion(value.reduced_motion),
      pseudo_styles: Some(pseudo_styles),
      style_transition: Some(style_transition),
      animations: Some(animations),
      decorations: Some(decorations),
      variants,
      values: Some(values),
      value_bindings: Some(value_bindings),
      value_subscriptions: Some(value_subscriptions),
      control_id: control_id.as_ref(),
      scope_id: scope_id.as_ref(),
      scope_root: value.scope_root,
      motion_name,
      named_targets: Some(named_targets),
      gestures,
      layout,
    },
  ))
}

fn write_slot<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionSlotDescriptor,
) -> Result<WIPOffset<wire::MotionSlotDescriptor<'a>>, ProtocolError> {
  let target = response_motion::write_target(builder, &value.target)?;
  let callbacks = wire::MotionCallbackSubscriptions::create(
    builder,
    &wire::MotionCallbackSubscriptionsArgs {
      start: value.callbacks.start,
      update: value.callbacks.update,
      repeat: value.callbacks.repeat,
      complete: value.callbacks.complete,
      stop: value.callbacks.stop,
      cancel: value.callbacks.cancel,
    },
  );
  Ok(wire::MotionSlotDescriptor::create(
    builder,
    &wire::MotionSlotDescriptorArgs {
      slot: value.slot.0,
      generation: value.generation.0,
      layer: motion_layer(value.layer),
      target: Some(target),
      callbacks: Some(callbacks),
    },
  ))
}

fn write_clock<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: MotionClockSource,
) -> WIPOffset<wire::MotionClockSource<'a>> {
  let (kind, object_id) = match value {
    MotionClockSource::Unscaled => (wire::MotionClockKind::Unscaled, None),
    MotionClockSource::Scaled => (wire::MotionClockKind::Scaled, None),
    MotionClockSource::Controlled(value) => (
      wire::MotionClockKind::Controlled,
      Some(uuid(value.as_uuid())),
    ),
    MotionClockSource::Audio(value) => (wire::MotionClockKind::Audio, Some(uuid(value.as_uuid()))),
  };
  wire::MotionClockSource::create(
    builder,
    &wire::MotionClockSourceArgs {
      kind,
      object_id: object_id.as_ref(),
    },
  )
}

fn write_pseudo_style<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionPseudoStyle,
) -> Result<WIPOffset<wire::MotionPseudoStyle<'a>>, ProtocolError> {
  let values = value
    .values
    .iter()
    .map(|value| response_motion::write_property_value(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let values = builder.create_vector(&values);
  Ok(wire::MotionPseudoStyle::create(
    builder,
    &wire::MotionPseudoStyleArgs {
      state: match value.state {
        MotionPseudoState::Hover => wire::MotionPseudoState::Hover,
        MotionPseudoState::Focus => wire::MotionPseudoState::Focus,
        MotionPseudoState::Active => wire::MotionPseudoState::Active,
        MotionPseudoState::Disabled => wire::MotionPseudoState::Disabled,
      },
      values: Some(values),
    },
  ))
}

fn write_style_transition<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::StyleTransitionDescriptor,
) -> Result<WIPOffset<wire::StyleTransitionDescriptor<'a>>, ProtocolError> {
  let properties = value
    .properties
    .iter()
    .map(|value| {
      let transition = response_motion::write_transition(builder, &value.transition)?;
      Ok(wire::StylePropertyTransition::create(
        builder,
        &wire::StylePropertyTransitionArgs {
          property: response_motion::property(value.property),
          transition: Some(transition),
        },
      ))
    })
    .collect::<Result<Vec<_>, ProtocolError>>()?;
  let all = value
    .all
    .as_ref()
    .map(|value| response_motion::write_transition(builder, value))
    .transpose()?;
  let properties = builder.create_vector(&properties);
  Ok(wire::StyleTransitionDescriptor::create(
    builder,
    &wire::StyleTransitionDescriptorArgs {
      properties: Some(properties),
      all,
      allow_discrete: value.allow_discrete,
    },
  ))
}

fn write_animation<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::CssAnimationDescriptor,
) -> Result<WIPOffset<wire::CssAnimationDescriptor<'a>>, ProtocolError> {
  let tracks = value
    .tracks
    .iter()
    .map(|track| {
      let values = track
        .values
        .iter()
        .map(|value| {
          let value = response_motion::write_value(builder, value)?;
          Ok(motion_wire::MotionValueEntry::create(
            builder,
            &motion_wire::MotionValueEntryArgs {
              value_type: value.kind,
              value: Some(value.value),
            },
          ))
        })
        .collect::<Result<Vec<_>, ProtocolError>>()?;
      let values = builder.create_vector(&values);
      let times = builder.create_vector(&track.times);
      let transition = response_motion::write_transition(builder, &track.transition)?;
      Ok(wire::CssPropertyTrack::create(
        builder,
        &wire::CssPropertyTrackArgs {
          property: response_motion::property(track.property),
          values: Some(values),
          times: Some(times),
          transition: Some(transition),
        },
      ))
    })
    .collect::<Result<Vec<_>, ProtocolError>>()?;
  let tracks = builder.create_vector(&tracks);
  let diagnostic_name = value
    .diagnostic_name
    .as_ref()
    .map(|value| builder.create_string(value));
  Ok(wire::CssAnimationDescriptor::create(
    builder,
    &wire::CssAnimationDescriptorArgs {
      slot: value.slot,
      generation: value.generation,
      restart_key: value.restart_key,
      tracks: Some(tracks),
      direction: animation_direction(value.direction),
      fill: animation_fill(value.fill),
      play_state: animation_play_state(value.play_state),
      composition: animation_composition(value.composition),
      diagnostic_name,
    },
  ))
}

fn write_decoration<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionDecorationDescriptor,
) -> Result<WIPOffset<wire::MotionDecorationDescriptor<'a>>, ProtocolError> {
  let style = response_ui::write_style_properties(builder, &value.style)?;
  let animations = value
    .animations
    .iter()
    .map(|value| write_animation(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let style = builder.create_vector(&style);
  let animations = builder.create_vector(&animations);
  Ok(wire::MotionDecorationDescriptor::create(
    builder,
    &wire::MotionDecorationDescriptorArgs {
      key: value.key,
      placement: match value.placement {
        DecorationPlacement::Before => wire::DecorationPlacement::Before,
        DecorationPlacement::After => wire::DecorationPlacement::After,
      },
      position: match value.position {
        DecorationPosition::Fill => wire::DecorationPosition::Fill,
        DecorationPosition::Border => wire::DecorationPosition::Border,
      },
      overflow: match value.overflow {
        DecorationOverflow::Hidden => wire::DecorationOverflow::Hidden,
        DecorationOverflow::Visible => wire::DecorationOverflow::Visible,
      },
      style: Some(style),
      animations: Some(animations),
    },
  ))
}

fn write_variants<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionVariantResolution,
) -> WIPOffset<wire::MotionVariantResolution<'a>> {
  let names = value
    .names
    .iter()
    .map(|value| builder.create_string(value))
    .collect::<Vec<_>>();
  let names = builder.create_vector(&names);
  wire::MotionVariantResolution::create(
    builder,
    &wire::MotionVariantResolutionArgs {
      names: Some(names),
      inherited: value.inherited,
      custom_snapshot: value.custom_snapshot,
      child_index: value.child_index,
      delay_micros: value.delay_micros,
      when: match value.when {
        battlement::VariantWhen::Together => wire::VariantWhen::Together,
        battlement::VariantWhen::BeforeChildren => wire::VariantWhen::BeforeChildren,
        battlement::VariantWhen::AfterChildren => wire::VariantWhen::AfterChildren,
      },
      stagger_direction: match value.stagger_direction {
        battlement::StaggerDirection::Forward => wire::StaggerDirection::Forward,
        battlement::StaggerDirection::Reverse => wire::StaggerDirection::Reverse,
      },
    },
  )
}

fn write_named_target<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionNamedTarget,
) -> Result<WIPOffset<wire::MotionNamedTarget<'a>>, ProtocolError> {
  let name = builder.create_string(&value.name);
  let target = response_motion::write_target(builder, &value.target)?;
  Ok(wire::MotionNamedTarget::create(
    builder,
    &wire::MotionNamedTargetArgs {
      name: Some(name),
      target: Some(target),
    },
  ))
}

fn write_layout<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionLayoutDescriptor,
) -> Result<WIPOffset<wire::MotionLayoutDescriptor<'a>>, ProtocolError> {
  let group = write_layout_identity(builder, &value.group);
  let layout_id = value
    .layout_id
    .as_ref()
    .map(|value| write_layout_identity(builder, value));
  let transition = response_motion::write_transition(builder, &value.transition)?;
  Ok(wire::MotionLayoutDescriptor::create(
    builder,
    &wire::MotionLayoutDescriptorArgs {
      mode: match value.mode {
        battlement::MotionLayoutMode::Position => wire::MotionLayoutMode::Position,
        battlement::MotionLayoutMode::Size => wire::MotionLayoutMode::Size,
        battlement::MotionLayoutMode::Both => wire::MotionLayoutMode::Both,
      },
      group: Some(group),
      layout_id,
      scroll: value.scroll,
      root: value.root,
      pop_layout: value.pop_layout,
      transition: Some(transition),
    },
  ))
}

fn write_layout_identity<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionLayoutIdentity,
) -> WIPOffset<wire::MotionLayoutIdentity<'a>> {
  let value_type = builder.create_string(&value.value_type);
  wire::MotionLayoutIdentity::create(
    builder,
    &wire::MotionLayoutIdentityArgs {
      value_type: Some(value_type),
      value_hash: value.value_hash,
    },
  )
}

fn motion_layer(value: MotionLayer) -> wire::MotionLayer {
  match value {
    MotionLayer::Animate => wire::MotionLayer::Animate,
    MotionLayer::InView => wire::MotionLayer::InView,
    MotionLayer::Focus => wire::MotionLayer::Focus,
    MotionLayer::FocusVisible => wire::MotionLayer::FocusVisible,
    MotionLayer::Hover => wire::MotionLayer::Hover,
    MotionLayer::Tap => wire::MotionLayer::Tap,
    MotionLayer::Drag => wire::MotionLayer::Drag,
    MotionLayer::Exit => wire::MotionLayer::Exit,
  }
}
fn reduced_motion(value: ReducedMotionPolicy) -> wire::ReducedMotionPolicy {
  match value {
    ReducedMotionPolicy::User => wire::ReducedMotionPolicy::User,
    ReducedMotionPolicy::Always => wire::ReducedMotionPolicy::Always,
    ReducedMotionPolicy::Never => wire::ReducedMotionPolicy::Never,
  }
}
fn animation_direction(value: AnimationDirection) -> wire::AnimationDirection {
  match value {
    AnimationDirection::Normal => wire::AnimationDirection::Normal,
    AnimationDirection::Reverse => wire::AnimationDirection::Reverse,
    AnimationDirection::Alternate => wire::AnimationDirection::Alternate,
    AnimationDirection::AlternateReverse => wire::AnimationDirection::AlternateReverse,
  }
}
fn animation_fill(value: AnimationFill) -> wire::AnimationFill {
  match value {
    AnimationFill::None => wire::AnimationFill::None,
    AnimationFill::Forwards => wire::AnimationFill::Forwards,
    AnimationFill::Backwards => wire::AnimationFill::Backwards,
    AnimationFill::Both => wire::AnimationFill::Both,
  }
}
fn animation_play_state(value: AnimationPlayState) -> wire::AnimationPlayState {
  match value {
    AnimationPlayState::Running => wire::AnimationPlayState::Running,
    AnimationPlayState::Paused => wire::AnimationPlayState::Paused,
  }
}
fn animation_composition(value: AnimationComposition) -> wire::AnimationComposition {
  match value {
    AnimationComposition::Replace => wire::AnimationComposition::Replace,
    AnimationComposition::Add => wire::AnimationComposition::Add,
    AnimationComposition::Accumulate => wire::AnimationComposition::Accumulate,
  }
}
fn uuid(value: &uuid::Uuid) -> common::Uuid {
  common::Uuid::new(value.as_bytes())
}

// Graph and gesture helpers are kept below the descriptor-level lowering so every
// nested table is authored directly into the same finished response allocation.
fn write_value_descriptor<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionValueDescriptor,
) -> Result<WIPOffset<wire::MotionValueDescriptor<'a>>, ProtocolError> {
  let initial = response_motion::write_value(builder, &value.initial)?;
  let source = write_value_source(builder, &value.source)?;
  let value_id = uuid(value.value_id.as_uuid());
  Ok(wire::MotionValueDescriptor::create(
    builder,
    &wire::MotionValueDescriptorArgs {
      value_id: Some(&value_id),
      initial_type: initial.kind,
      initial: Some(initial.value),
      source: Some(source),
    },
  ))
}
fn write_value_binding<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionValueBinding,
) -> WIPOffset<wire::MotionValueBinding<'a>> {
  let value_id = uuid(value.value_id.as_uuid());
  wire::MotionValueBinding::create(
    builder,
    &wire::MotionValueBindingArgs {
      property: response_motion::property(value.property),
      value_id: Some(&value_id),
      composition: match value.composition {
        battlement::MotionBindingComposition::Replace => wire::MotionBindingComposition::Replace,
        battlement::MotionBindingComposition::Compose => wire::MotionBindingComposition::Compose,
      },
    },
  )
}
fn write_value_subscription<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionValueSubscription,
) -> WIPOffset<wire::MotionValueSubscription<'a>> {
  let subscription_id = uuid(value.subscription_id.as_uuid());
  let value_id = uuid(value.value_id.as_uuid());
  wire::MotionValueSubscription::create(
    builder,
    &wire::MotionValueSubscriptionArgs {
      subscription_id: Some(&subscription_id),
      value_id: Some(&value_id),
      event: match value.event {
        battlement::MotionValueEventKind::Change => wire::MotionValueEventKind::Change,
        battlement::MotionValueEventKind::Velocity => wire::MotionValueEventKind::Velocity,
        battlement::MotionValueEventKind::AnimationFrame => {
          wire::MotionValueEventKind::AnimationFrame
        }
      },
    },
  )
}
fn write_gestures<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionGestureDescriptor,
) -> WIPOffset<wire::MotionGestureDescriptor<'a>> {
  let drag = value.drag.as_ref().map(|value| write_drag(builder, value));
  let subscriptions = wire::MotionGestureSubscriptions::create(
    builder,
    &wire::MotionGestureSubscriptionsArgs {
      hover: value.subscriptions.hover,
      tap: value.subscriptions.tap,
      focus: value.subscriptions.focus,
      focus_visible: value.subscriptions.focus_visible,
      pan: value.subscriptions.pan,
      pan_update: value.subscriptions.pan_update,
      drag: value.subscriptions.drag,
      drag_update: value.subscriptions.drag_update,
      momentum_complete: value.subscriptions.momentum_complete,
      constraints_measured: value.subscriptions.constraints_measured,
      scroll: value.subscriptions.scroll,
      in_view: value.subscriptions.in_view,
    },
  );
  let scroll_x_value = value.scroll_x_value.map(|value| uuid(value.as_uuid()));
  let scroll_y_value = value.scroll_y_value.map(|value| uuid(value.as_uuid()));
  let in_view_value = value.in_view_value.map(|value| uuid(value.as_uuid()));
  wire::MotionGestureDescriptor::create(
    builder,
    &wire::MotionGestureDescriptorArgs {
      pan_threshold: value.pan_threshold,
      direction_lock_threshold: value.direction_lock_threshold,
      pointer_tap_slop: value.pointer_tap_slop,
      touch_tap_slop: value.touch_tap_slop,
      pan: value.pan,
      drag,
      in_view: value.in_view,
      scroll: value.scroll,
      scroll_x_value: scroll_x_value.as_ref(),
      scroll_y_value: scroll_y_value.as_ref(),
      in_view_value: in_view_value.as_ref(),
      subscriptions: Some(subscriptions),
    },
  )
}

fn write_value_source<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &MotionValueSource,
) -> Result<WIPOffset<wire::MotionValueSource<'a>>, ProtocolError> {
  let mut clock = None;
  let mut source = None;
  let mut input = None;
  let mut output = None;
  let mut clamp = false;
  let mut spring = None;
  let mut expression = None;
  let mut inputs = None;
  let kind = match value {
    MotionValueSource::Mutable => wire::MotionValueSourceKind::Mutable,
    MotionValueSource::Time(value) => {
      clock = Some(write_clock(builder, *value));
      wire::MotionValueSourceKind::Time
    }
    MotionValueSource::Velocity { source: value } => {
      source = Some(uuid(value.as_uuid()));
      wire::MotionValueSourceKind::Velocity
    }
    MotionValueSource::Range {
      source: value_source,
      input: input_values,
      output: output_values,
      clamp: value_clamp,
    } => {
      source = Some(uuid(value_source.as_uuid()));
      let input_values = input_values
        .iter()
        .map(|value| write_value_entry(builder, value))
        .collect::<Result<Vec<_>, _>>()?;
      let output_values = output_values
        .iter()
        .map(|value| write_value_entry(builder, value))
        .collect::<Result<Vec<_>, _>>()?;
      input = Some(builder.create_vector(&input_values));
      output = Some(builder.create_vector(&output_values));
      clamp = *value_clamp;
      wire::MotionValueSourceKind::Range
    }
    MotionValueSource::Spring {
      source: value_source,
      configuration,
    } => {
      source = Some(uuid(value_source.as_uuid()));
      spring = Some(write_spring_configuration(builder, *configuration));
      wire::MotionValueSourceKind::Spring
    }
    MotionValueSource::Expression {
      operation,
      inputs: value_inputs,
    } => {
      expression = Some(write_expression(builder, *operation));
      let value_inputs = value_inputs
        .iter()
        .map(|value| uuid(value.as_uuid()))
        .collect::<Vec<_>>();
      inputs = Some(builder.create_vector(&value_inputs));
      wire::MotionValueSourceKind::Expression
    }
  };
  Ok(wire::MotionValueSource::create(
    builder,
    &wire::MotionValueSourceArgs {
      kind,
      clock,
      source: source.as_ref(),
      input,
      output,
      clamp,
      spring,
      expression,
      inputs,
    },
  ))
}

fn write_value_entry<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionValue,
) -> Result<WIPOffset<motion_wire::MotionValueEntry<'a>>, ProtocolError> {
  let value = response_motion::write_value(builder, value)?;
  Ok(motion_wire::MotionValueEntry::create(
    builder,
    &motion_wire::MotionValueEntryArgs {
      value_type: value.kind,
      value: Some(value.value),
    },
  ))
}

fn write_spring_configuration<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: SpringConfiguration,
) -> WIPOffset<wire::MotionSpringConfiguration<'a>> {
  let args = match value {
    SpringConfiguration::Physical {
      stiffness,
      damping,
      mass,
      initial_velocity,
      rest_speed,
      rest_delta,
    } => wire::MotionSpringConfigurationArgs {
      generator: motion_wire::TransitionGeneratorKind::SpringPhysical,
      stiffness,
      damping,
      mass,
      initial_velocity,
      rest_speed,
      rest_delta,
      ..Default::default()
    },
    SpringConfiguration::Duration {
      duration_micros,
      bounce,
      mass,
    } => wire::MotionSpringConfigurationArgs {
      generator: motion_wire::TransitionGeneratorKind::SpringDuration,
      duration_micros,
      bounce,
      mass,
      ..Default::default()
    },
    SpringConfiguration::VisualDuration {
      duration_micros,
      bounce,
      mass,
    } => wire::MotionSpringConfigurationArgs {
      generator: motion_wire::TransitionGeneratorKind::SpringVisualDuration,
      duration_micros,
      bounce,
      mass,
      ..Default::default()
    },
  };
  wire::MotionSpringConfiguration::create(builder, &args)
}

fn write_expression<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: MotionExpressionOperation,
) -> WIPOffset<wire::MotionExpressionOperation<'a>> {
  let (kind, scalar, minimum, maximum) = match value {
    MotionExpressionOperation::Add => (wire::MotionExpressionOperationKind::Add, 0.0, 0.0, 0.0),
    MotionExpressionOperation::Subtract => {
      (wire::MotionExpressionOperationKind::Subtract, 0.0, 0.0, 0.0)
    }
    MotionExpressionOperation::Multiply => {
      (wire::MotionExpressionOperationKind::Multiply, 0.0, 0.0, 0.0)
    }
    MotionExpressionOperation::Divide => {
      (wire::MotionExpressionOperationKind::Divide, 0.0, 0.0, 0.0)
    }
    MotionExpressionOperation::Power(value) => {
      (wire::MotionExpressionOperationKind::Power, value, 0.0, 0.0)
    }
    MotionExpressionOperation::SquareRoot => (
      wire::MotionExpressionOperationKind::SquareRoot,
      0.0,
      0.0,
      0.0,
    ),
    MotionExpressionOperation::Absolute => {
      (wire::MotionExpressionOperationKind::Absolute, 0.0, 0.0, 0.0)
    }
    MotionExpressionOperation::Minimum => {
      (wire::MotionExpressionOperationKind::Minimum, 0.0, 0.0, 0.0)
    }
    MotionExpressionOperation::Maximum => {
      (wire::MotionExpressionOperationKind::Maximum, 0.0, 0.0, 0.0)
    }
    MotionExpressionOperation::Clamp { min, max } => {
      (wire::MotionExpressionOperationKind::Clamp, 0.0, min, max)
    }
    MotionExpressionOperation::Modulo(value) => {
      (wire::MotionExpressionOperationKind::Modulo, value, 0.0, 0.0)
    }
    MotionExpressionOperation::Wrap { min, max } => {
      (wire::MotionExpressionOperationKind::Wrap, 0.0, min, max)
    }
    MotionExpressionOperation::ExponentialDecay { rate } => (
      wire::MotionExpressionOperationKind::ExponentialDecay,
      rate,
      0.0,
      0.0,
    ),
    MotionExpressionOperation::Mix => (wire::MotionExpressionOperationKind::Mix, 0.0, 0.0, 0.0),
  };
  wire::MotionExpressionOperation::create(
    builder,
    &wire::MotionExpressionOperationArgs {
      kind,
      value: scalar,
      minimum,
      maximum,
    },
  )
}

fn write_drag<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::MotionDragDescriptor,
) -> WIPOffset<wire::MotionDragDescriptor<'a>> {
  let constraints = value
    .constraints
    .map(|value| write_drag_constraint(builder, value));
  let elastic = wire::MotionDragElastic::create(
    builder,
    &wire::MotionDragElasticArgs {
      left: value.elastic.left,
      right: value.elastic.right,
      top: value.elastic.top,
      bottom: value.elastic.bottom,
    },
  );
  let transition = wire::MotionDragTransition::create(
    builder,
    &wire::MotionDragTransitionArgs {
      velocity_retention: value.transition.velocity_retention,
      rest_speed: value.transition.rest_speed,
      bounce_stiffness: value.transition.bounce_stiffness,
      bounce_damping: value.transition.bounce_damping,
    },
  );
  let control_id = value.control_id.map(|value| uuid(value.as_uuid()));
  let x_value = value.x_value.map(|value| uuid(value.as_uuid()));
  let y_value = value.y_value.map(|value| uuid(value.as_uuid()));
  wire::MotionDragDescriptor::create(
    builder,
    &wire::MotionDragDescriptorArgs {
      axis: gesture_axis(value.axis),
      constraints,
      elastic: Some(elastic),
      momentum: value.momentum,
      direction_lock: value.direction_lock,
      listener: value.listener,
      has_snap_to_origin: value.snap_to_origin.is_some(),
      snap_to_origin: value
        .snap_to_origin
        .map_or(motion_wire::MotionGestureAxis::X, gesture_axis),
      control_id: control_id.as_ref(),
      propagation: value.propagation,
      transition: Some(transition),
      x_value: x_value.as_ref(),
      y_value: y_value.as_ref(),
    },
  )
}

fn write_drag_constraint<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: battlement::MotionDragConstraint,
) -> WIPOffset<wire::MotionDragConstraint<'a>> {
  let (kind, bounds, element_id) = match value {
    battlement::MotionDragConstraint::Bounds(value) => {
      let bounds = wire::MotionDragBounds::create(
        builder,
        &wire::MotionDragBoundsArgs {
          min_x: value.min_x,
          max_x: value.max_x,
          min_y: value.min_y,
          max_y: value.max_y,
        },
      );
      (wire::MotionDragConstraintKind::Bounds, Some(bounds), None)
    }
    battlement::MotionDragConstraint::Element(value) => (
      wire::MotionDragConstraintKind::Element,
      None,
      Some(uuid(value.as_uuid())),
    ),
  };
  wire::MotionDragConstraint::create(
    builder,
    &wire::MotionDragConstraintArgs {
      kind,
      bounds,
      element_id: element_id.as_ref(),
    },
  )
}

fn gesture_axis(value: MotionGestureAxis) -> motion_wire::MotionGestureAxis {
  match value {
    MotionGestureAxis::X => motion_wire::MotionGestureAxis::X,
    MotionGestureAxis::Y => motion_wire::MotionGestureAxis::Y,
    MotionGestureAxis::Both => motion_wire::MotionGestureAxis::Both,
  }
}
