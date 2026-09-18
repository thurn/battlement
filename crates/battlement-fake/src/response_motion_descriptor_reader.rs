//! Verified retained decoding for UI motion descriptors.

use battlement::Prop;
use battlement_flatbuffers::schema_generated::{
  common_generated, motion_generated as motion_wire, ui_generated,
};
use common_generated::battlement::flat_buffers::generated as common_wire;
use ui_generated::battlement::flat_buffers::generated as wire;

use crate::{response_motion_reader as motion, response_reader::object_id};

pub(crate) fn motion(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::MotionDescriptor>, String> {
  Ok(match value.state() {
    wire::PropState::Unset => Prop::Unset,
    wire::PropState::Reset => Prop::Reset,
    wire::PropState::Set => Prop::Set(descriptor(
      value
        .value_as_motion_descriptor_property_value()
        .ok_or_else(|| "UI motion property payload is missing".to_owned())?
        .value(),
    )?),
    _ => return Err("UI motion property state is unknown".to_owned()),
  })
}

pub(crate) fn descriptor(
  value: wire::MotionDescriptor<'_>,
) -> Result<battlement::MotionDescriptor, String> {
  Ok(battlement::MotionDescriptor {
    descriptor_id: object_id(value.descriptor_id())?,
    host_id: object_id(value.host_id())?,
    generation: battlement::MotionGeneration(value.generation()),
    initial: value.initial().map(motion::target).transpose()?,
    initial_disabled: value.initial_disabled(),
    slots: value.slots().iter().map(slot).collect::<Result<_, _>>()?,
    clock: clock(value.clock())?,
    reduced_motion: reduced_motion(value.reduced_motion())?,
    pseudo_styles: value
      .pseudo_styles()
      .iter()
      .map(pseudo_style)
      .collect::<Result<_, _>>()?,
    style_transition: style_transition(value.style_transition())?,
    animations: value
      .animations()
      .iter()
      .map(animation)
      .collect::<Result<_, _>>()?,
    decorations: value
      .decorations()
      .iter()
      .map(decoration)
      .collect::<Result<_, _>>()?,
    variants: value.variants().map(variants).transpose()?,
    values: value
      .values()
      .iter()
      .map(value_descriptor)
      .collect::<Result<_, _>>()?,
    value_bindings: value
      .value_bindings()
      .iter()
      .map(value_binding)
      .collect::<Result<_, _>>()?,
    value_subscriptions: value
      .value_subscriptions()
      .iter()
      .map(value_subscription)
      .collect::<Result<_, _>>()?,
    control_id: value.control_id().map(object_id).transpose()?,
    scope_id: value.scope_id().map(object_id).transpose()?,
    scope_root: value.scope_root(),
    motion_name: value.motion_name().map(str::to_owned),
    named_targets: value
      .named_targets()
      .iter()
      .map(|value| {
        Ok(battlement::MotionNamedTarget {
          name: value.name().to_owned(),
          target: motion::target(value.target())?,
        })
      })
      .collect::<Result<_, String>>()?,
    gestures: value.gestures().map(gestures).transpose()?,
    layout: value.layout().map(layout).transpose()?,
  })
}

fn slot(value: wire::MotionSlotDescriptor<'_>) -> Result<battlement::MotionSlotDescriptor, String> {
  let callbacks = value.callbacks();
  Ok(battlement::MotionSlotDescriptor {
    slot: battlement::MotionSlotId(value.slot()),
    generation: battlement::MotionGeneration(value.generation()),
    layer: layer(value.layer())?,
    target: motion::target(value.target())?,
    callbacks: battlement::MotionCallbackSubscriptions {
      start: callbacks.start(),
      update: callbacks.update(),
      repeat: callbacks.repeat(),
      complete: callbacks.complete(),
      stop: callbacks.stop(),
      cancel: callbacks.cancel(),
    },
  })
}

fn clock(value: wire::MotionClockSource<'_>) -> Result<battlement::MotionClockSource, String> {
  Ok(match value.kind() {
    wire::MotionClockKind::Unscaled => battlement::MotionClockSource::Unscaled,
    wire::MotionClockKind::Scaled => battlement::MotionClockSource::Scaled,
    wire::MotionClockKind::Controlled => battlement::MotionClockSource::Controlled(object_id(
      value
        .object_id()
        .ok_or_else(|| "controlled motion clock identity is missing".to_owned())?,
    )?),
    wire::MotionClockKind::Audio => battlement::MotionClockSource::Audio(object_id(
      value
        .object_id()
        .ok_or_else(|| "audio motion clock identity is missing".to_owned())?,
    )?),
    _ => return Err("motion clock kind is unknown".to_owned()),
  })
}

fn pseudo_style(
  value: wire::MotionPseudoStyle<'_>,
) -> Result<battlement::MotionPseudoStyle, String> {
  Ok(battlement::MotionPseudoStyle {
    state: match value.state() {
      wire::MotionPseudoState::Hover => battlement::MotionPseudoState::Hover,
      wire::MotionPseudoState::Focus => battlement::MotionPseudoState::Focus,
      wire::MotionPseudoState::Active => battlement::MotionPseudoState::Active,
      wire::MotionPseudoState::Disabled => battlement::MotionPseudoState::Disabled,
      _ => return Err("motion pseudo state is unknown".to_owned()),
    },
    values: value
      .values()
      .iter()
      .map(property_value)
      .collect::<Result<_, _>>()?,
  })
}

fn property_value(
  value: motion_wire::MotionPropertyValue<'_>,
) -> Result<battlement::MotionPropertyValue, String> {
  Ok(battlement::MotionPropertyValue {
    property: motion::property(value.property())?,
    value: motion::motion_value(value.value_type(), value)?,
  })
}

fn style_transition(
  value: wire::StyleTransitionDescriptor<'_>,
) -> Result<battlement::StyleTransitionDescriptor, String> {
  Ok(battlement::StyleTransitionDescriptor {
    properties: value
      .properties()
      .iter()
      .map(|value| {
        Ok(battlement::StylePropertyTransition {
          property: motion::property(value.property())?,
          transition: motion::transition(value.transition())?,
        })
      })
      .collect::<Result<_, String>>()?,
    all: value.all().map(motion::transition).transpose()?,
    allow_discrete: value.allow_discrete(),
  })
}

fn animation(
  value: wire::CssAnimationDescriptor<'_>,
) -> Result<battlement::CssAnimationDescriptor, String> {
  Ok(battlement::CssAnimationDescriptor {
    slot: value.slot(),
    generation: value.generation(),
    restart_key: value.restart_key(),
    tracks: value
      .tracks()
      .iter()
      .map(|track| {
        Ok(battlement::CssPropertyTrack {
          property: motion::property(track.property())?,
          values: track
            .values()
            .iter()
            .map(|value| motion::motion_value(value.value_type(), value))
            .collect::<Result<_, _>>()?,
          times: track.times().iter().collect(),
          transition: motion::transition(track.transition())?,
        })
      })
      .collect::<Result<_, String>>()?,
    direction: match value.direction() {
      wire::AnimationDirection::Normal => battlement::AnimationDirection::Normal,
      wire::AnimationDirection::Reverse => battlement::AnimationDirection::Reverse,
      wire::AnimationDirection::Alternate => battlement::AnimationDirection::Alternate,
      wire::AnimationDirection::AlternateReverse => {
        battlement::AnimationDirection::AlternateReverse
      }
      _ => return Err("animation direction is unknown".to_owned()),
    },
    fill: match value.fill() {
      wire::AnimationFill::None => battlement::AnimationFill::None,
      wire::AnimationFill::Forwards => battlement::AnimationFill::Forwards,
      wire::AnimationFill::Backwards => battlement::AnimationFill::Backwards,
      wire::AnimationFill::Both => battlement::AnimationFill::Both,
      _ => return Err("animation fill is unknown".to_owned()),
    },
    play_state: match value.play_state() {
      wire::AnimationPlayState::Running => battlement::AnimationPlayState::Running,
      wire::AnimationPlayState::Paused => battlement::AnimationPlayState::Paused,
      _ => return Err("animation play state is unknown".to_owned()),
    },
    composition: match value.composition() {
      wire::AnimationComposition::Replace => battlement::AnimationComposition::Replace,
      wire::AnimationComposition::Add => battlement::AnimationComposition::Add,
      wire::AnimationComposition::Accumulate => battlement::AnimationComposition::Accumulate,
      _ => return Err("animation composition is unknown".to_owned()),
    },
    diagnostic_name: value.diagnostic_name().map(str::to_owned),
  })
}

fn decoration(
  value: wire::MotionDecorationDescriptor<'_>,
) -> Result<battlement::MotionDecorationDescriptor, String> {
  Ok(battlement::MotionDecorationDescriptor {
    key: value.key(),
    placement: match value.placement() {
      wire::DecorationPlacement::Before => battlement::DecorationPlacement::Before,
      wire::DecorationPlacement::After => battlement::DecorationPlacement::After,
      _ => return Err("motion decoration placement is unknown".to_owned()),
    },
    position: match value.position() {
      wire::DecorationPosition::Fill => battlement::DecorationPosition::Fill,
      wire::DecorationPosition::Border => battlement::DecorationPosition::Border,
      _ => return Err("motion decoration position is unknown".to_owned()),
    },
    overflow: match value.overflow() {
      wire::DecorationOverflow::Hidden => battlement::DecorationOverflow::Hidden,
      wire::DecorationOverflow::Visible => battlement::DecorationOverflow::Visible,
      _ => return Err("motion decoration overflow is unknown".to_owned()),
    },
    style: crate::response_ui_reader::read_style(value.style().iter())?,
    animations: value
      .animations()
      .iter()
      .map(animation)
      .collect::<Result<_, _>>()?,
  })
}

fn variants(
  value: wire::MotionVariantResolution<'_>,
) -> Result<battlement::MotionVariantResolution, String> {
  Ok(battlement::MotionVariantResolution {
    names: value.names().iter().map(str::to_owned).collect(),
    inherited: value.inherited(),
    custom_snapshot: value.custom_snapshot(),
    child_index: value.child_index(),
    delay_micros: value.delay_micros(),
    when: match value.when() {
      wire::VariantWhen::Together => battlement::VariantWhen::Together,
      wire::VariantWhen::BeforeChildren => battlement::VariantWhen::BeforeChildren,
      wire::VariantWhen::AfterChildren => battlement::VariantWhen::AfterChildren,
      _ => return Err("motion variant sequencing is unknown".to_owned()),
    },
    stagger_direction: match value.stagger_direction() {
      wire::StaggerDirection::Forward => battlement::StaggerDirection::Forward,
      wire::StaggerDirection::Reverse => battlement::StaggerDirection::Reverse,
      _ => return Err("motion stagger direction is unknown".to_owned()),
    },
  })
}

fn value_descriptor(
  value: wire::MotionValueDescriptor<'_>,
) -> Result<battlement::MotionValueDescriptor, String> {
  Ok(battlement::MotionValueDescriptor {
    value_id: object_id(value.value_id())?,
    initial: motion::motion_value(value.initial_type(), InitialValue(value))?,
    source: value_source(value.source())?,
  })
}

#[derive(Clone, Copy)]
struct InitialValue<'a>(wire::MotionValueDescriptor<'a>);

