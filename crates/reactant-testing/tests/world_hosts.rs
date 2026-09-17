use std::{
  cell::RefCell,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{
  CameraProjection, Color, GameObjectKind, LightType, MaterialAssignment, ObjectId, ParentScene,
  Quaternion, Vector3, object_id,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{app::App, hooks, host::ButtonHost, native_host, prelude::*, world};
use reactant_testing::Display;
use trox::ls;

const GROUP: ObjectId = object_id!("32100000-0000-4000-8000-000000000001");
const FRONT: ObjectId = object_id!("32100000-0000-4000-8000-000000000002");
const BACK: ObjectId = object_id!("32100000-0000-4000-8000-000000000003");
const MESH: ObjectId = object_id!("32100000-0000-4000-8000-000000000004");
const LIGHT: ObjectId = object_id!("32100000-0000-4000-8000-000000000005");
const CAMERA: ObjectId = object_id!("32100000-0000-4000-8000-000000000006");

struct Scene(Rc<RefCell<Option<ObjectRef>>>);

impl Component for Scene {
  fn render(&self) -> impl Render {
    let mesh_ref = native_host::use_object_ref();
    self.0.replace(Some(mesh_ref.clone()));
    let (changed, change) = hooks::use_state(false);
    let (front, show_front) = hooks::use_state(true);
    let (missing, load_missing) = hooks::use_state(false);
    (
      ButtonHost::new(ls("Change"))
        .name("change")
        .on_click(change.update_callback(|value| !value)),
      ButtonHost::new(ls("Face"))
        .name("face")
        .on_click(show_front.update_callback(|value| !value)),
      ButtonHost::new(ls("Missing"))
        .name("missing")
        .on_click(move || load_missing.set(true)),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        world::Group::new()
          .id(*GROUP.as_uuid())
          .position(Vector3::new(2.0, 3.0, 4.0))
          .child((
            front.then(|| {
              world::Sprite::new()
                .texture(if changed {
                  "test/front-b"
                } else {
                  "test/front-a"
                })
                .id(*FRONT.as_uuid())
                .size(if changed { 3.0 } else { 2.0 }, 4.0)
            }),
            world::Sprite::new()
              .texture("test/back")
              .id(*BACK.as_uuid())
              .size(2.0, 4.0)
              .rotation(Quaternion::new(0.0, 1.0, 0.0, 0.0))
              .position(Vector3::new(0.0, 0.0, 0.01)),
          )),
        world::Mesh::new()
          .mesh(if missing {
            "test/missing"
          } else if changed {
            "test/mesh-b"
          } else {
            "test/mesh-a"
          })
          .materials([MaterialAssignment::new(0, "test/material")])
          .id(*MESH.as_uuid())
          .reference(mesh_ref)
          .scale(Vector3::new(0.5, 1.5, 2.0))
          .rotation(Quaternion::new(0.0, 0.0, 1.0, 0.0)),
        world::Light::new()
          .id(*LIGHT.as_uuid())
          .light_type(if changed {
            LightType::Spot
          } else {
            LightType::Directional
          })
          .range(15.0)
          .spot_angles(15.0, 40.0)
          .intensity(if changed { 2.0 } else { 1.0 }),
        world::Camera::new()
          .id(*CAMERA.as_uuid())
          .enabled(false)
          .orthographic(if changed { 7.0 } else { 5.0 })
          .clipping(0.1, 100.0)
          .background(if changed { Color::WHITE } else { Color::BLACK }),
      )),
    )
  }
}

