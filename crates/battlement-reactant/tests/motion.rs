use std::panic::{self, AssertUnwindSafe};
use std::time::Duration;

use battlement::{
  AudioClipAddress, CameraState, CommandBody, GameObject, GameObjectKind,
  InertiaTarget as LoweredInertiaTarget, MotionDragConstraint, MotionEasing, MotionEventBatch,
  MotionGeneration, MotionGestureAxis, MotionGestureEvent, MotionGestureEventKind,
  MotionGestureVector, MotionPointerDevice, MotionProperty, MotionRepeat, MotionRepeatType,
  MotionSequence, MotionValue, ObjectId, PanelScaleMode, PanelSettings, ParentScene, PreparedAsset,
  Prop, Scene, SceneId, SessionId, Snapshot, SpringConfiguration, Style, TransitionGenerator,
  UiDocument, UiDocumentState, UiVisualElementProperties,
};
use battlement_reactant::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  prelude::*,
  runtime::{Reactant, ReactantCommit},
  semantics,
};

struct IdleSpawner;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum ValueVariant {
  Rest,
  Active,
}

struct ValueContract;

struct GestureContract;

struct ElementConstraintContract;

struct LayoutContract;

struct MotionConfigContract;

struct ButtonInteractionContract;

impl Component for ButtonInteractionContract {
  fn render(&self) -> impl Render {
    let behavior = use_button_state(ButtonOptions {
      name: semantics::text("Action"),
      is_disabled: false,
      on_press: || {},
    });
    let state = behavior.state;
    Button::new(format!("focus-visible={}", state.focus_visible))
      .behavior(behavior)
      .on_focus_visible_start(|events: &mut Vec<MotionGestureEventKind>, event| {
        events.push(event.kind);
      })
  }
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl Component for ValueContract {
  fn render(&self) -> impl Render {
    let source = use_motion_value(0.25_f32);
    let derived = use_transform(
      source,
      InputRange::new([0.0, 1.0]),
      OutputRange::new([0.2, 0.9]),
    );
    let controls = use_animation_controls::<ValueVariant>();
    let scope = use_animation_scope();
    View::new()
      .animation_scope(scope)
      .child(
        View::new()
          .animation_controls(controls)
          .motion_name("controlled")
          .variants(
            Variants::<ValueVariant, ()>::new()
              .target(ValueVariant::Rest, MotionStyle::new().opacity(0.3))
              .target(ValueVariant::Active, MotionStyle::new().opacity(1.0)),
          )
          .animate_variant(ValueVariant::Rest),
      )
      .child(View::new().animate(MotionStyle::new().opacity_value(derived)))
  }
}

impl Component for GestureContract {
  fn render(&self) -> impl Render {
    let x = use_motion_value(0.0_f32);
    let y = use_motion_value(0.0_f32);
    let scroll_x = use_motion_value(0.0_f32);
    let scroll_y = use_motion_value(0.0_f32);
    let in_view = use_motion_value(0.0_f32);
    let controls = use_drag_controls();
    View::new()
      .while_hover(MotionStyle::new().scale(1.04))
      .while_tap(MotionStyle::new().scale(0.96))
      .while_focus(MotionStyle::new().opacity(1.0))
      .while_drag(MotionStyle::new().scale(1.08))
      .while_in_view(MotionStyle::new().opacity(0.9))
      .pan(true)
      .drag(DragAxis::Both)
      .drag_constraints(DragConstraints::bounds(-40.0, 80.0, -20.0, 60.0))
      .drag_elastic(DragElastic::sides(0.1, 0.2, 0.3, 0.4))
      .drag_direction_lock(true)
      .drag_propagation(true)
      .drag_snap_to_origin(DragAxis::X)
      .drag_transition(
        DragTransition::new()
          .velocity_retention(0.04)
          .rest_speed(5.0),
      )
      .drag_motion_values(x, y)
      .drag_controls(controls)
      .scroll_motion_values(scroll_x, scroll_y)
      .in_view_motion_value(in_view)
      .on_hover_start(|events: &mut Vec<MotionGestureEventKind>, event| {
        events.push(event.kind);
      })
      .on_focus_visible_start(|events: &mut Vec<MotionGestureEventKind>, event| {
        events.push(event.kind);
      })
      .on_drag_start(|events: &mut Vec<MotionGestureEventKind>, event| {
        events.push(event.kind);
      })
      .on_drag_end(|events: &mut Vec<MotionGestureEventKind>, event| {
        events.push(event.kind);
      })
      .on_scroll_motion(|events: &mut Vec<MotionGestureEventKind>, event| {
        events.push(event.kind);
      })
      .on_viewport_enter(|events: &mut Vec<MotionGestureEventKind>, event| {
        events.push(event.kind);
      })
  }
}

impl Component for ElementConstraintContract {
  fn render(&self) -> impl Render {
    let constraint = use_element_ref();
    View::new().element_ref(constraint.clone()).child(
      View::new()
        .drag(DragAxis::Both)
        .drag_constraints(DragConstraints::element(constraint)),
    )
  }
}

impl Component for LayoutContract {
  fn render(&self) -> impl Render {
    LayoutGroup::new("settings").child(
      View::new()
        .layout(Layout::Both)
        .layout_id("active")
        .layout_scroll(true)
        .layout_root(true)
        .transition(Transition::tween().duration_secs(0.4).property(
          MotionProperty::Layout,
          Transition::tween().duration_secs(0.25),
        ))
        .child(View::new().reorder_item(ReorderAxis::Y)),
    )
  }
}

impl Component for MotionConfigContract {
  fn render(&self) -> impl Render {
    MotionConfig::new(MotionConfig::new(
      View::new()
        .animate(MotionStyle::new().x(24.0).opacity(1.0))
        .transition(Transition::tween().duration_secs(0.45)),
    ))
    .transition(Transition::tween().duration_secs(0.9).property(
      MotionProperty::Opacity,
      Transition::tween().duration_secs(0.2),
    ))
    .reduced_motion(ReducedMotion::Always)
    .time_source(MotionTimeSource::Scaled)
  }
}

#[derive(Clone)]
struct ForwardingCard {
  motion: MotionProps,
}

#[derive(Clone)]
struct MissingForwardingCard {
  motion: MotionProps,
}

#[derive(Clone)]
struct MultipleForwardingCard {
  motion: MotionProps,
}

impl Component for ForwardingCard {
  fn render(&self) -> impl Render {
    View::new()
      .name("forwarded-host")
      .motion(self.motion.clone())
      .child(Label::new("content"))
      .class("after-motion")
  }
}

impl MotionComponent for ForwardingCard {
  fn with_motion(mut self, motion: MotionProps) -> Self {
    self.motion = motion;
    self
  }
}

impl Component for MissingForwardingCard {
  fn render(&self) -> impl Render {
    let _motion = self.motion.clone();
    View::new()
  }
}

impl MotionComponent for MissingForwardingCard {
  fn with_motion(mut self, motion: MotionProps) -> Self {
    self.motion = motion;
    self
  }
}

impl Component for MultipleForwardingCard {
  fn render(&self) -> impl Render {
    Fragment::new((
      View::new().motion(self.motion.clone()),
      View::new().motion(self.motion.clone()),
    ))
  }
}

impl MotionComponent for MultipleForwardingCard {
  fn with_motion(mut self, motion: MotionProps) -> Self {
    self.motion = motion;
    self
  }
}

#[test]
fn host_methods_interleave_without_restarting_or_adding_a_host() {
  let document = document();
  let mut order = false;
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |order: &bool| {
    if *order {
      View::new()
        .initial(MotionStyle::new().opacity(0.0))
        .name("probe")
        .animate(MotionStyle::new().opacity(1.0))
        .class("card")
        .style(Style::new().width(120.0))
        .transition(Transition::tween().duration_secs(1.0))
        .child(Label::new("same"))
    } else {
      View::new()
        .style(Style::new().width(120.0))
        .class("card")
        .transition(Transition::tween().duration_secs(1.0))
        .child(Label::new("same"))
        .name("probe")
        .initial(MotionStyle::new().opacity(0.0))
        .animate(MotionStyle::new().opacity(1.0))
    }
  });
  let rendered = start(&mut reactant, &mut order, &document);
  let host = &rendered.children[0];
  assert_eq!(host.children.len(), 1);
  assert!(matches!(host.element.visual_element().motion, Prop::Set(_)));