impl<'a> motion::MotionValueSource<'a> for InitialValue<'a> {
  fn scalar(self) -> Option<motion_wire::ScalarMotionValue<'a>> {
    self.0.initial_as_scalar_motion_value()
  }
  fn length(self) -> Option<motion_wire::LengthMotionValue<'a>> {
    self.0.initial_as_length_motion_value()
  }
  fn color(self) -> Option<motion_wire::ColorMotionValue<'a>> {
    self.0.initial_as_color_motion_value()
  }
  fn vector2(self) -> Option<motion_wire::Vector2MotionValue<'a>> {
    self.0.initial_as_vector_2_motion_value()
  }
  fn vector3(self) -> Option<motion_wire::Vector3MotionValue<'a>> {
    self.0.initial_as_vector_3_motion_value()
  }
  fn angle(self) -> Option<motion_wire::AngleMotionValue<'a>> {
    self.0.initial_as_angle_motion_value()
  }
  fn transforms(self) -> Option<motion_wire::TransformListMotionValue<'a>> {
    self.0.initial_as_transform_list_motion_value()
  }
  fn filters(self) -> Option<motion_wire::FilterListMotionValue<'a>> {
    self.0.initial_as_filter_list_motion_value()
  }
  fn shadows(self) -> Option<motion_wire::ShadowListMotionValue<'a>> {
    self.0.initial_as_shadow_list_motion_value()
  }
  fn gradient(self) -> Option<motion_wire::GradientMotionValue<'a>> {
    self.0.initial_as_gradient_motion_value()
  }
  fn inset(self) -> Option<motion_wire::ClipInsetMotionValue<'a>> {
    self.0.initial_as_clip_inset_motion_value()
  }
  fn polygon(self) -> Option<motion_wire::ClipPolygonMotionValue<'a>> {
    self.0.initial_as_clip_polygon_motion_value()
  }
  fn discrete(self) -> Option<motion_wire::DiscreteMotionValue<'a>> {
    self.0.initial_as_discrete_motion_value()
  }
}