fn fixture() -> (Display<App>, ObjectId, Rc<RefCell<Option<ObjectRef>>>) {
  let mesh = Rc::new(RefCell::new(None));
  let app = App::new("test/scene")
    .ui(Scene(mesh.clone()))
    .camera(|camera| {
      world::Camera::new()
        .orthographic(5.0)
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("test/scene");
  assets.add_textures(["test/front-a", "test/front-b", "test/back"]);
  assets.add_mesh("test/mesh-a", 1);
  assets.add_mesh("test/mesh-b", 1);
  assets.add_material("test/material");
  (Display::connect(app, assets), root, mesh)
}

#[test]
fn typed_hosts_keep_explicit_geometry_and_update_existing_components() {
  let (mut display, root, _) = self::fixture();
  assert_eq!(
    display.object(GROUP).unwrap().local_transform().position,
    Vector3::new(2.0, 3.0, 4.0)
  );
  let back = display.object(BACK).unwrap().local_transform();
  assert_eq!(back.rotation, Quaternion::new(0.0, 1.0, 0.0, 0.0));
  assert_eq!(
    display.object(MESH).unwrap().local_transform().scale,
    Vector3::new(0.5, 1.5, 2.0)
  );
  let frame = display.frame();
  let time = display.presentation_time();
  let change = display.find_ui(root, "change");
  display.click_ui(change);
  let GameObjectKind::Image { image } = display.object(FRONT).unwrap().kind() else {
    panic!("sprite host");
  };
  assert_eq!(image.texture.as_str(), "test/front-b");
  assert_eq!(image.width, 3.0);
  assert_eq!(display.object(BACK).unwrap().local_transform(), back);
  let camera = display.object(CAMERA).unwrap().camera().unwrap();
  assert_eq!(camera.projection, CameraProjection::Orthographic);
  assert_eq!(camera.orthographic_size, 7.0);
  assert_eq!(camera.clear_color, Color::WHITE);
  let light = display.object(LIGHT).unwrap().light().unwrap();
  assert_eq!(light.light_type, LightType::Spot);
  assert_eq!(light.range, 15.0);
  assert_eq!(light.inner_spot_angle, 15.0);
  assert_eq!(light.outer_spot_angle, 40.0);
  assert_eq!(light.intensity, 2.0);
  assert_eq!(display.frame(), frame);
  assert_eq!(display.presentation_time(), time);
}

#[test]
fn conditional_faces_replace_independently_and_reconnect_with_prepared_geometry() {
  let (mut display, root, mesh) = self::fixture();
  let back = display.object(BACK).unwrap().local_transform();
  let face = display.find_ui(root, "face");
  for _ in 0..3 {
    display.click_ui(face);
    assert!(display.object(FRONT).is_none());
    assert_eq!(display.object(BACK).unwrap().local_transform(), back);
    assert!(display.object(GROUP).is_some());
    display.click_ui(face);
    assert!(display.object(FRONT).is_some());
  }
  let change = display.find_ui(root, "change");
  display.click_ui(change);
  display.reconnect();
  let mesh = mesh.borrow().as_ref().unwrap().object_id().unwrap();
  assert!(
    matches!(display.object(mesh).unwrap().kind(), GameObjectKind::Mesh { address, .. } if address.as_str() == "test/mesh-b")
  );
  assert_eq!(
    display.object(mesh).unwrap().material(0).unwrap().as_str(),
    "test/material"
  );
  assert_eq!(display.object(BACK).unwrap().local_transform(), back);
}

#[test]
fn missing_mesh_fails_asset_loading_before_dependent_replacement() {
  let (mut display, root, _) = self::fixture();
  let mesh = display.object(MESH).unwrap().kind().clone();
  let missing = display.find_ui(root, "missing");
  let failure = panic::catch_unwind(AssertUnwindSafe(|| display.click_ui(missing)));
  assert!(failure.is_err());
  let error = failure.unwrap_err();
  let message = error
    .downcast_ref::<String>()
    .map(String::as_str)
    .or_else(|| error.downcast_ref::<&str>().copied())
    .unwrap();
  assert!(
    message.contains("unknown prepared asset: Mesh"),
    "{message}"
  );
  assert_eq!(display.object(MESH).unwrap().kind(), &mesh);
  assert!(display.object(FRONT).is_some());
  assert!(display.object(BACK).is_some());
}
