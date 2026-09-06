use battlement::{
  ClickEvent, GameObjectKind, KeyModifiers, Length, MotionDescriptor, MotionEasing,
  MotionEventBatch, MotionEventKind, MotionLayer, MotionLifecycleEvent, MotionProperty,
  MotionSequence, MotionTargetDescriptor, MotionValue, ObjectId, PanelPoint, PointerButton, Prop,
  TransitionGenerator, UiEvent, UiVisualElementProperties,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{engine, select_control, setting_row};

#[test]
fn dropdown_uses_source_motion_and_reconnects_an_interrupted_exit() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-26");
  self::click_named(&mut client, "select-trigger");

  let menu = self::descriptor_named(&mut client, "select-popover-motion");
  let initial = menu.initial.as_ref().expect("menu entrance");
  self::assert_value(initial, MotionProperty::Opacity, MotionValue::Scalar(0.0));
  self::assert_value(
    initial,
    MotionProperty::Y,
    MotionValue::Length(Length::px(-12.0)),
  );
  self::assert_value(initial, MotionProperty::ScaleY, MotionValue::Scalar(0.76));
  let animate = self::slot(&menu, MotionLayer::Animate);
  self::assert_value(
    &animate.target,
    MotionProperty::Opacity,
    MotionValue::Scalar(1.0),
  );
  self::assert_value(
    &animate.target,
    MotionProperty::Y,
    MotionValue::Length(Length::px(0.0)),
  );
  self::assert_tween(
    &animate.target,
    MotionProperty::ScaleY,
    200_000,
    0,
    MotionEasing::CubicBezier([0.2, 0.8, 0.25, 1.0]),
  );

  for (index, name) in [
    "select-option-borderless",
    "select-option-fullscreen",
    "select-option-windowed",
  ]
  .into_iter()
  .enumerate()
  {
    let option = self::descriptor_named(&mut client, name);
    let initial = option.initial.as_ref().expect("option entrance");
    self::assert_value(
      initial,
      MotionProperty::X,
      MotionValue::Length(Length::px(-17.0)),
    );
    self::assert_value(initial, MotionProperty::Opacity, MotionValue::Scalar(0.0));
    self::assert_tween(
      &self::slot(&option, MotionLayer::Animate).target,
      MotionProperty::X,
      180_000,
      index as i64 * 28_000,
      MotionEasing::EaseOut,
    );
  }

  let caret = self::descriptor_named(&mut client, "select-caret");
  self::assert_value(
    &self::slot(&caret, MotionLayer::Animate).target,
    MotionProperty::Rotate,
    MotionValue::Angle(180.0),
  );
  self::assert_tween(
    &self::slot(&caret, MotionLayer::Animate).target,
    MotionProperty::Rotate,
    140_000,
    0,
    MotionEasing::CubicBezier([0.25, 0.1, 0.25, 1.0]),
  );

  self::click_named(&mut client, "select-option-windowed");
  let flash = self::descriptor_named(&mut client, "select-option-flash");
  let initial = flash.initial.as_ref().expect("selection flash entrance");
  self::assert_value(initial, MotionProperty::Opacity, MotionValue::Scalar(0.9));
  self::assert_value(
    initial,
    MotionProperty::Scale,
    MotionValue::Vector2([0.96, 0.96]),
  );
  self::assert_tween(
    &self::slot(&flash, MotionLayer::Animate).target,
    MotionProperty::Scale,
    380_000,
    0,
    MotionEasing::EaseOut,
  );
  let closing = self::descriptor_named(&mut client, "select-popover-motion");
  let exit = self::slot(&closing, MotionLayer::Exit);
  self::assert_value(
    &exit.target,
    MotionProperty::Y,
    MotionValue::Length(Length::px(-7.0)),
  );
  self::assert_value(
    &exit.target,
    MotionProperty::ScaleY,
    MotionValue::Scalar(0.42),
  );
  self::assert_tween(
    &exit.target,
    MotionProperty::Opacity,
    260_000,
    0,
    MotionEasing::CubicBezier([0.4, 0.0, 0.75, 0.3]),
  );

  self::click_named(&mut client, "select-trigger");
  client.poll();
  assert_eq!(self::named_count(&mut client, "select-listbox"), 1);
  let reopened = self::descriptor_named(&mut client, "select-popover-motion");
  assert!(
    reopened
      .slots
      .iter()
      .all(|slot| slot.layer != MotionLayer::Exit)
  );

  self::click_view_named(&mut client, "select-dismiss-layer");
  self::complete_all_exits(&mut client);
  assert_eq!(self::named_count(&mut client, "select-listbox"), 0);
  self::click_named(&mut client, "dropdown-animation-reset");
  assert_eq!(self::named_count(&mut client, "select-popover"), 0);
}

