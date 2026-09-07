use battlement::{
  GameObjectKind, Length, MotionDescriptor, MotionEasing, MotionEventBatch, MotionEventKind,
  MotionLayer, MotionLifecycleEvent, MotionProperty, MotionSequence, MotionTargetDescriptor,
  MotionValue, ObjectId, Prop, TransitionGenerator, UiVisualElementProperties,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

const EXIT_TIMES: &[f64] = &[0.0, 0.14, 0.38, 0.73, 1.0];

#[test]
fn exit_overlay_and_surfaces_preserve_the_source_ledger() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-34");
  assert_eq!(self::named_count(&mut client, "arcade-exit-content"), 1);
  assert!(self::named_optional(&mut client, "arcade-exit-sequence").is_none());

  self::click_named(&mut client, "exit-sequence-start");
  let content_id = self::named(&mut client, "arcade-exit-content-surface");
  assert_eq!(
    client
      .ui()
      .element(content_id)
      .element()
      .visual_element()
      .inert,
    Prop::Set(true)
  );
  let content = self::descriptor(&mut client, content_id);
  let content_target = &self::slot(&content, MotionLayer::Animate).target;
  self::assert_scalar_frames(
    content_target,
    MotionProperty::ScaleX,
    &[1.0, 1.008, 0.992, 1.025, 0.02],
    EXIT_TIMES,
  );
  self::assert_scalar_frames(
    content_target,
    MotionProperty::ScaleY,
    &[1.0, 0.994, 1.008, 0.035, 0.002],
    EXIT_TIMES,
  );
  self::assert_scalar_frames(
    content_target,
    MotionProperty::Opacity,
    &[1.0, 1.0, 1.0, 0.96, 0.0],
    EXIT_TIMES,
  );
  let clips = self::track(content_target, MotionProperty::ClipInset);
  assert_eq!(
    clips.values,
    [
      self::clip(0.0, 0.0, 0.0, 0.0),
      self::clip(0.0, 0.0, 0.0, 0.0),
      self::clip(0.0, 0.0, 0.0, 0.0),
      self::clip(46.62, 0.0, 52.48, 0.0),
      self::clip(47.02, 49.5, 52.88, 49.5),
    ]
    .map(MotionValue::ClipInset)
  );
  assert_eq!(clips.times.as_deref(), Some(EXIT_TIMES));
  self::assert_tween(
    content_target,
    MotionProperty::ClipInset,
    620_000,
    MotionEasing::CubicBezier([0.65, 0.0, 0.35, 1.0]),
  );

  let frame = self::descriptor_named(&mut client, "exit-frame-surface");
  let frame_target = &self::slot(&frame, MotionLayer::Animate).target;
  self::assert_scalar_frames(
    frame_target,
    MotionProperty::ScaleY,
    &[1.0, 0.996, 1.006, 0.045, 0.002],
    EXIT_TIMES,
  );
  self::assert_length_frames(
    frame_target,
    MotionProperty::X,
    &[0.0, 4.0, -4.0, 0.0, 0.0],
    EXIT_TIMES,
  );

  let flash = self::animate_named(&mut client, "arcade-exit-flash");
  self::assert_scalar_frames(
    &flash,
    MotionProperty::Opacity,
    &[0.0, 0.7, 0.25, 0.0],
    &[0.0, 0.25, 0.65, 1.0],
  );
  self::assert_tween(
    &flash,
    MotionProperty::Opacity,
    360_000,
    MotionEasing::EaseOut,
  );

  let beam = self::animate_named(&mut client, "arcade-exit-expanding-beam");
  self::assert_scalar_frames(
    &beam,
    MotionProperty::ScaleY,
    &[0.2, 1.0, 0.08],
    &[0.0, 0.34, 1.0],
  );
  self::assert_tween(
    &beam,
    MotionProperty::ScaleY,
    430_000,
    MotionEasing::CubicBezier([0.2, 0.8, 0.2, 1.0]),
  );

  self::assert_converging_line(&mut client, "arcade-exit-top-line", MotionProperty::Top);
  self::assert_converging_line(
    &mut client,
    "arcade-exit-bottom-line",
    MotionProperty::Bottom,
  );

  let central = self::animate_named(&mut client, "arcade-exit-central-collapse");
  self::assert_scalar_frames(
    &central,
    MotionProperty::Opacity,
    &[0.0, 0.0, 1.0, 0.92, 0.0],
    &[0.0, 0.52, 0.72, 0.87, 1.0],
  );
  self::assert_scalar_frames(
    &central,
    MotionProperty::ScaleX,
    &[0.08, 0.08, 1.0, 0.32, 0.01],
    &[0.0, 0.52, 0.72, 0.87, 1.0],
  );
  self::assert_tween(
    &central,
    MotionProperty::ScaleX,
    620_000,
    MotionEasing::EaseOut,
  );
}

