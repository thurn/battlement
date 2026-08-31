use std::panic::{self, AssertUnwindSafe};

use battlement::{
  CameraState, CommandBody, GameObject, GameObjectKind, InertiaTarget as LoweredInertiaTarget,
  MotionEasing, MotionProperty, MotionRepeat, MotionRepeatType, MotionValue, ObjectId,
  PanelScaleMode, PanelSettings, ParentScene, PreparedAsset, Prop, Scene, SceneId, SessionId,
  Snapshot, SpringConfiguration, Style, TransitionGenerator, UiDocument, UiDocumentState,
  UiVisualElementProperties,
};
use battlement_reactant::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  prelude::*,
  runtime::{Reactant, ReactantCommit},
};

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
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
  reactant
    .begin_session(game)
    .unwrap()
    .into_parts(snapshot(document))
    .0
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
        GameObjectKind::UiDocument(
          UiDocumentState::new(document.root_id)
            .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantPixelSize)),
        ),
      )
      .parent_scene(ParentScene::Persistent),
    ],
    camera_id,
  )
}