fn value_source(
  value: wire::MotionValueSource<'_>,
) -> Result<battlement::MotionValueSource, String> {
  let source_id = || {
    object_id(
      value
        .source()
        .ok_or_else(|| "motion value source identity is missing".to_owned())?,
    )
  };
  Ok(match value.kind() {
    wire::MotionValueSourceKind::Mutable => battlement::MotionValueSource::Mutable,
    wire::MotionValueSourceKind::Time => battlement::MotionValueSource::Time(clock(
      value
        .clock()
        .ok_or_else(|| "motion value clock is missing".to_owned())?,
    )?),
    wire::MotionValueSourceKind::Velocity => battlement::MotionValueSource::Velocity {
      source: source_id()?,
    },
    wire::MotionValueSourceKind::Range => battlement::MotionValueSource::Range {
      source: source_id()?,
      input: value
        .input()
        .ok_or_else(|| "motion range input is missing".to_owned())?
        .iter()
        .map(|value| motion::motion_value(value.value_type(), value))
        .collect::<Result<_, _>>()?,
      output: value
        .output()
        .ok_or_else(|| "motion range output is missing".to_owned())?
        .iter()
        .map(|value| motion::motion_value(value.value_type(), value))
        .collect::<Result<_, _>>()?,
      clamp: value.clamp(),
    },
    wire::MotionValueSourceKind::Spring => battlement::MotionValueSource::Spring {
      source: source_id()?,
      configuration: spring(
        value
          .spring()
          .ok_or_else(|| "motion graph spring is missing".to_owned())?,
      )?,
    },
    wire::MotionValueSourceKind::Expression => battlement::MotionValueSource::Expression {
      operation: expression(
        value
          .expression()
          .ok_or_else(|| "motion graph expression is missing".to_owned())?,
      )?,
      inputs: value
        .inputs()
        .ok_or_else(|| "motion expression inputs are missing".to_owned())?
        .iter()
        .map(object_id)
        .collect::<Result<_, _>>()?,
    },
    _ => return Err("motion value source kind is unknown".to_owned()),
  })
}