  order = true;
  assert!(reactant.refresh(&mut order).unwrap().is_empty());
  let _ = reactant.shutdown(&mut order).into_groups();
}

#[test]
fn public_targets_serialize_keyframes_overrides_repeats_and_transition_end() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |(): &()| {
    View::new()
      .initial(MotionStyle::new().opacity(0.0).x(-12.0))
      .animate(
        MotionTarget::new(
          MotionStyle::new()
            .opacity_keyframes(Keyframes::new([0.0, 0.8, 1.0]).times([0.0, 0.2, 1.0]))
            .x(24.0),
        )
        .transition_end(MotionStyle::new().opacity(0.7)),
      )
      .exit(MotionStyle::new().opacity(0.0))
      .transition(
        Transition::tween()
          .duration_secs(1.2)
          .delay_secs(-0.25)
          .ease(Easing::Linear)
          .repeat(Repeat::Count(2))
          .repeat_delay_secs(0.1)
          .repeat_type(RepeatType::Reverse)
          .property(
            MotionProperty::X,
            Transition::tween()
              .duration_secs(0.4)
              .ease(Easing::EaseOut)
              .repeat(Repeat::Count(1))
              .repeat_type(RepeatType::Mirror),
          ),
      )
  });
  let rendered = start(&mut reactant, &mut (), &document);
  let Prop::Set(descriptor) = &rendered.children[0].element.visual_element().motion else {
    panic!("public Motion props did not lower");
  };
  descriptor.validate().unwrap();
  assert_eq!(descriptor.slots.len(), 1);
  let target = &descriptor.slots[0].target;
  assert_eq!(target.tracks.len(), 2);
  assert_eq!(target.transition_end[0].value, MotionValue::Scalar(0.7));
  let opacity = target
    .tracks
    .iter()
    .find(|track| track.property == MotionProperty::Opacity)
    .unwrap();
  assert_eq!(opacity.times.as_deref(), Some(&[0.0, 0.2, 1.0][..]));
  assert_eq!(opacity.transition.repeat, MotionRepeat::Count(2));
  assert_eq!(opacity.transition.repeat_type, MotionRepeatType::Reverse);
  assert!(matches!(
    opacity.transition.generator,
    TransitionGenerator::Tween {
      duration_micros: 1_200_000,
      ref easings,
      ..
    } if easings == &[MotionEasing::Linear]
  ));
  let x = target
    .tracks
    .iter()
    .find(|track| track.property == MotionProperty::X)
    .unwrap();
  assert_eq!(x.transition.repeat_type, MotionRepeatType::Mirror);
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn unspecified_transitions_use_motion_property_and_keyframe_defaults() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |(): &()| {
    View::new().animate(
      MotionStyle::new()
        .opacity(0.7)
        .x(20.0)
        .scale(0.0)
        .background_color_keyframes(Keyframes::new([
          MotionColor::new(0.0, 0.0, 0.0, 1.0),
          MotionColor::new(0.5, 0.5, 0.5, 1.0),
          MotionColor::new(1.0, 1.0, 1.0, 1.0),
        ])),
    )
  });
  let rendered = start(&mut reactant, &mut (), &document);
  let Prop::Set(descriptor) = &rendered.children[0].element.visual_element().motion else {
    panic!("public Motion props did not lower");
  };
  let track = |property| {
    descriptor.slots[0]
      .target
      .tracks
      .iter()
      .find(|track| track.property == property)
      .unwrap()
  };
  assert!(matches!(
    track(MotionProperty::Opacity).transition.generator,
    TransitionGenerator::Tween {
      duration_micros: 300_000,
      ref easings,
      ..
    } if easings == &[MotionEasing::CubicBezier([0.25, 0.1, 0.35, 1.0])]
  ));
  assert!(matches!(
    track(MotionProperty::X).transition.generator,
    TransitionGenerator::Spring(SpringConfiguration::Physical {
      stiffness: 500.0,
      damping: 25.0,
      rest_speed: Some(10.0),
      ..
    })
  ));
  assert!(matches!(
    track(MotionProperty::Scale).transition.generator,
    TransitionGenerator::Spring(SpringConfiguration::Physical { damping, .. })
      if (damping - 2.0 * 550.0_f64.sqrt()).abs() < f64::EPSILON
  ));
  assert!(matches!(
    track(MotionProperty::BackgroundColor).transition.generator,
    TransitionGenerator::Tween {
      duration_micros: 800_000,
      ..
    }
  ));
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn public_physical_transitions_lower_every_configuration_form() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |(): &()| {
    Fragment::new((
      View::new().animate(
        MotionTarget::new(MotionStyle::new().x(120.0)).transition(
          Transition::spring()
            .stiffness(420.0)
            .damping(32.0)
            .mass(1.5)
            .initial_velocity(-24.0)
            .rest_speed(0.2)
            .rest_delta(0.1),
        ),
      ),
      View::new().animate(
        MotionTarget::new(MotionStyle::new().scale(1.2)).transition(
          Transition::spring()
            .duration_secs(0.65)
            .bounce(0.42)
            .mass(2.0),
        ),
      ),
      View::new().animate(
        MotionTarget::new(MotionStyle::new().opacity(1.0)).transition(
          Transition::inertia()
            .initial_velocity(180.0)
            .power(0.7)
            .time_constant_secs(0.24)
            .minimum(-20.0)
            .maximum(140.0)
            .rest_delta(0.25)
            .bounce_stiffness(620.0)
            .bounce_damping(18.0)
            .target(InertiaTarget::nearest_multiple(20.0)),
        ),
      ),
    ))
  });
  let rendered = start(&mut reactant, &mut (), &document);
  let generators = rendered
    .children
    .iter()
    .map(|host| {
      let Prop::Set(descriptor) = &host.element.visual_element().motion else {
        panic!("physical transition did not lower");
      };
      descriptor.validate().unwrap();
      descriptor.slots[0].target.tracks[0]
        .transition
        .generator
        .clone()
    })
    .collect::<Vec<_>>();
  assert!(matches!(
    generators[0],
    TransitionGenerator::Spring(SpringConfiguration::Physical {
      stiffness: 420.0,
      damping: 32.0,
      mass: 1.5,
      initial_velocity: Some(-24.0),
      rest_speed: Some(0.2),
      rest_delta: Some(0.1),
    })
  ));
  assert!(matches!(
    generators[1],
    TransitionGenerator::Spring(SpringConfiguration::Duration {
      duration_micros: 650_000,
      bounce: 0.42,
      mass: 2.0,
    })
  ));
  assert!(matches!(
    generators[2],
    TransitionGenerator::Inertia {
      initial_velocity: 180.0,
      power: 0.7,
      time_constant_micros: 240_000,
      minimum: Some(-20.0),
      maximum: Some(140.0),
      rest_delta: 0.25,
      bounce_stiffness: 620.0,
      bounce_damping: 18.0,
      target: LoweredInertiaTarget::NearestMultiple(20.0),
    }
  ));
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn spring_duration_and_physical_options_are_exclusive() {
  let result = panic::catch_unwind(|| Transition::spring().stiffness(300.0).duration_secs(0.5));
  assert!(result.is_err());
}

