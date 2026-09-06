use battlement::{
  AccessibilitySnapshot, ClickEvent, CommandBody, GameObjectKind, KeyModifiers, Length,
  MotionDescriptor, MotionEasing, MotionEventBatch, MotionEventKind, MotionLayer,
  MotionLifecycleEvent, MotionProperty, MotionSequence, MotionTargetDescriptor, MotionValue,
  ObjectId, PanelPoint, PointerButton, Prop, SemanticRole, TransitionGenerator, UiEvent,
  UiVisualElementProperties,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{engine, select_control, setting_row};

#[test]
fn tabs_use_source_directional_motion_and_hide_exiting_semantics() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-27");
  self::assert_only_panel(&client, "Gameplay");
  let gameplay_id = self::panel_motion_id(&mut client, "Gameplay");

  self::click_label(&mut client, "Graphics");
  self::assert_only_panel(&client, "Graphics");
  let graphics_id = self::panel_motion_id(&mut client, "Graphics");
  assert_ne!(graphics_id, gameplay_id);
  let incoming = self::descriptor(&mut client, graphics_id);
  self::assert_initial_panel(&incoming, 58.0);
  self::assert_tween(
    &self::slot(&incoming, MotionLayer::Animate).target,
    MotionProperty::X,
    360_000,
    MotionEasing::CubicBezier([0.16, 1.0, 0.3, 1.0]),
  );
  let outgoing = self::descriptor(&mut client, gameplay_id);
  self::assert_exit_panel(&outgoing, -34.0);
  assert!(
    outgoing
      .layout
      .as_ref()
      .is_some_and(|layout| layout.pop_layout)
  );
  assert_eq!(
    client
      .ui()
      .element(gameplay_id)
      .element()
      .visual_element()
      .inert,
    Prop::Set(true)
  );
  self::assert_sweep(&mut client, -90.0, 940.0, -12.0);
  self::assert_scan(&mut client);

  let mut client = self::client();
  self::click_named(&mut client, "review-page-27");
  self::click_label(&mut client, "Input");
  let input_id = self::panel_motion_id(&mut client, "Input");
  self::click_label(&mut client, "Sound");
  let sound_id = self::panel_motion_id(&mut client, "Sound");
  self::assert_initial_panel(&self::descriptor(&mut client, sound_id), -58.0);
  self::assert_exit_panel(&self::descriptor(&mut client, input_id), 34.0);
  let sweep = self::descriptor_descendant_named(&mut client, sound_id, "arcade-tab-light-sweep");
  self::assert_sweep_descriptor(&sweep, 940.0, -90.0, 12.0);
}

#[test]
fn interruption_wrap_reduced_motion_and_reset_leave_one_panel() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-27");
  self::click_label(&mut client, "Graphics");
  self::click_label(&mut client, "Sound");
  self::assert_only_panel(&client, "Sound");
  assert_eq!(self::panel_count(&mut client), 3);
  self::complete_all_exits(&mut client);
  assert_eq!(self::panel_count(&mut client), 1);

  self::click_label(&mut client, "Input");
  self::complete_all_exits(&mut client);
  self::click_label(&mut client, "Gameplay");
  self::assert_initial_panel(
    &self::descriptor_named(&mut client, "arcade-tab-panel-gameplay"),
    -58.0,
  );
  self::complete_all_exits(&mut client);

  self::click_named(&mut client, "tab-transition-motion-policy");
  self::click_label(&mut client, "Graphics");
  let graphics_id = self::panel_motion_id(&mut client, "Graphics");
  let reduced = self::descriptor(&mut client, graphics_id);
  assert!(reduced.initial_disabled);
  assert!(
    reduced
      .slots
      .iter()
      .all(|slot| slot.layer != MotionLayer::Exit)
  );
  self::complete_all_exits(&mut client);
  assert_eq!(self::named_count(&mut client, "arcade-tab-light-sweep"), 0);
  assert_eq!(self::named_count(&mut client, "arcade-tab-scan-line"), 0);

  self::click_named(&mut client, "tab-transition-reset");
  self::assert_only_panel(&client, "Gameplay");
  assert_eq!(self::panel_count(&mut client), 1);
  assert_eq!(self::named_count(&mut client, "arcade-tab-light-sweep"), 0);
}

fn assert_initial_panel(descriptor: &MotionDescriptor, x: f32) {
  let initial = descriptor.initial.as_ref().expect("panel entrance");
  self::assert_value(initial, MotionProperty::Opacity, MotionValue::Scalar(0.0));
  self::assert_value(
    initial,
    MotionProperty::X,
    MotionValue::Length(Length::px(x)),
  );
  self::assert_value(
    initial,
    MotionProperty::Scale,
    MotionValue::Vector2([0.99, 0.99]),
  );
}

fn assert_exit_panel(descriptor: &MotionDescriptor, x: f32) {
  let exit = self::slot(descriptor, MotionLayer::Exit);
  self::assert_value(
    &exit.target,
    MotionProperty::Opacity,
    MotionValue::Scalar(0.0),
  );
  self::assert_value(
    &exit.target,
    MotionProperty::X,
    MotionValue::Length(Length::px(x)),
  );
  self::assert_value(
    &exit.target,
    MotionProperty::Scale,
    MotionValue::Vector2([1.01, 1.01]),
  );
  self::assert_tween(
    &exit.target,
    MotionProperty::X,
    150_000,
    MotionEasing::CubicBezier([0.7, 0.0, 1.0, 0.5]),
  );
}