fn spring(
  value: wire::MotionSpringConfiguration<'_>,
) -> Result<battlement::SpringConfiguration, String> {
  Ok(match value.generator() {
    motion_wire::TransitionGeneratorKind::SpringPhysical => {
      battlement::SpringConfiguration::Physical {
        stiffness: value.stiffness(),
        damping: value.damping(),
        mass: value.mass(),
        initial_velocity: value.initial_velocity(),
        rest_speed: value.rest_speed(),
        rest_delta: value.rest_delta(),
      }
    }
    motion_wire::TransitionGeneratorKind::SpringDuration => {
      battlement::SpringConfiguration::Duration {
        duration_micros: value.duration_micros(),
        bounce: value.bounce(),
        mass: value.mass(),
      }
    }
    motion_wire::TransitionGeneratorKind::SpringVisualDuration => {
      battlement::SpringConfiguration::VisualDuration {
        duration_micros: value.duration_micros(),
        bounce: value.bounce(),
        mass: value.mass(),
      }
    }
    _ => return Err("motion graph spring kind is unknown".to_owned()),
  })
}

fn expression(
  value: wire::MotionExpressionOperation<'_>,
) -> Result<battlement::MotionExpressionOperation, String> {
  Ok(match value.kind() {
    wire::MotionExpressionOperationKind::Add => battlement::MotionExpressionOperation::Add,
    wire::MotionExpressionOperationKind::Subtract => {
      battlement::MotionExpressionOperation::Subtract
    }
    wire::MotionExpressionOperationKind::Multiply => {
      battlement::MotionExpressionOperation::Multiply
    }
    wire::MotionExpressionOperationKind::Divide => battlement::MotionExpressionOperation::Divide,
    wire::MotionExpressionOperationKind::Power => {
      battlement::MotionExpressionOperation::Power(value.value())
    }
    wire::MotionExpressionOperationKind::SquareRoot => {
      battlement::MotionExpressionOperation::SquareRoot
    }
    wire::MotionExpressionOperationKind::Absolute => {
      battlement::MotionExpressionOperation::Absolute
    }
    wire::MotionExpressionOperationKind::Minimum => battlement::MotionExpressionOperation::Minimum,
    wire::MotionExpressionOperationKind::Maximum => battlement::MotionExpressionOperation::Maximum,
    wire::MotionExpressionOperationKind::Clamp => battlement::MotionExpressionOperation::Clamp {
      min: value.minimum(),
      max: value.maximum(),
    },
    wire::MotionExpressionOperationKind::Modulo => {
      battlement::MotionExpressionOperation::Modulo(value.value())
    }
    wire::MotionExpressionOperationKind::Wrap => battlement::MotionExpressionOperation::Wrap {
      min: value.minimum(),
      max: value.maximum(),
    },
    wire::MotionExpressionOperationKind::ExponentialDecay => {
      battlement::MotionExpressionOperation::ExponentialDecay {
        rate: value.value(),
      }
    }
    wire::MotionExpressionOperationKind::Mix => battlement::MotionExpressionOperation::Mix,
    _ => return Err("motion expression operation is unknown".to_owned()),
  })
}