#[test]
fn forwarding_component_collects_complete_props_without_a_wrapper_host() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |(): &()| {
    ForwardingCard {
      motion: MotionProps::new(),
    }
    .animate(MotionStyle::new().opacity(1.0))
    .initial(MotionStyle::new().opacity(0.0))
    .exit(MotionStyle::new().opacity(0.0))
    .transition(Transition::tween().duration_secs(0.5))
  });
  let rendered = start(&mut reactant, &mut (), &document);
  assert_eq!(rendered.children.len(), 1);
  assert_eq!(rendered.children[0].children.len(), 1);
  let Prop::Set(descriptor) = &rendered.children[0].element.visual_element().motion else {
    panic!("forwarded host has no descriptor");
  };
  assert!(descriptor.initial.is_some());
  assert_eq!(descriptor.slots.len(), 1);
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn forwarding_component_rejects_missing_or_multiple_hosts() {
  let document = document();
  let mut missing = Reactant::new(IdleSpawner);
  missing.register_root(document.clone(), |(): &()| {
    MissingForwardingCard {
      motion: MotionProps::new(),
    }
    .animate(MotionStyle::new().opacity(1.0))
  });
  let result = panic::catch_unwind(AssertUnwindSafe(|| start(&mut missing, &mut (), &document)));
  assert!(result.is_err());

  let mut multiple = Reactant::new(IdleSpawner);
  multiple.register_root(document.clone(), |(): &()| {
    MultipleForwardingCard {
      motion: MotionProps::new(),
    }
    .animate(MotionStyle::new().opacity(1.0))
  });
  let result = panic::catch_unwind(AssertUnwindSafe(|| {
    start(&mut multiple, &mut (), &document)
  }));
  assert!(result.is_err());
}

