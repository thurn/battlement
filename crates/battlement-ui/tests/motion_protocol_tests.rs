use battlement_types::ObjectId;
use battlement_ui::{
  MotionCallbackSubscriptions, MotionClockSource, MotionControlledClockCommand,
  MotionControlledClockOperation, MotionDescriptor, MotionEasing, MotionGeneration, MotionLayer,
  MotionPlaybackCommand, MotionPlaybackDirection, MotionPlaybackOperation, MotionProperty,
  MotionPropertyTrack, MotionPropertyValue, MotionRepeat, MotionRepeatType, MotionSlotDescriptor,
  MotionSlotId, MotionTargetDescriptor, MotionValue, MotionVariantResolution, ReducedMotionPolicy,
  StaggerDirection, TransitionDefinition, TransitionGenerator, VariantWhen,
};

#[test]
fn descriptor_json_round_trips_every_timeline_identity_and_field() {
  let host_id = ObjectId::new_v4();
  let descriptor = MotionDescriptor {
    descriptor_id: ObjectId::new_v4(),
    host_id,
    generation: MotionGeneration(9),
    static_baseline: vec![MotionPropertyValue {
      property: MotionProperty::Opacity,
      value: MotionValue::Scalar(0.2),
    }],
    initial: Some(MotionTargetDescriptor {
      tracks: vec![MotionPropertyTrack {
        property: MotionProperty::X,
        values: vec![MotionValue::Length(battlement_ui::MotionLength {
          px: -12.0,
          percent: 0.0,
        })],
        times: None,
        transition: TransitionDefinition {
          generator: TransitionGenerator::Immediate,
          delay_micros: 0,
          repeat: MotionRepeat::None,
          repeat_delay_micros: 0,
          repeat_type: MotionRepeatType::Loop,
        },
      }],
      transition_end: Vec::new(),
    }),
    initial_disabled: false,
    slots: vec![MotionSlotDescriptor {
      slot: MotionSlotId(42),
      generation: MotionGeneration(3),
      layer: MotionLayer::Hover,
      target: MotionTargetDescriptor {
        tracks: vec![MotionPropertyTrack {
          property: MotionProperty::Opacity,
          values: vec![
            MotionValue::Scalar(0.2),
            MotionValue::Scalar(0.7),
            MotionValue::Scalar(1.0),
          ],
          times: Some(vec![0.0, 0.4, 1.0]),
          transition: TransitionDefinition {
            generator: TransitionGenerator::Tween {
              duration_micros: 750_000,
              easings: vec![MotionEasing::EaseIn, MotionEasing::EaseOut],
              times: Some(vec![0.0, 0.4, 1.0]),
            },
            delay_micros: -50_000,
            repeat: MotionRepeat::Count(2),
            repeat_delay_micros: 25_000,
            repeat_type: MotionRepeatType::Mirror,
          },
        }],
        transition_end: vec![MotionPropertyValue {
          property: MotionProperty::Visibility,
          value: MotionValue::Discrete(serde_json::json!("hidden")),
        }],
      },
      callbacks: MotionCallbackSubscriptions {
        start: true,
        update: true,
        repeat: true,
        complete: true,
        stop: true,
        cancel: true,
      },
    }],
    clock: MotionClockSource::Controlled(ObjectId::new_v4()),
    reduced_motion: ReducedMotionPolicy::Always,
    pseudo_styles: Vec::new(),
    style_transition: battlement_ui::StyleTransitionDescriptor::default(),
    animations: Vec::new(),
    decorations: Vec::new(),
    values: Vec::new(),
    value_bindings: Vec::new(),
    value_subscriptions: Vec::new(),
    control_id: None,
    scope_id: None,
    scope_root: false,
    motion_name: None,
    named_targets: Vec::new(),
    variants: Some(MotionVariantResolution {
      names: vec!["west".to_owned(), "selected".to_owned()],
      inherited: true,
      custom_snapshot: 91,
      child_index: 3,
      delay_micros: 470_000,
      when: VariantWhen::AfterChildren,
      stagger_direction: StaggerDirection::Reverse,
    }),
  };

  let json = serde_json::to_string(&descriptor).unwrap();
  let decoded: MotionDescriptor = serde_json::from_str(&json).unwrap();
  assert_eq!(decoded, descriptor);
  assert!(json.contains("\"Mirror\""));
  assert!(json.contains("\"Controlled\""));
  assert!(json.contains("\"transition_end\""));
  assert!(json.contains("\"custom_snapshot\":91"));
}

#[test]
fn playback_and_controlled_clock_operations_round_trip_every_variant_shape() {
  let playback = MotionPlaybackOperation {
    descriptor_id: ObjectId::new_v4(),
    slot: MotionSlotId(7),
    generation: MotionGeneration(3),
    command: MotionPlaybackCommand::SetDirection {
      value: MotionPlaybackDirection::AlternateReverse,
    },
  };
  let clock = MotionControlledClockOperation {
    clock_id: ObjectId::new_v4(),
    command: MotionControlledClockCommand::Advance {
      delta_micros: 250_000,
    },
  };

  let playback_json = serde_json::to_string(&playback).unwrap();
  let clock_json = serde_json::to_string(&clock).unwrap();
  assert_eq!(
    serde_json::from_str::<MotionPlaybackOperation>(&playback_json).unwrap(),
    playback
  );
  assert_eq!(
    serde_json::from_str::<MotionControlledClockOperation>(&clock_json).unwrap(),
    clock
  );
  assert!(playback_json.contains("AlternateReverse"));
  assert!(clock_json.contains("delta_micros"));
}