fn value_binding(
  value: wire::MotionValueBinding<'_>,
) -> Result<battlement::MotionValueBinding, String> {
  Ok(battlement::MotionValueBinding {
    property: motion::property(value.property())?,
    value_id: object_id(value.value_id())?,
    composition: match value.composition() {
      wire::MotionBindingComposition::Replace => battlement::MotionBindingComposition::Replace,
      wire::MotionBindingComposition::Compose => battlement::MotionBindingComposition::Compose,
      _ => return Err("motion value composition is unknown".to_owned()),
    },
  })
}

fn value_subscription(
  value: wire::MotionValueSubscription<'_>,
) -> Result<battlement::MotionValueSubscription, String> {
  Ok(battlement::MotionValueSubscription {
    subscription_id: object_id(value.subscription_id())?,
    value_id: object_id(value.value_id())?,
    event: match value.event() {
      wire::MotionValueEventKind::Change => battlement::MotionValueEventKind::Change,
      wire::MotionValueEventKind::Velocity => battlement::MotionValueEventKind::Velocity,
      wire::MotionValueEventKind::AnimationFrame => {
        battlement::MotionValueEventKind::AnimationFrame
      }
      _ => return Err("motion value event kind is unknown".to_owned()),
    },
  })
}

fn gestures(
  value: wire::MotionGestureDescriptor<'_>,
) -> Result<battlement::MotionGestureDescriptor, String> {
  let subscriptions = value.subscriptions();
  Ok(battlement::MotionGestureDescriptor {
    pan_threshold: value.pan_threshold(),
    direction_lock_threshold: value.direction_lock_threshold(),
    pointer_tap_slop: value.pointer_tap_slop(),
    touch_tap_slop: value.touch_tap_slop(),
    pan: value.pan(),
    drag: value.drag().map(drag).transpose()?,
    in_view: value.in_view(),
    scroll: value.scroll(),
    scroll_x_value: value.scroll_x_value().map(object_id).transpose()?,
    scroll_y_value: value.scroll_y_value().map(object_id).transpose()?,
    in_view_value: value.in_view_value().map(object_id).transpose()?,
    subscriptions: battlement::MotionGestureSubscriptions {
      hover: subscriptions.hover(),
      tap: subscriptions.tap(),
      focus: subscriptions.focus(),
      focus_visible: subscriptions.focus_visible(),
      pan: subscriptions.pan(),
      pan_update: subscriptions.pan_update(),
      drag: subscriptions.drag(),
      drag_update: subscriptions.drag_update(),
      momentum_complete: subscriptions.momentum_complete(),
      constraints_measured: subscriptions.constraints_measured(),
      scroll: subscriptions.scroll(),
      in_view: subscriptions.in_view(),
    },
  })
}