#[test]
fn same_frame_retargets_advance_generation_without_recreating_the_host() {
  let document = document();
  let mut target = 0.2_f32;
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |target: &f32| {
    View::new().animate(MotionStyle::new().opacity(*target))
  });
  let rendered = start(&mut reactant, &mut target, &document);
  let host_id = rendered.children[0].object_id;

  target = 0.5;
  let first = motion_update(reactant.refresh(&mut target).unwrap());
  target = 0.9;
  let second = motion_update(reactant.refresh(&mut target).unwrap());
  assert_eq!(first.0, host_id);
  assert_eq!(second.0, host_id);
  assert_eq!(first.1, 2);
  assert_eq!(second.1, 3);
  let _ = reactant.shutdown(&mut target).into_groups();
}

#[test]
fn variants_propagate_merge_in_order_and_schedule_logical_children() {
  #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
  enum TestVariant {
    Open,
    Emphasis,
  }

  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |(): &()| {
    View::new()
      .variants(
        Variants::<TestVariant>::new()
          .target(TestVariant::Open, MotionStyle::new().x(8.0).opacity(0.4))
          .target(
            TestVariant::Emphasis,
            VariantTarget::new(
              MotionTarget::new(MotionStyle::new().opacity(1.0))
                .transition(Transition::tween().duration_secs(0.2)),
            )
            .orchestration(
              VariantOrchestration::new()
                .delay_children_secs(0.05)
                .stagger_secs(0.1)
                .when(VariantWhen::BeforeChildren),
            ),
          ),
      )
      .animate_variants([TestVariant::Open, TestVariant::Emphasis])
      .child(
        View::new().variants(
          Variants::<TestVariant>::new()
            .target(TestVariant::Open, MotionStyle::new().x(20.0).opacity(0.5))
            .target(TestVariant::Emphasis, MotionStyle::new().opacity(0.9)),
        ),
      )
      .child(
        View::new()
          .variants(
            Variants::<TestVariant>::new()
              .target(TestVariant::Open, MotionStyle::new().x(40.0))
              .target(TestVariant::Emphasis, MotionStyle::new().opacity(0.8)),
          )
          .inherit_variants(false),
      )
      .child(
        View::new().variants(
          Variants::<TestVariant>::new()
            .target(TestVariant::Open, MotionStyle::new().x(60.0))
            .target(TestVariant::Emphasis, MotionStyle::new().opacity(0.7)),
        ),
      )
  });
  let rendered = start(&mut reactant, &mut (), &document);
  let parent = &rendered.children[0];
  let descriptors = parent
    .children
    .iter()
    .map(|child| match &child.element.visual_element().motion {
      Prop::Set(value) => value,
      Prop::Unset | Prop::Reset => panic!("variant child is missing a descriptor"),
    })
    .collect::<Vec<_>>();
  assert_eq!(descriptors[0].variants.as_ref().unwrap().child_index, 0);
  assert_eq!(
    descriptors[0].variants.as_ref().unwrap().delay_micros,
    250_000
  );
  let first = &descriptors[0].slots[0].target.tracks;
  assert_eq!(
    first
      .iter()
      .find(|track| track.property == MotionProperty::Opacity)
      .unwrap()
      .values,
    [MotionValue::Scalar(0.9)]
  );
  assert_eq!(descriptors[2].variants.as_ref().unwrap().child_index, 1);
  assert_eq!(
    descriptors[2].variants.as_ref().unwrap().delay_micros,
    350_000
  );
  assert!(descriptors[1].variants.is_none());
  assert_eq!(
    first
      .iter()
      .find(|track| track.property == MotionProperty::X)
      .unwrap()
      .transition
      .delay_micros,
    250_000
  );
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn computed_variant_custom_data_is_snapshotted_until_selection_changes() {
  #[derive(Clone, Hash)]
  struct RouteData {
    offset: i32,
  }

  #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
  enum RouteVariant {
    Enter,
    Route,
    Exit,
  }

  let document = document();
  let mut data = RouteData { offset: -32 };
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |data: &RouteData| {
    View::new()
      .variants(
        Variants::<RouteVariant, RouteData>::new()
          .target(RouteVariant::Enter, MotionStyle::new().x(-12.0))
          .resolver(RouteVariant::Route, |snapshot| {
            VariantTarget::new(MotionStyle::new().x(snapshot.offset as f32))
          })
          .target(RouteVariant::Exit, MotionStyle::new().x(12.0)),
      )
      .custom(data.clone())
      .initial_variant(RouteVariant::Enter)
      .animate_variant(RouteVariant::Route)
      .exit_variant(RouteVariant::Exit)
  });
  let rendered = start(&mut reactant, &mut data, &document);
  let Prop::Set(first) = &rendered.children[0].element.visual_element().motion else {
    panic!("computed variant did not lower");
  };
  let snapshot = first.variants.as_ref().unwrap().custom_snapshot;
  let MotionValue::Length(initial_x) = first.initial.as_ref().unwrap().tracks[0].values[0] else {
    panic!("initial variant did not lower a length");
  };
  assert_eq!(initial_x.px, -12.0);
  data.offset = 96;
  assert!(reactant.refresh(&mut data).unwrap().is_empty());
  let _ = reactant.shutdown(&mut data).into_groups();
  assert_ne!(snapshot, 0);
}

