use battlement::{
  ClickEvent, GameObjectKind, KeyModifiers, Length, MotionDescriptor, MotionEasing,
  MotionEventBatch, MotionEventKind, MotionLayer, MotionLifecycleEvent, MotionProperty,
  MotionRepeat, MotionSequence, MotionTargetDescriptor, MotionValue, ObjectId, PanelPoint,
  PointerButton, Prop, TransitionGenerator, UiEvent, UiVisualElementProperties,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

#[test]
fn modal_preserves_source_frames_and_reconnects_an_interrupted_exit() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-28");
  self::click_named(&mut client, "modal-animation-erase");

  let backdrop = self::descriptor_named(&mut client, "arcade-modal-backdrop");
  self::assert_value(
    backdrop.initial.as_ref().expect("backdrop entrance"),
    MotionProperty::Opacity,
    MotionValue::Scalar(0.0),
  );
  self::assert_tween(
    &self::slot(&backdrop, MotionLayer::Animate).target,
    MotionProperty::Opacity,
    200_000,
  );

  let panel = self::descriptor_named(&mut client, "arcade-modal-panel");
  let initial = panel.initial.as_ref().expect("panel entrance");
  self::assert_value(initial, MotionProperty::Opacity, MotionValue::Scalar(0.0));
  self::assert_value(initial, MotionProperty::ScaleX, MotionValue::Scalar(0.72));
  self::assert_value(initial, MotionProperty::ScaleY, MotionValue::Scalar(0.04));
  self::assert_value(
    initial,
    MotionProperty::X,
    MotionValue::Length(Length::px(-30.0)),
  );
  let entrance = &self::slot(&panel, MotionLayer::Animate).target;
  self::assert_scalar_frames(
    entrance,
    MotionProperty::Opacity,
    &[0.0, 1.0, 0.72, 1.0],
    &[0.0, 0.48, 0.72, 1.0],
  );
  self::assert_tween(entrance, MotionProperty::ScaleX, 420_000);
  assert!(self::track(entrance, MotionProperty::Filter).values.len() == 4);
  let chrome = self::descriptor_named(&mut client, "arcade-modal-panel-chrome");
  self::assert_value(
    chrome.initial.as_ref().expect("chrome entrance"),
    MotionProperty::SkewX,
    MotionValue::Angle(-7.0),
  );
  let chrome_entrance = &self::slot(&chrome, MotionLayer::Animate).target;
  self::assert_scalar_frames(
    chrome_entrance,
    MotionProperty::SkewX,
    &[-7.0, 2.5, -1.0, 0.0],
    &[0.0, 0.48, 0.72, 1.0],
  );
  assert!(
    self::track(chrome_entrance, MotionProperty::PaintFilter)
      .values
      .len()
      == 4
  );

  let shine = self::descriptor_named(&mut client, "arcade-modal-shine");
  let shine_x = self::track(
    &self::slot(&shine, MotionLayer::Animate).target,
    MotionProperty::X,
  );
  assert_eq!(shine_x.values, [MotionValue::Length(Length::px(190.0))]);
  assert_eq!(shine_x.transition.repeat, MotionRepeat::Forever);
  assert_eq!(shine_x.transition.repeat_delay_micros, 1_200_000);
  assert!(matches!(
    &shine_x.transition.generator,
    TransitionGenerator::Tween { duration_micros: 1_800_000, easings, .. }
      if easings == &[MotionEasing::Linear]
  ));

  self::click_view_named(&mut client, "arcade-modal-backdrop");
  let closing_panel_id = self::named(&mut client, "arcade-modal-panel");
  assert_eq!(
    client
      .ui()
      .element(closing_panel_id)
      .element()
      .visual_element()
      .inert,
    Prop::Set(true)
  );
  let closing = self::descriptor(&mut client, closing_panel_id);
  let exit = &self::slot(&closing, MotionLayer::Exit).target;
  self::assert_scalar_frames(
    exit,
    MotionProperty::ScaleY,
    &[1.0, 0.82, 0.035],
    &[0.0, 0.5, 1.0],
  );
  self::assert_tween(exit, MotionProperty::Opacity, 300_000);
  let closing_chrome = self::descriptor_named(&mut client, "arcade-modal-panel-chrome");
  self::assert_scalar_frames(
    &self::slot(&closing_chrome, MotionLayer::Exit).target,
    MotionProperty::SkewX,
    &[0.0, -3.0, 8.0],
    &[0.0, 0.5, 1.0],
  );

  self::click_named(&mut client, "modal-animation-erase");
  client.poll();
  assert_eq!(self::named_count(&mut client, "arcade-modal-panel"), 1);
  assert_eq!(self::named_count(&mut client, "arcade-modal-shine"), 1);
  self::click_view_named(&mut client, "arcade-modal-backdrop");
  self::complete_all_exits(&mut client, 1);
  assert_eq!(self::named_count(&mut client, "arcade-modal-panel"), 0);

  self::click_named(&mut client, "modal-animation-help");
  assert_eq!(self::named_count(&mut client, "arcade-modal-panel"), 1);
}

