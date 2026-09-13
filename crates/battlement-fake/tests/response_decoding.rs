use battlement::{CommandBody, Response};

#[test]
fn decodes_every_motion_command_from_verified_response_bytes() {
  let session_id = battlement::SessionId::new_v4();
  let value_id = battlement::ObjectId::new_v4();
  let playback_id = battlement::ObjectId::new_v4();
  let descriptor_id = battlement::ObjectId::new_v4();
  let clock_id = battlement::ObjectId::new_v4();
  let control_id = battlement::ObjectId::new_v4();
  let scope_id = battlement::ObjectId::new_v4();
  let target = battlement::MotionTargetDescriptor {
    tracks: vec![battlement::MotionPropertyTrack {
      property: battlement::MotionProperty::Opacity,
      values: vec![battlement::MotionValue::Scalar(0.25)],
      times: None,
      transition: battlement::TransitionDefinition::tween(),
    }],
    transition_end: Vec::new(),
  };
  let bodies = vec![
    CommandBody::MotionValue(battlement::MotionValueOperation {
      value_id,
      command: battlement::MotionValueCommand::Animate {
        playback_id,
        generation: 4,
        target: Box::new(battlement::MotionValue::Scalar(3.5)),
        transition: battlement::TransitionDefinition::spring(),
      },
    }),
    CommandBody::MotionValuePlayback(battlement::MotionValuePlaybackOperation {
      playback_id,
      generation: 4,
      command: battlement::MotionPlaybackCommand::SetSpeed { value: 1.5 },
    }),
    CommandBody::MotionPlayback(battlement::MotionPlaybackOperation {
      descriptor_id,
      slot: battlement::MotionSlotId(7),
      generation: battlement::MotionGeneration(2),
      command: battlement::MotionPlaybackCommand::Seek {
        elapsed_micros: 90_000,
      },
    }),
    CommandBody::MotionControlledClock(battlement::MotionControlledClockOperation {
      clock_id,
      command: battlement::MotionControlledClockCommand::Advance {
        delta_micros: 16_667,
      },
    }),
    CommandBody::MotionControl(battlement::MotionControlOperation {
      control_id,
      command: battlement::MotionControlCommand::Start {
        playback_id,
        generation: 5,
        target: battlement::MotionControlTarget::Target(target.clone()),
      },
    }),
    CommandBody::MotionScope(battlement::MotionScopeOperation {
      scope_id,
      command: battlement::MotionScopeCommand::Set {
        selector: battlement::MotionSelector::Descendants,
        target,
      },
    }),
    CommandBody::MotionDragControl(battlement::MotionDragControlOperation {
      control_id,
      pointer_id: 11,
      device: battlement::MotionPointerDevice::Touch,
      point: battlement::MotionGestureVector { x: 12.0, y: 34.0 },
      snap_to_cursor: true,
    }),
  ];
  let expected = Response::commands(session_id, bodies.clone());
  let bytes = battlement_flatbuffers::test_support::core_response(&expected)
    .expect("encode response fixture");

  let decoded = battlement_fake::read_response(bytes.as_bytes()).expect("decode response");
  let actual = decoded
    .messages
    .into_iter()
    .flat_map(|message| match message {
      battlement::ResponseMessage::Batch(batch) => batch
        .groups
        .into_iter()
        .flat_map(|group| group.commands)
        .map(|command| command.body)
        .collect(),
      battlement::ResponseMessage::Snapshot(_) => Vec::new(),
    })
    .collect::<Vec<_>>();

  assert_eq!(actual, bodies);
}

#[test]
fn decodes_nested_ui_state_from_verified_response_bytes() {
  let session_id = battlement::SessionId::new_v4();
  let document_id = battlement::ObjectId::new_v4();
  let root_id = battlement::ObjectId::new_v4();
  let host_id = battlement::ObjectId::new_v4();
  let value_id = battlement::ObjectId::new_v4();
  let descriptor = battlement::MotionDescriptor {
    descriptor_id: battlement::ObjectId::new_v4(),
    host_id,
    generation: battlement::MotionGeneration(2),
    initial: None,
    initial_disabled: false,
    slots: vec![battlement::MotionSlotDescriptor {
      slot: battlement::MotionSlotId(8),
      generation: battlement::MotionGeneration(3),
      layer: battlement::MotionLayer::Animate,
      target: battlement::MotionTargetDescriptor {
        tracks: vec![battlement::MotionPropertyTrack {
          property: battlement::MotionProperty::Opacity,
          values: vec![battlement::MotionValue::Scalar(0.75)],
          times: None,
          transition: battlement::TransitionDefinition::tween(),
        }],
        transition_end: Vec::new(),
      },
      callbacks: battlement::MotionCallbackSubscriptions::default(),
    }],
    clock: battlement::MotionClockSource::Unscaled,
    reduced_motion: battlement::ReducedMotionPolicy::Never,
    pseudo_styles: Vec::new(),
    style_transition: battlement::StyleTransitionDescriptor::default(),
    animations: Vec::new(),
    decorations: Vec::new(),
    variants: None,
    values: vec![battlement::MotionValueDescriptor {
      value_id,
      initial: battlement::MotionValue::Scalar(1.0),
      source: battlement::MotionValueSource::Mutable,
    }],
    value_bindings: vec![battlement::MotionValueBinding {
      property: battlement::MotionProperty::FlexGrow,
      value_id,
      composition: battlement::MotionBindingComposition::Replace,
    }],
    value_subscriptions: Vec::new(),
    control_id: None,
    scope_id: None,
    scope_root: false,
    motion_name: Some("card".to_owned()),
    named_targets: Vec::new(),
    gestures: None,
    layout: None,
  };
  let paint = battlement::PaintStyle::fill(battlement::Color::rgb(0.2, 0.3, 0.4)).box_shadow([
    battlement::Shadow {
      x: 1.0,
      y: 2.0,
      blur: 3.0,
      spread: 4.0,
      color: battlement::Color::rgba(0.5, 0.6, 0.7, 0.8),
      inset: false,
    },
  ]);
  let element = battlement::UiGroupBox::new()
    .text("transport")
    .style(
      battlement::Style::new()
        .opacity(0.6)
        .padding_left(12.0)
        .background_color(battlement::Color::rgb(0.1, 0.2, 0.3)),
    )
    .title_style(battlement::Style::new().width(18.0))
    .paint(paint)
    .motion_descriptor(descriptor);
  let document = battlement::UiDocument::with_root_id(document_id, root_id)
    .child(battlement::UiNode::new(host_id, element));
  let scene_id = battlement::SceneId::new_v4();
  let expected = Response::snapshot(
    battlement::Snapshot::new_with_main_camera(
      session_id,
      vec![battlement::PreparedAsset::scene("scene/main")],
      vec![battlement::Scene::new(scene_id, "scene/main")],
      Vec::new(),
    )
    .ui_document(document),
  );
  let bytes = battlement_flatbuffers::test_support::core_response(&expected)
    .expect("encode response fixture");

  let decoded = battlement_fake::read_response(bytes.as_bytes()).expect("decode response");

  assert_eq!(decoded, expected);
}
