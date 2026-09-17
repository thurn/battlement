use battlement::{
  Color, MaterialInstance, MaterialParameter, MaterialValue, MaterialVector, ObjectId, ParentScene,
  Vector3, object_id,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{app::App, hooks, host::ButtonHost, prelude::*, world};
use reactant_testing::Display;
use std::{
  panic::{self, AssertUnwindSafe},
  sync::Arc,
};
use trox::ls;

const LEFT: ObjectId = object_id!("32300000-0000-4000-8000-000000000001");
const RIGHT: ObjectId = object_id!("32300000-0000-4000-8000-000000000002");
const CLIP: MaterialParameter<f64> = MaterialParameter::new("_Clip");
const ACCENT: MaterialParameter<Color> = MaterialParameter::new("_Accent");
const WARP: MaterialParameter<MaterialVector> = MaterialParameter::new("_Warp");

struct Scene;
impl Component for Scene {
  fn render(&self) -> impl Render {
    let (changed, change) = hooks::use_state(false);
    let (clear, clear_values) = hooks::use_state(false);
    let (bad, set_bad) = hooks::use_state(0);
    let mut left = world::Sprite::new()
      .texture("test/art")
      .id(*LEFT.as_uuid())
      .size(2.0, 3.0)
      .position(Vector3::new(if bad == 0 { 0.0 } else { 99.0 }, 0.0, 0.0));
    if !clear {
      let mut material =
        MaterialInstance::new("test/material").parameter(CLIP, if changed { 0.8 } else { 0.2 });
      if changed {
        material = material
          .parameter(ACCENT, Color::WHITE)
          .parameter(WARP, MaterialVector([1.0, 2.0, 3.0, 4.0]));
      }
      if bad == 1 {
        material = material.parameter(MaterialParameter::<f64>::new("_Accent"), 0.5);
      }
      if bad == 2 {
        material = material.parameter(MaterialParameter::<f64>::new("_Missing"), 0.5);
      }
      if bad == 3 {
        material.address = "test/wrong".into();
      }
      left = left.material(material);
    }
    (
      ButtonHost::new(ls("Change"))
        .name("change")
        .on_click(change.update_callback(|v| !v)),
      ButtonHost::new(ls("Clear"))
        .name("clear")
        .on_click(clear_values.update_callback(|v| !v)),
      ButtonHost::new(ls("Wrong type")).name("wrong").on_click({
        let set_bad = set_bad.clone();
        move || set_bad.set(1)
      }),
      ButtonHost::new(ls("Missing")).name("missing").on_click({
        let set_bad = set_bad.clone();
        move || set_bad.set(2)
      }),
      ButtonHost::new(ls("Wrong asset"))
        .name("asset")
        .on_click(move || set_bad.set(3)),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        left,
        world::Sprite::new()
          .texture("test/art")
          .id(*RIGHT.as_uuid())
          .size(1.0, 2.0)
          .material(MaterialInstance::new("test/material").parameter(CLIP, 0.6)),
      )),
    )
  }
}
fn fixture() -> (Display<App>, ObjectId, Arc<FakeAssetCatalog>) {
  let app = App::new("test/scene").ui(Scene);
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("test/scene");
  assets.add_texture("test/art");
  assets.add_texture("test/wrong");
  assets.add_material_with_parameters(
    "test/material",
    [
      CLIP.value(0.0),
      ACCENT.value(Color::BLACK),
      WARP.value(MaterialVector([0.0; 4])),
    ],
  );
  let assets = Arc::new(assets);
  (Display::connect(app, assets.clone()), root, assets)
}
#[test]
fn independent_material_values_update_clear_and_reconnect_without_advancing_time() {
  let (mut display, root, assets) = fixture();
  let right = display.object(RIGHT).unwrap().clone();
  let time = display.presentation_time();
  let frame = display.frame();
  assert_eq!(
    display.object(LEFT).unwrap().material_parameter(0, "_Clip"),
    Some(&MaterialValue::Float(0.2))
  );
  let change = display.find_ui(root, "change");
  display.click_ui(change);
  assert_eq!(
    display.object(LEFT).unwrap().material_parameter(0, "_Clip"),
    Some(&MaterialValue::Float(0.8))
  );
  assert_eq!(
    display.object(LEFT).unwrap().material_parameter(0, "_Warp"),
    Some(&MaterialValue::Vector(MaterialVector([1.0, 2.0, 3.0, 4.0])))
  );
  assert_eq!(display.object(RIGHT).unwrap(), &right);
  assert_eq!(
    assets.material_parameter(&"test/material".into(), "_Clip"),
    Some(&MaterialValue::Float(0.0))
  );
  display.reconnect();
  assert_eq!(
    display.object(LEFT).unwrap().material_parameter(0, "_Clip"),
    Some(&MaterialValue::Float(0.8))
  );
  let clear = display.find_ui(root, "clear");
  display.click_ui(clear);
  assert!(
    display
      .object(LEFT)
      .unwrap()
      .material_parameter(0, "_Clip")
      .is_none()
  );
  assert_eq!(display.object(RIGHT).unwrap(), &right);
  assert_eq!(display.frame(), frame);
  assert_eq!(display.presentation_time(), time);
}
#[test]
fn required_parameter_declarations_fail_before_dependent_visual_changes() {
  for button in ["wrong", "missing", "asset"] {
    let (mut display, root, _) = fixture();
    let left = display.object(LEFT).unwrap().clone();
    let right = display.object(RIGHT).unwrap().clone();
    let target = display.find_ui(root, button);
    assert!(panic::catch_unwind(AssertUnwindSafe(|| display.click_ui(target))).is_err());
    assert_eq!(display.object(LEFT).unwrap(), &left);
    assert_eq!(display.object(RIGHT).unwrap(), &right);
  }
}