fn assert_sweep(client: &mut FakeClient<App>, start: f32, end: f32, skew: f32) {
  let sweep = self::descriptor_named(client, "arcade-tab-light-sweep");
  self::assert_sweep_descriptor(&sweep, start, end, skew);
}

fn assert_sweep_descriptor(sweep: &MotionDescriptor, start: f32, end: f32, skew: f32) {
  let initial = sweep.initial.as_ref().expect("sweep entrance");
  self::assert_value(
    initial,
    MotionProperty::X,
    MotionValue::Length(Length::px(start)),
  );
  self::assert_value(initial, MotionProperty::SkewX, MotionValue::Angle(skew));
  let target = &self::slot(sweep, MotionLayer::Animate).target;
  self::assert_value(
    target,
    MotionProperty::X,
    MotionValue::Length(Length::px(end)),
  );
  self::assert_keyframes(
    target,
    MotionProperty::Opacity,
    &[0.0, 0.68, 0.68, 0.0],
    &[0.0, 0.22, 0.72, 1.0],
  );
  self::assert_tween(
    target,
    MotionProperty::X,
    340_000,
    MotionEasing::CubicBezier([0.4, 0.0, 0.2, 1.0]),
  );
}

fn assert_scan(client: &mut FakeClient<App>) {
  let scan = self::descriptor_named(client, "arcade-tab-scan-line");
  let initial = scan.initial.as_ref().expect("scan entrance");
  self::assert_value(
    initial,
    MotionProperty::Y,
    MotionValue::Length(Length::px(-12.0)),
  );
  let target = &self::slot(&scan, MotionLayer::Animate).target;
  self::assert_value(
    target,
    MotionProperty::Y,
    MotionValue::Length(Length::px(1000.0)),
  );
  self::assert_keyframes(
    target,
    MotionProperty::Opacity,
    &[0.0, 0.38, 0.22, 0.0],
    &[0.0, 0.1, 0.72, 1.0],
  );
  self::assert_tween(target, MotionProperty::Y, 420_000, MotionEasing::Linear);
}

fn assert_value(target: &MotionTargetDescriptor, property: MotionProperty, expected: MotionValue) {
  let track = self::track(target, property);
  assert_eq!(track.values, [expected]);
}

fn assert_keyframes(
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

fn assert_only_panel(client: &FakeClient<App>, label: &str) {
  let panels = self::snapshot(client)
    .nodes
    .iter()
    .filter(|node| node.role == SemanticRole::TabPanel)
    .collect::<Vec<_>>();
  assert_eq!(panels.len(), 1);
  assert_eq!(panels[0].label.as_deref(), Some(label));
}

fn panel_motion_id(client: &mut FakeClient<App>, label: &str) -> ObjectId {
  let content = self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::TabPanel && node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing {label} panel"))
    .object_id;
  client
    .ui()
    .element(content)
    .parent_id()
    .expect("panel content must belong to its motion host")
}

fn descriptor_named(client: &mut FakeClient<App>, name: &str) -> MotionDescriptor {
  let target = self::named(client, name);
  self::descriptor(client, target)
}

fn descriptor(client: &mut FakeClient<App>, target: ObjectId) -> MotionDescriptor {
  let motion = client
    .ui()
    .element(target)
    .element()
    .visual_element()
    .motion
    .clone();
  let Prop::Set(descriptor) = motion else {
    panic!("missing motion descriptor");
  };
  descriptor
}

fn descriptor_descendant_named(
  client: &mut FakeClient<App>,
  root: ObjectId,
  name: &str,
) -> MotionDescriptor {
  let mut pending = vec![root];
  while let Some(id) = pending.pop() {
    if client.ui().element(id).name() == Some(name) {
      return self::descriptor(client, id);
    }
    pending.extend(client.ui().element(id).children());
  }
  panic!("missing {name} below {root}")
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
  if events.is_empty() {
    return;
  }
  let last = events.len() as u64;
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
  client.poll();
}

fn click_named(client: &mut FakeClient<App>, name: &str) {
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

fn click_label(client: &mut FakeClient<App>, label: &str) {
  let target = self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Tab && node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing tab {label}"))
    .object_id;
  client.ui().click(target);
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

fn snapshot(client: &FakeClient<App>) -> &AccessibilitySnapshot {
  client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref(),
      _ => None,
    })
    .expect("committed semantics")
}

fn named(client: &mut FakeClient<App>, name: &str) -> ObjectId {
  let ids = self::all_ids(client);
  ids
    .iter()
    .copied()
    .find(|id| client.ui().element(*id).name() == Some(name))
    .unwrap_or_else(|| {
      let names = ids
        .into_iter()
        .filter_map(|id| client.ui().element(id).name().map(str::to_owned))
        .collect::<Vec<_>>();
      panic!("missing {name}; names: {names:?}")
    })
}

fn named_count(client: &mut FakeClient<App>, name: &str) -> usize {
  self::all_ids(client)
    .into_iter()
    .filter(|id| client.ui().element(*id).name() == Some(name))
    .count()
}

fn panel_count(client: &mut FakeClient<App>) -> usize {
  self::all_ids(client)
    .into_iter()
    .filter(|id| {
      client
        .ui()
        .element(*id)
        .name()
        .is_some_and(|name| name.starts_with("arcade-tab-panel-"))
    })
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
