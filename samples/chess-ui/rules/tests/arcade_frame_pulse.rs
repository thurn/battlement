use battlement::{
  GameObjectKind, Length, LengthOrAuto, MotionDescriptor, MotionEasing, MotionProperty,
  MotionRepeat, MotionValue, ObjectId, Prop, StyleValue, TransitionGenerator,
  UiVisualElementProperties,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

#[test]
fn frame_comets_follow_the_source_perimeter_timeline() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-30");
  assert!(self::named_optional(&mut client, "arcade-frame-pulse").is_none());
  self::click_named(&mut client, "frame-pulse-preview");

  self::named(&mut client, "arcade-frame-comet-0");
  self::named(&mut client, "arcade-frame-comet-1");
  let descriptor = self::descriptor_named(&mut client, "arcade-frame-comet-0-top-beam");
  assert_eq!(descriptor.animations.len(), 1);
  let animation = &descriptor.animations[0];
  assert_eq!(
    animation.diagnostic_name.as_deref(),
    Some("arcade-border-comet-lap")
  );
  self::assert_track(
    &animation.tracks,
    MotionProperty::X,
    [0.0, 982.0, 982.0, 982.0, 982.0, 0.0, 0.0, 0.0, 0.0]
      .map(|value| MotionValue::Length(Length::px(value))),
  );
  self::assert_track(
    &animation.tracks,
    MotionProperty::Y,
    [0.0, 0.0, 0.0, 1404.0, 1404.0, 1404.0, 1404.0, 0.0, 0.0]
      .map(|value| MotionValue::Length(Length::px(value))),
  );
  self::assert_track(
    &animation.tracks,
    MotionProperty::Rotate,
    [0.0, 0.0, 90.0, 90.0, 180.0, 180.0, 270.0, 270.0, 360.0].map(MotionValue::Angle),
  );
  let track = self::track(&animation.tracks, MotionProperty::X);
  assert_eq!(
    track.times,
    [0.0, 0.24, 0.25, 0.49, 0.5, 0.74, 0.75, 0.99, 1.0]
  );
  assert_eq!(track.transition.repeat, MotionRepeat::Forever);
  assert!(matches!(
    &track.transition.generator,
    TransitionGenerator::Tween { duration_micros: 6_500_000, easings, .. }
      if easings == &[MotionEasing::Linear]
  ));
}

#[test]
fn settings_cutout_and_reduced_motion_are_explicit_review_states() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-30");
  assert!(self::named_optional(&mut client, "arcade-frame-pulse").is_none());
  self::click_named(&mut client, "frame-pulse-preview");
  self::named(&mut client, "arcade-frame-comet-0-bottom-window");
  assert!(self::named_optional(&mut client, "return-button").is_none());

  self::click_named(&mut client, "frame-pulse-context");
  self::named(&mut client, "return-button");
  assert!(self::named_optional(&mut client, "arcade-frame-comet-0-bottom-window").is_none());
  self::assert_window(
    &mut client,
    "arcade-frame-comet-0-bottom-left-window",
    15.0,
    282.0,
  );
  self::assert_window(
    &mut client,
    "arcade-frame-comet-0-bottom-right-window",
    685.0,
    282.0,
  );

  self::click_named(&mut client, "frame-pulse-motion-policy");
  let pulse = self::named(&mut client, "arcade-frame-pulse");
  assert_eq!(
    client.ui().element(pulse).style().opacity,
    Prop::Set(StyleValue::Value(0.28.into()))
  );
  let beam = self::named(&mut client, "arcade-frame-comet-0-top-beam");
  assert!(!matches!(
    client.ui().element(beam).element().visual_element().motion,
    Prop::Set(_)
  ));

  self::click_named(&mut client, "frame-pulse-reset");
  self::named(&mut client, "arcade-frame-comet-0-bottom-window");
  assert!(self::named_optional(&mut client, "return-button").is_none());
  self::descriptor_named(&mut client, "arcade-frame-comet-0-top-beam");
}

fn assert_track<const N: usize>(
  tracks: &[battlement::CssPropertyTrack],
  property: MotionProperty,
  expected: [MotionValue; N],
) {
  assert_eq!(self::track(tracks, property).values, expected);
}

fn assert_window(client: &mut FakeClient<App>, name: &str, left: f32, width: f32) {
  let element = self::named(client, name);
  let ui = client.ui();
  let style = ui.element(element).style();
  assert_eq!(
    style.left,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(left)))
  );
  assert_eq!(
    style.width,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(width)))
  );
  assert_eq!(
    style.top,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(1389.0)))
  );
  assert_eq!(
    style.height,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(15.0)))
  );
}

fn track(
  tracks: &[battlement::CssPropertyTrack],
  property: MotionProperty,
) -> &battlement::CssPropertyTrack {
  tracks
    .iter()
    .find(|track| track.property == property)
    .unwrap_or_else(|| panic!("missing {property:?}"))
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
