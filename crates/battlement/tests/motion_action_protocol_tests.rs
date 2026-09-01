use battlement::{
  ActionBody, MotionEventBatch, MotionEventKind, MotionGeneration, MotionLifecycleEvent,
  MotionPresentationSample, MotionProperty, MotionPropertyValue, MotionSequence, MotionSlotId,
  MotionValue, ObjectId, json,
};

#[test]
fn motion_event_action_round_trips_boundaries_and_samples() {
  let descriptor_id = ObjectId::new_v4();
  let body = ActionBody::MotionEvents(MotionEventBatch {
    first_sequence: MotionSequence(8),
    last_sequence: MotionSequence(8),
    events: vec![MotionLifecycleEvent {
      sequence: MotionSequence(8),
      descriptor_id,
      slot: MotionSlotId(3),
      generation: MotionGeneration(5),
      elapsed_micros: 240_000,
      kind: MotionEventKind::Repeated { first: 1, last: 2 },
    }],
    samples: vec![MotionPresentationSample {
      descriptor_id,
      slot: MotionSlotId(3),
      generation: MotionGeneration(5),
      elapsed_micros: 250_000,
      values: vec![MotionPropertyValue {
        property: MotionProperty::Opacity,
        value: MotionValue::Scalar(0.75),
      }],
    }],
    value_samples: Vec::new(),
    playback_events: Vec::new(),
  });
  let encoded = json::to_vec(&body).unwrap();
  assert_eq!(json::from_slice::<ActionBody>(&encoded).unwrap(), body);
  assert!(!String::from_utf8(encoded).unwrap().contains("Value"));
}
