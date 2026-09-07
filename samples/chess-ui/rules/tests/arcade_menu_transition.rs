use battlement::{
  AccessibilitySnapshot, CommandBody, GameObjectKind, Length, MotionDescriptor, MotionEasing,
  MotionEventBatch, MotionEventKind, MotionLayer, MotionLifecycleEvent, MotionProperty,
  MotionSequence, MotionTargetDescriptor, MotionValue, ObjectId, Prop, SemanticRole,
  TransitionGenerator, UiVisualElementProperties,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

#[test]
fn menu_transition_preserves_source_screen_scan_and_beam_frames() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-33");
  self::assert_only_specimen(&client, "Main transition specimen");
  assert!(self::named_optional(&mut client, "arcade-menu-contained-effect").is_none());

  self::click_named(&mut client, "menu-transition-route-settings");
  self::assert_only_specimen(&client, "Settings transition specimen");
  assert_eq!(self::panel_count(&mut client), 2);

  let incoming = self::descriptor_named(&mut client, "arcade-menu-screen-settings");
  let initial = incoming.initial.as_ref().expect("screen entrance");
  self::assert_clip(initial, 49.35, 8.0);
  self::assert_value(initial, MotionProperty::Opacity, MotionValue::Scalar(0.0));
  let center = &self::slot(&incoming, MotionLayer::Animate).target;
  self::assert_value(
    center,
    MotionProperty::ClipInset,
    MotionValue::ClipInset([Length::px(0.0); 4]),
  );
  self::assert_tween(
    center,
    MotionProperty::ClipInset,
    300_000,
    170_000,
    MotionEasing::CubicBezier([0.16, 1.0, 0.3, 1.0]),
  );

  let outgoing_id = self::named(&mut client, "arcade-menu-screen-main");
  assert_eq!(
    client
      .ui()
      .element(outgoing_id)
      .element()
      .visual_element()
      .inert,
    Prop::Set(true)
  );
  let outgoing = self::descriptor(&mut client, outgoing_id);
  self::assert_clip(&self::slot(&outgoing, MotionLayer::Exit).target, 49.35, 8.0);

  let scan = self::descriptor_named(&mut client, "arcade-menu-reveal-scan");
  self::assert_value(
    scan.initial.as_ref().expect("scan entrance"),
    MotionProperty::ClipInset,
    MotionValue::ClipInset(self::scan_clip(49.7)),
  );
  let scan_target = &self::slot(&scan, MotionLayer::Animate).target;
  let clip = self::track(scan_target, MotionProperty::ClipInset);
  assert_eq!(
    clip.values,
    [49.7, 46.0, 0.0].map(|value| MotionValue::ClipInset(self::scan_clip(value)))
  );
  assert_eq!(clip.times.as_deref(), Some(&[0.0, 0.44, 1.0][..]));
  self::assert_scalar_frames(
    scan_target,
    MotionProperty::Opacity,
    &[0.0, 0.48, 0.0],
    &[0.0, 0.44, 1.0],
  );
  self::assert_tween(
    scan_target,
    MotionProperty::ClipInset,
    500_000,
    0,
    MotionEasing::CubicBezier([0.65, 0.0, 0.35, 1.0]),
  );

  let beam = self::descriptor_named(&mut client, "arcade-menu-transition-beam");
  let beam_target = &self::slot(&beam, MotionLayer::Animate).target;
  self::assert_scalar_frames(
    beam_target,
    MotionProperty::Opacity,
    &[0.0, 0.68, 0.0],
    &[0.0, 0.46, 1.0],
  );
  self::assert_scalar_frames(
    beam_target,
    MotionProperty::ScaleX,
    &[0.25, 1.0, 0.7],
    &[0.0, 0.46, 1.0],
  );
  self::assert_tween(
    beam_target,
    MotionProperty::ScaleX,
    500_000,
    0,
    MotionEasing::EaseInOut,
  );
}