fn drag(value: wire::MotionDragDescriptor<'_>) -> Result<battlement::MotionDragDescriptor, String> {
  let elastic = value.elastic();
  let transition = value.transition();
  Ok(battlement::MotionDragDescriptor {
    axis: axis(value.axis())?,
    constraints: value.constraints().map(constraint).transpose()?,
    elastic: battlement::MotionDragElastic {
      left: elastic.left(),
      right: elastic.right(),
      top: elastic.top(),
      bottom: elastic.bottom(),
    },
    momentum: value.momentum(),
    direction_lock: value.direction_lock(),
    listener: value.listener(),
    snap_to_origin: value
      .has_snap_to_origin()
      .then(|| axis(value.snap_to_origin()))
      .transpose()?,
    control_id: value.control_id().map(object_id).transpose()?,
    propagation: value.propagation(),
    transition: battlement::MotionDragTransition {
      velocity_retention: transition.velocity_retention(),
      rest_speed: transition.rest_speed(),
      bounce_stiffness: transition.bounce_stiffness(),
      bounce_damping: transition.bounce_damping(),
    },
    x_value: value.x_value().map(object_id).transpose()?,
    y_value: value.y_value().map(object_id).transpose()?,
  })
}

fn constraint(
  value: wire::MotionDragConstraint<'_>,
) -> Result<battlement::MotionDragConstraint, String> {
  Ok(match value.kind() {
    wire::MotionDragConstraintKind::Bounds => {
      let value = value
        .bounds()
        .ok_or_else(|| "motion drag bounds are missing".to_owned())?;
      battlement::MotionDragConstraint::Bounds(battlement::MotionDragBounds {
        min_x: value.min_x(),
        max_x: value.max_x(),
        min_y: value.min_y(),
        max_y: value.max_y(),
      })
    }
    wire::MotionDragConstraintKind::Element => {
      battlement::MotionDragConstraint::Element(object_id(
        value
          .element_id()
          .ok_or_else(|| "motion drag element is missing".to_owned())?,
      )?)
    }
    _ => return Err("motion drag constraint kind is unknown".to_owned()),
  })
}

