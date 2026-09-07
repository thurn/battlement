use battlement::{
  CommandBody, GameObjectKind, MotionClockSource, MotionProperty, ObjectId, Prop, SemanticRole,
  UiVisualElementProperties,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{
  action_button, background_music::BACKGROUND_MUSIC, engine, music_heartbeat::heartbeat_strength,
  select_control, setting_row,
};

#[test]
fn heartbeat_uses_the_source_two_hit_audio_ledger() {
  assert!((heartbeat_strength(1.04) - 1.0).abs() < 0.000_01);
  assert!((heartbeat_strength(1.173_93) - 1.0).abs() < 0.000_01);
  assert_eq!(heartbeat_strength(1.32), 0.0);

  let mut client = self::client();
  self::click_named(&mut client, "review-page-32");
  self::semantic(&client, SemanticRole::Button, "Enable background music");
  self::named(&mut client, "music-speaker-slash");
  self::assert_no_animation(&mut client);

  self::click_named(&mut client, "music-playback-indicator");
  self::semantic(&client, SemanticRole::Button, "Mute background music");
  assert!(self::named_optional(&mut client, "music-speaker-slash").is_none());
  let descriptor = self::heartbeat_descriptor(&mut client);
  assert!(matches!(descriptor.clock, MotionClockSource::Audio(_)));
  let animation = descriptor
    .animations
    .iter()
    .find(|animation| animation.diagnostic_name.as_deref() == Some("music-control-heartbeat"))
    .expect("audio heartbeat animation");
  let scale = animation
    .tracks
    .iter()
    .find(|track| track.property == MotionProperty::Scale)
    .expect("heartbeat scale track");
  assert_eq!(scale.values.len(), 97);

  self::click_named(&mut client, "music-heartbeat-pulse");
  assert!(client.commands().iter().any(|entry| {
    matches!(
      &entry.command.body,
      CommandBody::AudioSeek(command) if command.position_ms == 1_040
    )
  }));

  self::click_named(&mut client, "music-heartbeat-motion-policy");
  self::assert_no_animation(&mut client);
}

#[test]
fn indicator_mutes_without_rewinding_and_enable_restores_zero_volumes() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-32");
  self::click_named(&mut client, "music-playback-indicator");
  assert_eq!(self::audio_plays(&client), 1);

  self::click_named(&mut client, "music-heartbeat-pulse");
  self::click_named(&mut client, "music-playback-indicator");
  self::semantic(&client, SemanticRole::Button, "Enable background music");
  self::named(&mut client, "music-speaker-slash");
  assert_eq!(self::audio_stops(&client), 0);

  self::click_named(&mut client, "music-heartbeat-zero-volume");
  self::click_named(&mut client, "music-playback-indicator");
  self::semantic(&client, SemanticRole::Button, "Mute background music");
  assert_eq!(self::audio_plays(&client), 1);
  assert_eq!(self::latest_volume(&client), 0.52);

  self::click_named(&mut client, "music-heartbeat-availability");
  self::semantic(&client, SemanticRole::Button, "Enable background music");
  self::named(&mut client, "music-speaker-slash");
  assert_eq!(self::audio_stops(&client), 1);

  self::click_named(&mut client, "music-heartbeat-reset");
  self::semantic(&client, SemanticRole::Button, "Enable background music");
  assert_eq!(self::audio_stops(&client), 1);
}

fn heartbeat_descriptor(client: &mut FakeClient<App>) -> battlement::MotionDescriptor {
  let target = self::named(client, "music-heartbeat");
  let Prop::Set(descriptor) = client
    .ui()
    .element(target)
    .element()
    .visual_element()
    .motion
    .clone()
  else {
    panic!("missing music heartbeat descriptor")
  };
  descriptor
}

fn assert_no_animation(client: &mut FakeClient<App>) {
  let target = self::named(client, "music-heartbeat");
  assert!(!matches!(
    client
      .ui()
      .element(target)
      .element()
      .visual_element()
      .motion,
    Prop::Set(_)
  ));
}

fn audio_plays(client: &FakeClient<App>) -> usize {
  client
    .commands()
    .iter()
    .filter(|entry| matches!(&entry.command.body, CommandBody::AudioPlay(_)))
    .count()
}

fn audio_stops(client: &FakeClient<App>) -> usize {
  client
    .commands()
    .iter()
    .filter(|entry| matches!(&entry.command.body, CommandBody::AudioStop(_)))
    .count()
}

fn latest_volume(client: &FakeClient<App>) -> f64 {
  client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AudioSetVolume(command) => Some(command.payload.volume),
      _ => None,
    })
    .expect("audio volume command")
}

fn semantic(client: &FakeClient<App>, role: SemanticRole, label: &str) -> ObjectId {
  client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref(),
      _ => None,
    })
    .expect("music indicator semantics")
    .nodes
    .iter()
    .find(|node| node.role == role && node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing {role:?} {label}"))
    .object_id
}

fn click_named(client: &mut FakeClient<App>, name: &str) {
  let target = self::named(client, name);
  client.ui().click(target);
  client.poll();
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

fn client() -> FakeClient<App> {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("chess-ui/content");
  assets.add_audio_clip(BACKGROUND_MUSIC);
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  assets.add_ui_font(setting_row::DISPLAY_FONT);
  assets.add_ui_font(select_control::VALUE_FONT);
  assets.add_ui_font(action_button::ACTION_FONT);
  let mut client = FakeClient::connect(engine::create_engine(), assets);
  client.poll();
  client
}