#[test]
fn reduced_motion_uses_short_fades_and_reset_removes_shine() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-28");
  self::click_named(&mut client, "modal-animation-motion-policy");
  self::click_named(&mut client, "modal-animation-rebinding");

  let panel = self::descriptor_named(&mut client, "arcade-modal-panel");
  assert_eq!(
    panel
      .initial
      .as_ref()
      .expect("reduced entrance")
      .tracks
      .len(),
    1
  );
  let entrance = &self::slot(&panel, MotionLayer::Animate).target;
  assert_eq!(entrance.tracks.len(), 1);
  self::assert_tween(entrance, MotionProperty::Opacity, 10_000);
  assert_eq!(self::named_count(&mut client, "arcade-modal-shine"), 0);

  self::click_view_named(&mut client, "arcade-modal-backdrop");
  let exit = self::descriptor_named(&mut client, "arcade-modal-panel");
  let exit = &self::slot(&exit, MotionLayer::Exit).target;
  assert_eq!(exit.tracks.len(), 1);
  self::assert_tween(exit, MotionProperty::Opacity, 10_000);
  self::complete_all_exits(&mut client, 1);
  self::click_named(&mut client, "modal-animation-reset");
  assert_eq!(self::named_count(&mut client, "arcade-modal-panel"), 0);
  assert_eq!(self::named_count(&mut client, "arcade-modal-shine"), 0);
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
  let expected = if property == MotionProperty::SkewX {
    values
      .iter()
      .copied()
      .map(MotionValue::Angle)
      .collect::<Vec<_>>()
  } else {
    values
      .iter()
      .copied()
      .map(MotionValue::Scalar)
      .collect::<Vec<_>>()
  };
  assert_eq!(track.values, expected);
  assert_eq!(track.times.as_deref(), Some(times));
}

fn assert_tween(target: &MotionTargetDescriptor, property: MotionProperty, duration_micros: u64) {
  let track = self::track(target, property);
  assert!(matches!(
    &track.transition.generator,
    TransitionGenerator::Tween { duration_micros: actual, easings, .. }
      if *actual == duration_micros && easings == &[MotionEasing::EaseOut]
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

fn descriptor(client: &mut FakeClient<App>, target: ObjectId) -> MotionDescriptor {
  let motion = client
    .ui()
    .element(target)
    .element()
    .visual_element()
    .motion
    .clone();
  let Prop::Set(value) = motion else {
    panic!("missing motion descriptor")
  };
  value
}

fn descriptor_named(client: &mut FakeClient<App>, name: &str) -> MotionDescriptor {
  let target = self::named(client, name);
  self::descriptor(client, target)
}

fn complete_all_exits(client: &mut FakeClient<App>, first_sequence: u64) -> u64 {
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
  let event_count = events.len() as u64;
  for (index, event) in events.into_iter().enumerate() {
    let sequence = MotionSequence(first_sequence + index as u64);
    client.submit_motion(MotionEventBatch {
      first_sequence: sequence,
      last_sequence: sequence,
      events: vec![MotionLifecycleEvent { sequence, ..event }],
      samples: Vec::new(),
      value_samples: Vec::new(),
      playback_events: Vec::new(),
      gesture_events: Vec::new(),
    });
    client.poll();
  }
  first_sequence + event_count
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
  assets.add_ui_font(action_button::ACTION_FONT);
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