#[test]
fn invalid_variant_maps_and_selections_panic_before_commit() {
  #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
  enum TestVariant {
    Open,
    Missing,
  }

  assert!(
    panic::catch_unwind(|| {
      Variants::<TestVariant>::new()
        .target(TestVariant::Open, MotionStyle::new().opacity(1.0))
        .target(TestVariant::Open, MotionStyle::new().opacity(0.5))
    })
    .is_err()
  );
  assert!(
    panic::catch_unwind(|| {
      MotionProps::new().animate_variants([TestVariant::Open, TestVariant::Open])
    })
    .is_err()
  );
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let document = document();
      let mut reactant = Reactant::new(IdleSpawner);
      reactant.register_root(document.clone(), |(): &()| {
        View::new()
          .variants(
            Variants::<TestVariant>::new()
              .target(TestVariant::Open, MotionStyle::new().opacity(1.0)),
          )
          .animate_variant(TestVariant::Missing)
      });
      let _ = start(&mut reactant, &mut (), &document);
    }))
    .is_err()
  );
}

#[test]
fn invalid_public_keyframe_times_fail_at_the_authoring_boundary() {
  let result = panic::catch_unwind(AssertUnwindSafe(|| {
    Keyframes::new([0.0, 1.0]).times([0.2, 1.0])
  }));
  assert!(result.is_err());
}

