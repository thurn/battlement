use std::{fs, path::Path, sync::Arc};

use battlement::{ClickEvent, Command, KeyModifiers, ObjectId, PanelPoint, PointerButton, UiEvent};
use battlement_fake::{
  assets::FakeAssetCatalog,
  client::{FakeClient, ui::UiClient},
};
use battlement_native::Engine;
use battlement_rules::{
  CONTENT_SCENE, MOTION_AUDIO_CLIP, MOTION_MATERIAL, MOTION_TEXTURE, ROOT_ID, create_engine,
  generated_asset_addresses,
};

#[test]
fn release_lab_navigates_every_focused_screen() {
  let mut client = FakeClient::connect(
    create_engine().expect("Reactant sample engine should initialize"),
    catalog(),
  );
  for (navigation, canvas) in [
    ("composition-navigation", "composition-canvas"),
    ("events-navigation", "events-canvas"),
    ("state-navigation", "state-canvas"),
    ("context-navigation", "context-canvas"),
    ("effects-navigation", "effects-canvas"),
    ("resources-navigation", "resources-canvas"),
    ("refs-navigation", "refs-canvas"),
    ("assets-navigation", "assets-canvas"),
  ] {
    let navigation = find_named(&client.ui(), ROOT_ID, navigation);
    client.ui().click(navigation);
    let canvas = find_named(&client.ui(), ROOT_ID, canvas);
    assert!(!client.ui().element(canvas).children().is_empty());
  }
  let animation_navigation = find_named(&client.ui(), ROOT_ID, "targets-timelines-navigation");
  click_label(&mut client, animation_navigation);
  let values_navigation = find_named(&client.ui(), ROOT_ID, "values-navigation");
  click_label(&mut client, values_navigation);
  let canvas = find_named(&client.ui(), ROOT_ID, "values-time-controls-canvas");
  assert!(!client.ui().element(canvas).children().is_empty());
  let gestures_navigation = find_named(&client.ui(), ROOT_ID, "gestures-navigation");
  click_label(&mut client, gestures_navigation);
  let canvas = find_named(&client.ui(), ROOT_ID, "gestures-drag-canvas");
  assert!(!client.ui().element(canvas).children().is_empty());
  for (navigation, canvas) in [
    ("layout-gallery-navigation", "layout-gallery-canvas"),
    ("layout-reorder-navigation", "layout-reorder-canvas"),
    ("composed-effects-navigation", "composed-effects-canvas"),
    ("layout-performance-navigation", "layout-performance-canvas"),
    ("motion-performance-navigation", "motion-performance-canvas"),
  ] {
    let navigation = find_named(&client.ui(), ROOT_ID, navigation);
    click_label(&mut client, navigation);
    let canvas = find_named(&client.ui(), ROOT_ID, canvas);
    assert!(!client.ui().element(canvas).children().is_empty());
  }
}

#[test]
fn release_sample_source_contains_no_c_sharp() {
  assert_no_c_sharp(
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .parent()
      .expect("sample rules package should have a project parent"),
  );
}

fn click_label<E>(client: &mut FakeClient<E>, target_id: ObjectId)
where
  E: Engine<Command = Command>,
{
  client.ui().send_event(UiEvent::click(
    target_id,
    ClickEvent::pointer(
      0,
      PanelPoint::default(),
      PointerButton::Left,
      1,
      KeyModifiers::default(),
    ),
  ));
}

fn catalog() -> Arc<FakeAssetCatalog> {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene(CONTENT_SCENE);
  catalog.add_textures(generated_asset_addresses());
  catalog.add_material(MOTION_MATERIAL);
  catalog.add_texture(MOTION_TEXTURE);
  catalog.add_audio_clip(MOTION_AUDIO_CLIP);
  Arc::new(catalog)
}

fn find_named<E>(ui: &UiClient<'_, E>, root: ObjectId, expected: &str) -> ObjectId
where
  E: Engine<Command = Command>,
{
  let mut pending = vec![root];
  while let Some(object_id) = pending.pop() {
    let element = ui.element(object_id);
    if element.name() == Some(expected) {
      return object_id;
    }
    pending.extend(element.children());
  }
  panic!("missing UI element named {expected}");
}

fn assert_no_c_sharp(directory: &Path) {
  for entry in fs::read_dir(directory).expect("sample source directory should be readable") {
    let path = entry
      .expect("sample source entry should be readable")
      .path();
    if path.is_dir() {
      if !matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("Build" | "Library" | "Logs" | "Temp" | "UserSettings" | "obj" | "target")
      ) {
        assert_no_c_sharp(&path);
      }
    } else {
      assert_ne!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("cs"),
        "sample-specific C# is forbidden: {}",
        path.display()
      );
    }
  }
}