#[test]
fn reduced_motion_keeps_dropdown_spatially_still() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-26");
  self::click_named(&mut client, "dropdown-motion-policy");
  self::click_named(&mut client, "select-trigger");

  for name in ["select-popover-motion", "select-option-windowed"] {
    let descriptor = self::descriptor_named(&mut client, name);
    let initial = descriptor.initial.as_ref().expect("reduced entrance");
    assert_eq!(initial.tracks.len(), 1);
    self::assert_value(initial, MotionProperty::Opacity, MotionValue::Scalar(1.0));
    let animate = self::slot(&descriptor, MotionLayer::Animate);
    for track in &animate.target.tracks {
      assert_eq!(track.transition.delay_micros, 0);
      assert!(matches!(
        track.transition.generator,
        TransitionGenerator::Tween {
          duration_micros: 10_000,
          ..
        }
      ));
    }
  }

  let caret = self::descriptor_named(&mut client, "select-caret");
  assert!(
    caret
      .slots
      .iter()
      .flat_map(|slot| &slot.target.tracks)
      .all(|track| track.property != MotionProperty::Rotate)
  );

  self::click_view_named(&mut client, "select-dismiss-layer");
  let descriptor = self::descriptor_named(&mut client, "select-popover-motion");
  let exit = self::slot(&descriptor, MotionLayer::Exit);
  assert_eq!(exit.target.tracks.len(), 1);
  self::assert_value(
    &exit.target,
    MotionProperty::Opacity,
    MotionValue::Scalar(0.0),
  );
}

fn assert_value(target: &MotionTargetDescriptor, property: MotionProperty, expected: MotionValue) {
  let track = target
    .tracks
    .iter()
    .find(|track| track.property == property)
    .unwrap_or_else(|| panic!("missing {property:?}"));
  assert_eq!(track.values, [expected]);
}

fn assert_tween(
  target: &MotionTargetDescriptor,
  property: MotionProperty,
  duration_micros: u64,
  delay_micros: i64,
  easing: MotionEasing,
) {
  let track = target
    .tracks
    .iter()
    .find(|track| track.property == property)
    .unwrap_or_else(|| panic!("missing {property:?}"));
  assert_eq!(track.transition.delay_micros, delay_micros);
  assert!(matches!(
    &track.transition.generator,
    TransitionGenerator::Tween {
      duration_micros: actual,
      easings,
      ..
    } if *actual == duration_micros && easings == &[easing]
  ));
}

fn slot(descriptor: &MotionDescriptor, layer: MotionLayer) -> &battlement::MotionSlotDescriptor {
  descriptor
    .slots
    .iter()
    .find(|slot| slot.layer == layer)
    .unwrap_or_else(|| panic!("missing {layer:?} slot"))
}

fn descriptor(client: &mut FakeClient<App>, target: ObjectId) -> MotionDescriptor {
  let ui = client.ui();
  let Prop::Set(descriptor) = &ui.element(target).element().visual_element().motion else {
    panic!("missing motion descriptor");
  };
  descriptor.clone()
}

fn descriptor_named(client: &mut FakeClient<App>, name: &str) -> MotionDescriptor {
  let target = self::named(client, name);
  self::descriptor(client, target)
}

fn complete_all_exits(client: &mut FakeClient<App>) {
  let mut events = Vec::new();
  for id in self::all_ids(client) {
    let motion = client
      .ui()
      .element(id)
      .element()
      .visual_element()
      .motion
      .clone();
    let Prop::Set(descriptor) = motion else {
      continue;
    };
    for slot in descriptor
      .slots
      .iter()
      .filter(|slot| slot.layer == MotionLayer::Exit)
    {
      let sequence = MotionSequence(events.len() as u64 + 1);
      events.push(MotionLifecycleEvent {
        sequence,
        descriptor_id: descriptor.descriptor_id,
        slot: slot.slot,
        generation: slot.generation,
        elapsed_micros: 1_000_000,
        kind: MotionEventKind::Completed,
      });
    }
  }
  let last = events.len() as u64;
  assert!(last > 0, "missing exit motion");
  client.submit_motion(MotionEventBatch {
    first_sequence: MotionSequence(1),
    last_sequence: MotionSequence(last),
    events,
    samples: Vec::new(),
    value_samples: Vec::new(),
    playback_events: Vec::new(),
    gesture_events: Vec::new(),
  });
  client.poll();
}

fn click_named(client: &mut FakeClient<App>, name: &str) {
  let target = self::named(client, name);
  client.ui().click(target);
  client.poll();
}

fn click_view_named(client: &mut FakeClient<App>, name: &str) {
  let target = self::named(client, name);
  client.ui().send_event(UiEvent::click(
    target,
    ClickEvent::pointer(
      0,
      PanelPoint::default(),
      PointerButton::Left,
      1,
      KeyModifiers::default(),
    ),
  ));
  client.poll();
}

fn client() -> FakeClient<App> {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("chess-ui/content");
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  assets.add_ui_font(setting_row::DISPLAY_FONT);
  assets.add_ui_font(select_control::VALUE_FONT);
  let mut client = FakeClient::connect(engine::create_engine(), assets);
  client.poll();
  client
}

fn named(client: &mut FakeClient<App>, name: &str) -> ObjectId {
  self::all_ids(client)
    .into_iter()
    .find(|id| client.ui().element(*id).name() == Some(name))
    .unwrap_or_else(|| panic!("missing {name}"))
}

fn named_count(client: &mut FakeClient<App>, name: &str) -> usize {
  self::all_ids(client)
    .into_iter()
    .filter(|id| client.ui().element(*id).name() == Some(name))
    .count()
}

fn all_ids(client: &mut FakeClient<App>) -> Vec<ObjectId> {
  let mut pending = client
    .world()
    .objects()
    .filter_map(|object| match object.kind() {
      GameObjectKind::UiDocument(document) => Some(document.root_id()),
      _ => None,
    })
    .collect::<Vec<_>>();
  let mut ids = Vec::new();
  while let Some(id) = pending.pop() {
    ids.push(id);
    pending.extend(client.ui().element(id).children());
  }
  ids
}