#[test]
fn interruption_reduced_motion_and_reset_leave_one_main_screen() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-33");
  self::click_named(&mut client, "menu-transition-route-settings");
  self::click_named(&mut client, "menu-transition-route-main");
  self::assert_only_specimen(&client, "Main transition specimen");
  assert!(self::panel_count(&mut client) >= 2);
  self::complete_all_exits(&mut client);
  assert_eq!(self::panel_count(&mut client), 1);

  let mut client = self::client();
  self::click_named(&mut client, "review-page-33");
  self::click_named(&mut client, "menu-transition-motion-policy");
  self::click_named(&mut client, "menu-transition-route-settings");
  self::assert_only_specimen(&client, "Settings transition specimen");
  assert_eq!(self::panel_count(&mut client), 1);
  let reduced = self::descriptor_named(&mut client, "arcade-menu-screen-settings");
  assert!(reduced.initial_disabled);
  assert!(
    reduced
      .slots
      .iter()
      .all(|slot| slot.layer != MotionLayer::Exit)
  );
  assert!(self::named_optional(&mut client, "arcade-menu-contained-effect").is_none());

  self::click_named(&mut client, "menu-transition-reset");
  self::assert_only_specimen(&client, "Main transition specimen");
  assert_eq!(self::panel_count(&mut client), 1);
  assert!(self::named_optional(&mut client, "arcade-menu-contained-effect").is_none());
}

fn assert_clip(target: &MotionTargetDescriptor, vertical: f32, horizontal: f32) {
  self::assert_value(
    target,
    MotionProperty::ClipInset,
    MotionValue::ClipInset(self::clip(vertical, horizontal)),
  );
}

fn clip(vertical: f32, horizontal: f32) -> [Length; 4] {
  [
    Length::percent(vertical),
    Length::percent(horizontal),
    Length::percent(vertical),
    Length::percent(horizontal),
  ]
}

fn scan_clip(vertical: f32) -> [Length; 4] {
  [
    Length::percent(vertical),
    Length::px(0.0),
    Length::percent(vertical),
    Length::px(0.0),
  ]
}

fn assert_value(target: &MotionTargetDescriptor, property: MotionProperty, expected: MotionValue) {
  assert_eq!(self::track(target, property).values, [expected]);
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

fn assert_tween(
  target: &MotionTargetDescriptor,
  property: MotionProperty,
  duration_micros: u64,
  delay_micros: i64,
  easing: MotionEasing,
) {
  let track = self::track(target, property);
  assert_eq!(track.transition.delay_micros, delay_micros);
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

fn complete_all_exits(client: &mut FakeClient<App>) {
  let mut events = Vec::new();
  for id in self::all_ids(client) {
    let Prop::Set(descriptor) = client
      .ui()
      .element(id)
      .element()
      .visual_element()
      .motion
      .clone()
    else {
      continue;
    };
    for slot in descriptor
      .slots
      .iter()
      .filter(|slot| slot.layer == MotionLayer::Exit)
    {
      events.push(MotionLifecycleEvent {
        sequence: MotionSequence(events.len() as u64 + 1),
        descriptor_id: descriptor.descriptor_id,
        slot: slot.slot,
        generation: slot.generation,
        elapsed_micros: 1_000_000,
        kind: MotionEventKind::Completed,
      });
    }
  }
  assert!(!events.is_empty(), "missing exit motion");
  client.submit_motion(MotionEventBatch {
    first_sequence: MotionSequence(1),
    last_sequence: MotionSequence(events.len() as u64),
    events,
    samples: Vec::new(),
    value_samples: Vec::new(),
    playback_events: Vec::new(),
    gesture_events: Vec::new(),
  });
  client.poll();
  client.poll();
}

fn panel_count(client: &mut FakeClient<App>) -> usize {
  ["arcade-menu-screen-main", "arcade-menu-screen-settings"]
    .into_iter()
    .map(|name| self::named_count(client, name))
    .sum()
}

fn assert_only_specimen(client: &FakeClient<App>, label: &str) {
  let regions = self::snapshot(client)
    .nodes
    .iter()
    .filter(|node| {
      node.role == SemanticRole::Region
        && node
          .label
          .as_deref()
          .is_some_and(|name| name.ends_with("transition specimen"))
    })
    .collect::<Vec<_>>();
  assert_eq!(regions.len(), 1);
  assert_eq!(regions[0].label.as_deref(), Some(label));
}

fn snapshot(client: &FakeClient<App>) -> &AccessibilitySnapshot {
  client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref(),
      _ => None,
    })
    .expect("accessibility snapshot")
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