#[test]
fn css_authoring_lowers_pseudo_animation_and_keyed_decorations() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |(): &()| {
    View::new()
      .hover_style(Style::new().opacity(0.8))
      .focus_style(MotionStyle::new().scale(1.1))
      .style_transition(StyleTransition::new().property(
        StyleProperty::Opacity,
        Transition::tween().duration_secs(0.2),
      ))
      .animation(
        Animation::new(Keyframes::new([
          MotionStyle::new().x(0.0).scale(0.9),
          MotionStyle::new().x(40.0).scale(1.1),
        ]))
        .duration_secs(2.0)
        .iterations(AnimationIterations::Count(3))
        .direction(AnimationDirection::Alternate)
        .fill(AnimationFill::Both)
        .animation_key("pulse"),
      )
      .before(
        Decoration::new()
          .key(7_u8)
          .position(DecorationPosition::Border)
          .style(Style::new().opacity(0.5)),
      )
  });
  let rendered = start(&mut reactant, &mut (), &document);
  let Prop::Set(descriptor) = &rendered.children[0].element.visual_element().motion else {
    panic!("CSS authoring did not lower");
  };
  descriptor.validate().unwrap();
  assert_eq!(descriptor.pseudo_styles.len(), 2);
  assert_eq!(descriptor.style_transition.properties.len(), 1);
  assert_eq!(descriptor.animations.len(), 1);
  assert_eq!(descriptor.animations[0].tracks.len(), 2);
  assert!(matches!(
    descriptor.animations[0].tracks[0].transition.repeat,
    MotionRepeat::Count(2)
  ));
  assert_eq!(descriptor.decorations.len(), 1);
  assert_ne!(descriptor.decorations[0].key, 0);
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn css_and_motion_property_conflicts_fail_atomically() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |(): &()| {
    View::new()
      .animate(MotionStyle::new().opacity(1.0))
      .style_transition(StyleTransition::new().property(
        StyleProperty::Opacity,
        Transition::tween().duration_secs(0.2),
      ))
  });
  let result = panic::catch_unwind(AssertUnwindSafe(|| {
    let rendered = start(&mut reactant, &mut (), &document);
    let Prop::Set(descriptor) = &rendered.children[0].element.visual_element().motion else {
      panic!("descriptor is missing");
    };
    descriptor.validate().unwrap();
  }));
  assert!(result.is_err());
}