#[test]
fn completion_reduced_motion_and_reset_end_in_one_stable_state() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-34");
  self::click_named(&mut client, "exit-sequence-start");
  self::complete_content(&mut client);
  assert_eq!(self::named_count(&mut client, "arcade-exit-black-stage"), 1);
  assert_eq!(self::named_count(&mut client, "arcade-exit-content"), 1);

  self::click_named(&mut client, "exit-sequence-reset");
  assert_eq!(self::named_count(&mut client, "arcade-exit-content"), 1);
  assert_eq!(self::named_count(&mut client, "arcade-exit-black-stage"), 0);

  let mut reduced_client = self::client();
  self::click_named(&mut reduced_client, "review-page-34");
  self::click_named(&mut reduced_client, "exit-sequence-motion-policy");
  self::click_named(&mut reduced_client, "exit-sequence-start");
  assert!(self::named_optional(&mut reduced_client, "arcade-exit-sequence").is_none());
  let content = self::descriptor_named(&mut reduced_client, "arcade-exit-content-surface");
  let target = &self::slot(&content, MotionLayer::Animate).target;
  assert_eq!(target.tracks.len(), 1);
  self::assert_tween(
    target,
    MotionProperty::Opacity,
    80_000,
    MotionEasing::CubicBezier([0.65, 0.0, 0.35, 1.0]),
  );
  self::complete_content(&mut reduced_client);
  assert_eq!(
    self::named_count(&mut reduced_client, "arcade-exit-black-stage"),
    1
  );
}

fn assert_converging_line(client: &mut FakeClient<App>, name: &str, property: MotionProperty) {
  let target = self::animate_named(client, name);
  let position = self::track(&target, property);
  assert_eq!(
    position.values,
    [7.0, 50.0, 50.0].map(|value| MotionValue::Length(Length::percent(value)))
  );
  assert_eq!(position.times.as_deref(), Some(&[0.0, 0.72, 1.0][..]));
  self::assert_scalar_frames(
    &target,
    MotionProperty::Opacity,
    &[0.0, 0.78, 0.0],
    &[0.0, 0.72, 1.0],
  );
  self::assert_tween(
    &target,
    property,
    500_000,
    MotionEasing::CubicBezier([0.7, 0.0, 0.3, 1.0]),
  );
}

fn complete_content(client: &mut FakeClient<App>) {
  let descriptor = self::descriptor_named(client, "arcade-exit-content-surface");
  let slot = self::slot(&descriptor, MotionLayer::Animate);
  let sequence = MotionSequence(1);
  client.submit_motion(MotionEventBatch {
    first_sequence: sequence,
    last_sequence: sequence,
    events: vec![MotionLifecycleEvent {
      sequence,
      descriptor_id: descriptor.descriptor_id,
      slot: slot.slot,
      generation: slot.generation,
      elapsed_micros: 620_000,
      kind: MotionEventKind::Completed,
    }],
    samples: Vec::new(),
    value_samples: Vec::new(),
    playback_events: Vec::new(),
    gesture_events: Vec::new(),
  });
  client.poll();
  client.poll();
}

