use battlement::{
  GameObjectKind, MotionDescriptor, MotionEasing, MotionLayer, MotionProperty, MotionRepeat,
  MotionRepeatType, MotionValue, ObjectId, Prop, Scale, StyleValue, TransitionGenerator,
  UiVisualElementProperties,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

#[test]
fn attract_mode_uses_source_grid_and_seeded_particle_motion() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-29");

  assert_eq!(
    self::named_prefix_count(&mut client, "arcade-attract-particle-"),
    48
  );
  assert_eq!(
    self::named_prefix_count(&mut client, "arcade-attract-grid-ray-"),
    9
  );
  assert_eq!(
    self::named_prefix_count(&mut client, "arcade-attract-grid-horizon-"),
    7
  );

  let grid = self::descriptor_named(&mut client, "arcade-attract-grid");
  let initial = grid.initial.as_ref().expect("grid initial state");
  self::assert_value(
    initial,
    MotionProperty::Y,
    MotionValue::Length((-6.0).into()),
  );
  self::assert_value(initial, MotionProperty::ScaleY, MotionValue::Scalar(0.96));
  self::assert_value(initial, MotionProperty::Opacity, MotionValue::Scalar(0.58));
  let grid_y = self::track(
    &self::slot(&grid, MotionLayer::Animate).target,
    MotionProperty::Y,
  );
  assert_eq!(grid_y.values, [MotionValue::Length(12.0.into())]);
  assert_eq!(grid_y.transition.repeat, MotionRepeat::Forever);
  assert_eq!(grid_y.transition.repeat_type, MotionRepeatType::Reverse);
  assert!(matches!(
    &grid_y.transition.generator,
    TransitionGenerator::Tween { duration_micros: 5_200_000, easings, .. }
      if easings == &[MotionEasing::EaseInOut]
  ));

  let particle = self::descriptor_named(&mut client, "arcade-attract-particle-0");
  let initial = particle.initial.as_ref().expect("particle initial state");
  self::assert_value(
    initial,
    MotionProperty::Y,
    MotionValue::Length(110.0.into()),
  );
  self::assert_value(
    initial,
    MotionProperty::Scale,
    MotionValue::Vector2([0.55, 0.55]),
  );
  self::assert_value(initial, MotionProperty::Opacity, MotionValue::Scalar(0.0));
  let opacity = self::track(
    &self::slot(&particle, MotionLayer::Animate).target,
    MotionProperty::Opacity,
  );
  assert_eq!(
    opacity.values,
    [0.0, 0.52, 0.96, 0.74, 0.0].map(MotionValue::Scalar)
  );
  assert_eq!(
    opacity.times.as_deref(),
    Some(&[0.0, 0.12, 0.38, 0.76, 1.0][..])
  );
  assert_eq!(opacity.transition.repeat, MotionRepeat::Forever);
  assert!(opacity.transition.delay_micros < 0);
  assert!(matches!(
    &opacity.transition.generator,
    TransitionGenerator::Tween { duration_micros, easings, .. }
      if (8_000_000..15_000_000).contains(duration_micros)
        && easings == &[MotionEasing::Linear]
  ));
}

#[test]
fn reset_preserves_the_seeded_scene_and_reduced_motion_is_static() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-29");
  let first = self::named(&mut client, "arcade-attract-particle-0");
  let first_style = client.ui().element(first).style().clone();

  self::click_named(&mut client, "attract-reset");
  let reset = self::named(&mut client, "arcade-attract-particle-0");
  assert_eq!(first, reset);
  assert_eq!(client.ui().element(reset).style(), &first_style);

  self::click_named(&mut client, "attract-motion-policy");
  let grid = self::named(&mut client, "arcade-attract-grid");
  assert!(!matches!(
    client.ui().element(grid).element().visual_element().motion,
    Prop::Set(_)
  ));
  assert_eq!(
    client.ui().element(grid).style().opacity,
    Prop::Set(StyleValue::Value(0.26.into()))
  );
  let particle = self::named(&mut client, "arcade-attract-particle-0");
  assert!(!matches!(
    client
      .ui()
      .element(particle)
      .element()
      .visual_element()
      .motion,
    Prop::Set(_)
  ));
  assert_eq!(
    client.ui().element(particle).style().opacity,
    Prop::Set(StyleValue::Value(0.62.into()))
  );
  assert_eq!(
    client.ui().element(particle).style().scale,
    Prop::Set(StyleValue::Value(Scale::uniform(0.85)))
  );
}

fn assert_value(
  target: &battlement::MotionTargetDescriptor,
  property: MotionProperty,
  expected: MotionValue,
) {
  assert_eq!(self::track(target, property).values, [expected]);
}

fn track(
  target: &battlement::MotionTargetDescriptor,
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
  let target = self::named(client, name);
  let motion = client
    .ui()
    .element(target)
    .element()
    .visual_element()
    .motion
    .clone();
  let Prop::Set(value) = motion else {
    panic!("missing motion descriptor for {name}")
  };
  value
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

fn named(client: &mut FakeClient<App>, name: &str) -> ObjectId {
  self::all_ids(client)
    .into_iter()
    .find(|id| client.ui().element(*id).name() == Some(name))
    .unwrap_or_else(|| panic!("missing {name}"))
}

fn named_prefix_count(client: &mut FakeClient<App>, prefix: &str) -> usize {
  self::all_ids(client)
    .into_iter()
    .filter(|id| {
      client
        .ui()
        .element(*id)
        .name()
        .is_some_and(|name| name.starts_with(prefix))
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