#[test]
fn typed_motion_values_controls_and_scopes_lower_closed_native_contract() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |(): &()| ValueContract);
  let rendered = start(&mut reactant, &mut (), &document);
  let root = &rendered.children[0];
  let Prop::Set(scope) = &root.element.visual_element().motion else {
    panic!("animation scope did not lower");
  };
  assert!(scope.scope_root);
  assert!(scope.scope_id.is_some());

  let Prop::Set(controlled) = &root.children[0].element.visual_element().motion else {
    panic!("animation controls did not lower");
  };
  assert!(controlled.control_id.is_some());
  assert_eq!(controlled.motion_name.as_deref(), Some("controlled"));
  assert_eq!(
    controlled
      .named_targets
      .iter()
      .map(|value| value.name.as_str())
      .collect::<Vec<_>>(),
    ["Rest", "Active"]
  );

  let Prop::Set(graph) = &root.children[1].element.visual_element().motion else {
    panic!("motion-value graph did not lower");
  };
  graph.validate().unwrap();
  assert_eq!(graph.values.len(), 2);
  assert_eq!(graph.value_bindings.len(), 1);
  assert_eq!(graph.value_bindings[0].property, MotionProperty::Opacity);

  let audio = AudioPlayback::new(ObjectId::new_v4());
  let play = audio.play_command(
    AudioClipAddress::from_static("test/pulse"),
    AudioPlaybackOptions::new().looping(true),
  );
  assert_eq!(play.command_id.into_uuid(), audio.id().into_uuid());
  assert!(matches!(play.body, CommandBody::AudioPlay(_)));
  assert!(matches!(audio.pause().body, CommandBody::AudioPause(_)));
  assert!(matches!(
    audio.seek(Duration::from_millis(350)).body,
    CommandBody::AudioSeek(_)
  ));
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn gesture_drag_scroll_and_viewport_props_lower_native_contract() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_: &Vec<MotionGestureEventKind>| {
    GestureContract
  });
  let mut events = Vec::new();
  let rendered = start(&mut reactant, &mut events, &document);
  let Prop::Set(descriptor) = &rendered.children[0].element.visual_element().motion else {
    panic!("gesture descriptor did not lower");
  };
  descriptor.validate().unwrap();
  let gestures = descriptor.gestures.unwrap();
  let drag = gestures.drag.unwrap();
  assert_eq!(drag.axis, MotionGestureAxis::Both);
  assert_eq!(drag.snap_to_origin, Some(MotionGestureAxis::X));
  assert!(drag.direction_lock);
  assert!(drag.propagation);
  assert!(drag.control_id.is_some());
  assert_eq!(drag.elastic.left, 0.1);
  assert_eq!(drag.transition.rest_speed, 5.0);
  assert_eq!(descriptor.values.len(), 5);
  assert!(gestures.subscriptions.hover);
  assert!(gestures.subscriptions.focus_visible);
  assert!(gestures.subscriptions.drag);
  assert!(gestures.subscriptions.scroll);
  assert!(gestures.subscriptions.in_view);

  let _ = reactant
    .motion_events(
      &mut events,
      MotionEventBatch {
        first_sequence: MotionSequence(0),
        last_sequence: MotionSequence(0),
        events: Vec::new(),
        samples: Vec::new(),
        value_samples: Vec::new(),
        playback_events: Vec::new(),
        gesture_events: vec![MotionGestureEvent {
          descriptor_id: descriptor.descriptor_id,
          generation: MotionGeneration(descriptor.generation.0),
          kind: MotionGestureEventKind::DragStart,
          pointer_id: 7,
          device: MotionPointerDevice::Pen,
          point: MotionGestureVector { x: 20.0, y: 30.0 },
          delta: MotionGestureVector::default(),
          offset: MotionGestureVector::default(),
          velocity: MotionGestureVector::default(),
          axis: Some(MotionGestureAxis::X),
          momentum_generation: 0,
          constrained: true,
        }],
      },
    )
    .unwrap();
  assert_eq!(events, [MotionGestureEventKind::DragStart]);
  let _ = reactant.shutdown(&mut events).into_groups();
}

#[test]
fn button_interaction_state_uses_native_focus_visible_events() {
  let document = document();
  let mut events = Vec::new();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_: &Vec<MotionGestureEventKind>| {
    ButtonInteractionContract
  });
  let rendered = start(&mut reactant, &mut events, &document);
  let Prop::Set(descriptor) = &rendered.children[0].element.visual_element().motion else {
    panic!("button interaction state did not install native subscriptions");
  };
  assert!(descriptor.gestures.unwrap().subscriptions.focus_visible);
  let commit = reactant
    .motion_events(
      &mut events,
      MotionEventBatch {
        first_sequence: MotionSequence(0),
        last_sequence: MotionSequence(0),
        events: Vec::new(),
        samples: Vec::new(),
        value_samples: Vec::new(),
        playback_events: Vec::new(),
        gesture_events: vec![MotionGestureEvent {
          descriptor_id: descriptor.descriptor_id,
          generation: descriptor.generation,
          kind: MotionGestureEventKind::FocusVisibleStart,
          pointer_id: -1,
          device: MotionPointerDevice::Keyboard,
          point: MotionGestureVector::default(),
          delta: MotionGestureVector::default(),
          offset: MotionGestureVector::default(),
          velocity: MotionGestureVector::default(),
          axis: None,
          momentum_generation: 0,
          constrained: false,
        }],
      },
    )
    .unwrap();
  assert!(!commit.into_groups().is_empty());
  assert_eq!(events, [MotionGestureEventKind::FocusVisibleStart]);
  let _ = reactant.shutdown(&mut events).into_groups();
}