fn layout(
  value: wire::MotionLayoutDescriptor<'_>,
) -> Result<battlement::MotionLayoutDescriptor, String> {
  Ok(battlement::MotionLayoutDescriptor {
    mode: match value.mode() {
      wire::MotionLayoutMode::Position => battlement::MotionLayoutMode::Position,
      wire::MotionLayoutMode::Size => battlement::MotionLayoutMode::Size,
      wire::MotionLayoutMode::Both => battlement::MotionLayoutMode::Both,
      _ => return Err("motion layout mode is unknown".to_owned()),
    },
    group: layout_identity(value.group()),
    layout_id: value.layout_id().map(layout_identity),
    scroll: value.scroll(),
    root: value.root(),
    pop_layout: value.pop_layout(),
    transition: motion::transition(value.transition())?,
    projection: value.projection().map(projection).transpose()?,
  })
}

fn projection(
  value: wire::MotionProjectionDescriptor<'_>,
) -> Result<battlement::MotionProjectionDescriptor, String> {
  let vector = |value: &common_wire::Vector3d| battlement::Vector3 {
    x: value.x(),
    y: value.y(),
    z: value.z(),
  };
  let plane = value.plane();
  let world_rect = value.world_rect();
  Ok(battlement::MotionProjectionDescriptor {
    camera: match value.camera_kind() {
      wire::MotionProjectionCameraKind::Input => battlement::MotionProjectionCamera::Input,
      wire::MotionProjectionCameraKind::Object => {
        battlement::MotionProjectionCamera::Object(object_id(
          value
            .camera_object_id()
            .ok_or_else(|| "motion projection camera object is missing".to_owned())?,
        )?)
      }
      _ => return Err("motion projection camera kind is unknown".to_owned()),
    },
    plane: battlement::MotionProjectionPlane {
      origin: vector(plane.origin()),
      x_axis: vector(plane.x_axis()),
      y_axis: vector(plane.y_axis()),
    },
    world_rect: battlement::Rect::new(
      world_rect.x(),
      world_rect.y(),
      world_rect.width(),
      world_rect.height(),
    ),
  })
}

fn layout_identity(value: wire::MotionLayoutIdentity<'_>) -> battlement::MotionLayoutIdentity {
  battlement::MotionLayoutIdentity {
    value_type: value.value_type().to_owned(),
    value_hash: value.value_hash(),
  }
}

fn layer(value: wire::MotionLayer) -> Result<battlement::MotionLayer, String> {
  Ok(match value {
    wire::MotionLayer::Animate => battlement::MotionLayer::Animate,
    wire::MotionLayer::InView => battlement::MotionLayer::InView,
    wire::MotionLayer::Focus => battlement::MotionLayer::Focus,
    wire::MotionLayer::FocusVisible => battlement::MotionLayer::FocusVisible,
    wire::MotionLayer::Hover => battlement::MotionLayer::Hover,
    wire::MotionLayer::Tap => battlement::MotionLayer::Tap,
    wire::MotionLayer::Drag => battlement::MotionLayer::Drag,
    wire::MotionLayer::Exit => battlement::MotionLayer::Exit,
    _ => return Err("motion layer is unknown".to_owned()),
  })
}

fn reduced_motion(
  value: wire::ReducedMotionPolicy,
) -> Result<battlement::ReducedMotionPolicy, String> {
  Ok(match value {
    wire::ReducedMotionPolicy::User => battlement::ReducedMotionPolicy::User,
    wire::ReducedMotionPolicy::Always => battlement::ReducedMotionPolicy::Always,
    wire::ReducedMotionPolicy::Never => battlement::ReducedMotionPolicy::Never,
    _ => return Err("reduced-motion policy is unknown".to_owned()),
  })
}

fn axis(value: motion_wire::MotionGestureAxis) -> Result<battlement::MotionGestureAxis, String> {
  Ok(match value {
    motion_wire::MotionGestureAxis::X => battlement::MotionGestureAxis::X,
    motion_wire::MotionGestureAxis::Y => battlement::MotionGestureAxis::Y,
    motion_wire::MotionGestureAxis::Both => battlement::MotionGestureAxis::Both,
    _ => return Err("motion gesture axis is unknown".to_owned()),
  })
}