fn clip(top: f32, right: f32, bottom: f32, left: f32) -> [Length; 4] {
  [top, right, bottom, left].map(Length::percent)
}

fn assert_scalar_frames(
  target: &MotionTargetDescriptor,
  property: MotionProperty,
  values: &[f32],
  times: &[f64],
) {
  let track = self::track(target, property);
  assert_eq!(
    track.values,
    values
      .iter()
      .copied()
      .map(MotionValue::Scalar)
      .collect::<Vec<_>>()
  );
  assert_eq!(track.times.as_deref(), Some(times));
}

fn assert_length_frames(
  target: &MotionTargetDescriptor,
  property: MotionProperty,
  values: &[f32],
  times: &[f64],
) {
  let track = self::track(target, property);
  assert_eq!(
    track.values,
    values
      .iter()
      .copied()
      .map(|value| MotionValue::Length(Length::px(value)))
      .collect::<Vec<_>>()
  );
  assert_eq!(track.times.as_deref(), Some(times));
}

fn assert_tween(
  target: &MotionTargetDescriptor,
  property: MotionProperty,
  duration_micros: u64,
  easing: MotionEasing,
) {
  let track = self::track(target, property);
  assert!(matches!(
    &track.transition.generator,
    TransitionGenerator::Tween { duration_micros: actual, easings, .. }
      if *actual == duration_micros && easings == &[easing]
  ));
}

fn track(
  target: &MotionTargetDescriptor,
  property: MotionProperty,
) -> &battlement::MotionPropertyTrack {
  target
    .tracks
    .iter()
    .find(|track| track.property == property)
    .unwrap_or_else(|| panic!("missing {property:?}"))
}

fn slot(descriptor: &MotionDescriptor, layer: MotionLayer) -> &battlement::MotionSlotDescriptor {
  descriptor
    .slots
    .iter()
    .find(|slot| slot.layer == layer)
    .unwrap_or_else(|| panic!("missing {layer:?} slot"))
}

fn animate_named(client: &mut FakeClient<App>, name: &str) -> MotionTargetDescriptor {
  self::slot(&self::descriptor_named(client, name), MotionLayer::Animate)
    .target
    .clone()
}

fn descriptor_named(client: &mut FakeClient<App>, name: &str) -> MotionDescriptor {
  let id = self::named(client, name);
  self::descriptor(client, id)
}

fn descriptor(client: &mut FakeClient<App>, id: ObjectId) -> MotionDescriptor {
  let Prop::Set(descriptor) = client
    .ui()
    .element(id)
    .element()
    .visual_element()
    .motion
    .clone()
  else {
    panic!("missing motion descriptor")
  };
  descriptor
}

fn click_named(client: &mut FakeClient<App>, name: &str) {
  let target = self::named(client, name);
  client.ui().click(target);
  client.poll();
}

fn client() -> FakeClient<App> {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("chess-ui/content");
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  assets.add_ui_font(setting_row::DISPLAY_FONT);
  assets.add_ui_font(select_control::VALUE_FONT);
  assets.add_ui_font(action_button::ACTION_FONT);
  let mut client = FakeClient::connect(engine::create_engine(), assets);
  client.poll();
  client
}

fn named_count(client: &mut FakeClient<App>, name: &str) -> usize {
  self::all_ids(client)
    .into_iter()
    .filter(|id| client.ui().element(*id).name() == Some(name))
    .count()
}

fn named(client: &mut FakeClient<App>, name: &str) -> ObjectId {
  self::named_optional(client, name).unwrap_or_else(|| panic!("missing {name}"))
}

fn named_optional(client: &mut FakeClient<App>, name: &str) -> Option<ObjectId> {
  self::all_ids(client)
    .into_iter()
    .find(|id| client.ui().element(*id).name() == Some(name))
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