#[test]
fn element_drag_constraints_resolve_on_their_first_shared_render() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| ElementConstraintContract);
  let rendered = start(&mut reactant, &mut (), &document);
  let constraint = &rendered.children[0];
  let target = &constraint.children[0];
  let Prop::Set(descriptor) = &target.element.visual_element().motion else {
    panic!("element drag constraint did not lower");
  };
  let drag = descriptor
    .gestures
    .expect("gesture descriptor")
    .drag
    .expect("drag descriptor");
  assert_eq!(
    drag.constraints,
    Some(MotionDragConstraint::Element(constraint.object_id))
  );
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn layout_projection_shared_handoff_and_reorder_lower_native_contract() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| LayoutContract);
  let rendered = start(&mut reactant, &mut (), &document);
  let root = &rendered.children[0];
  let Prop::Set(descriptor) = &root.element.visual_element().motion else {
    panic!("layout descriptor did not lower");
  };
  let layout = descriptor.layout.as_ref().expect("layout configuration");
  assert_eq!(layout.mode, battlement::MotionLayoutMode::Both);
  assert!(layout.layout_id.is_some());
  assert!(layout.scroll);
  assert!(layout.root);
  assert!(matches!(
    layout.transition.generator,
    TransitionGenerator::Tween {
      duration_micros: 250_000,
      ..
    }
  ));

  let Prop::Set(reorder) = &root.children[0].element.visual_element().motion else {
    panic!("reorder descriptor did not lower");
  };
  assert_eq!(
    reorder.layout.as_ref().unwrap().mode,
    battlement::MotionLayoutMode::Position
  );
  assert_eq!(
    reorder.gestures.unwrap().drag.unwrap().axis,
    MotionGestureAxis::Y
  );
  assert_eq!(reorder_index(1, 80.0, &[20.0, 60.0, 100.0]), 2);
  assert_eq!(reorder_index(2, -100.0, &[20.0, 60.0, 100.0]), 0);
  assert_eq!(reorder_index(1, 10.0, &[20.0, 60.0, 100.0]), 1);
  assert_eq!(reorder_index(1, 41.0, &[20.0, 60.0, 100.0]), 2);
  assert_eq!(reorder_index(1, -41.0, &[20.0, 60.0, 100.0]), 0);
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn motion_config_inherits_transition_and_reduced_motion_without_a_host() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| MotionConfigContract);
  let rendered = start(&mut reactant, &mut (), &document);
  let root = &rendered.children[0];
  let Prop::Set(descriptor) = &root.element.visual_element().motion else {
    panic!("configured motion descriptor did not lower");
  };
  assert_eq!(
    descriptor.reduced_motion,
    battlement::ReducedMotionPolicy::Always
  );
  assert!(matches!(
    descriptor.clock,
    battlement::MotionClockSource::Scaled
  ));
  let x = descriptor.slots[0]
    .target
    .tracks
    .iter()
    .find(|track| track.property == MotionProperty::X)
    .unwrap();
  let opacity = descriptor.slots[0]
    .target
    .tracks
    .iter()
    .find(|track| track.property == MotionProperty::Opacity)
    .unwrap();
  assert!(matches!(
    x.transition.generator,
    TransitionGenerator::Tween {
      duration_micros: 450_000,
      ..
    }
  ));
  assert!(matches!(
    opacity.transition.generator,
    TransitionGenerator::Tween {
      duration_micros: 200_000,
      ..
    }
  ));
  let _ = reactant.shutdown(&mut ()).into_groups();
}

fn motion_update(commit: ReactantCommit) -> (ObjectId, u32) {
  let commands = commit
    .into_groups()
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
  let [CommandBody::VisualElementUpdate(update)] = commands.as_slice() else {
    panic!("retarget should emit one sparse update");
  };
  let battlement::VisualElementUpdate::Properties { object_id, element } = update.as_ref() else {
    panic!("retarget should emit a property update");
  };
  let Prop::Set(descriptor) = &element.visual_element().motion else {
    panic!("retarget update is missing Motion");
  };
  (*object_id, descriptor.generation.0)
}

fn start<G: 'static>(
  reactant: &mut Reactant<G>,
  game: &mut G,
  document: &UiDocument,
) -> UiDocument {
  let (snapshot, commit) = reactant
    .begin_session(game)
    .unwrap()
    .into_parts(snapshot(document));
  let _ = commit.into_groups();
  snapshot
    .ui
    .into_iter()
    .find(|value| value.document_id == document.document_id)
    .unwrap()
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(document: &UiDocument) -> Snapshot {
  let camera_id = ObjectId::new_v4();
  Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    vec![
      GameObject::new(camera_id, CameraState::new()),
      GameObject::new(
        document.document_id,
        GameObjectKind::UiDocument(UiDocumentState::new(document.root_id).panel_settings(
          PanelSettings::new().scale_mode(PanelScaleMode::ConstantLogicalPixelSize),
        )),
      )
      .parent_scene(ParentScene::Persistent),
    ],
    camera_id,
  )
}
