use battlement::{
  ActionBody, CommandBody, MotionDragControlOperation, MotionEventBatch, MotionEventKind,
  MotionGeneration, MotionGestureEvent, MotionGestureEventKind, MotionGestureVector,
  MotionLifecycleEvent, MotionPointerDevice, MotionPresentationSample, MotionProperty,
  MotionPropertyValue, MotionSequence, MotionSlotId, MotionValue, ObjectId, json,
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
    gesture_events: Vec::new(),
  });
  let encoded = json::to_vec(&body).unwrap();
  assert_eq!(json::from_slice::<ActionBody>(&encoded).unwrap(), body);
  assert!(!String::from_utf8(encoded).unwrap().contains("Value"));
}

#[test]
fn external_drag_control_command_round_trips_pointer_identity() {
  let body = CommandBody::MotionDragControl(MotionDragControlOperation {
    control_id: ObjectId::new_v4(),
    pointer_id: 19,
    device: MotionPointerDevice::Pen,
    point: MotionGestureVector { x: 14.0, y: 28.0 },
    snap_to_cursor: true,
  });
  let encoded = json::to_vec(&body).unwrap();
  assert_eq!(json::from_slice::<CommandBody>(&encoded).unwrap(), body);
  assert!(
    String::from_utf8(encoded)
      .unwrap()
      .contains("MotionDragControl")
  );
}

#[test]
fn native_gesture_event_defaults_an_omitted_false_constraint_flag() {
  let event = MotionGestureEvent {
    descriptor_id: ObjectId::new_v4(),
    generation: MotionGeneration(4),
    kind: MotionGestureEventKind::Tap,
    pointer_id: 7,
    device: MotionPointerDevice::Mouse,
    point: MotionGestureVector { x: 10.0, y: 12.0 },
    delta: MotionGestureVector { x: 0.0, y: 0.0 },
    offset: MotionGestureVector { x: 0.0, y: 0.0 },
    velocity: MotionGestureVector { x: 0.0, y: 0.0 },
    axis: None,
    momentum_generation: 0,
    constrained: false,
  };
  let sparse = String::from_utf8(json::to_vec(&event).unwrap())
    .unwrap()
    .replace(",\"constrained\":false", "");

  assert_eq!(
    json::from_slice::<MotionGestureEvent>(sparse.as_bytes()).unwrap(),
    event
  );
}
