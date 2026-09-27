use battlement::{ObjectId, ParentScene, Vector3};
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use reactant::{
  host::{ButtonHost, ToggleButtonGroup},
  prelude::*,
  testing::App,
};
use reactant_testing::Display;
use trox::ls;

#[test]
fn a_retained_ui_host_moves_beneath_a_new_parent_in_a_full_choice_group() {
  let id = ObjectId::new_v4();
  let app = App::with_model("identity/scene", false).root(move |changed| {
    (
      ButtonHost::new(ls("Replace parent"))
        .name("replace")
        .on_click(|changed: &mut bool| *changed = true),
      ToggleButtonGroup::new().child(
        (0..64_u32)
          .map(|index| {
            ButtonHost::new(ls(format!("Choice {index}")))
              .key((index, index == 0 && *changed))
              .child((index == 0).then(|| View::new().id(*id.as_uuid()).name("survivor")))
          })
          .collect::<Vec<_>>(),
      ),
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("identity/scene");
  let mut display = Display::connect(app, assets);
  let survivor = display.find_ui(root, "survivor");
  display.click_ui(display.find_ui(root, "replace"));
  assert_eq!(display.find_ui(root, "survivor"), survivor);
}

#[test]
fn a_world_host_keeps_its_handle_across_new_ancestors_and_scene_attachments() {
  let id = ObjectId::new_v4();
  let app = App::with_model("identity/scene", false).root(move |changed| {
    (
      ButtonHost::new(ls("Move world"))
        .name("move")
        .on_click(|changed: &mut bool| *changed = !*changed),
      SceneRoot::new(if *changed {
        ParentScene::Persistent
      } else {
        ParentScene::PrimaryScene
      })
      .child(
        Prefab::at(if *changed {
          "identity/second"
        } else {
          "identity/first"
        })
        .child(
          WorldGroup::new()
            .id(*id.as_uuid())
            .position(Vector3::new(2.0, 3.0, 4.0)),
        ),
      ),
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("identity/scene");
  assets.add_prefab("identity/first", FakePrefab::new());
  assets.add_prefab("identity/second", FakePrefab::new());
  let mut display = Display::connect(app, assets);
  let original = display.object(id).unwrap().parent_id();
  for changed in [true, false] {
    display.click_ui(display.find_ui(root, "move"));
    let world = display.object(id).unwrap();
    assert_ne!(world.parent_id(), original);
    assert_eq!(world.scene_id().is_none(), changed);
    assert_eq!(
      world.local_transform().position,
      Vector3::new(2.0, 3.0, 4.0)
    );
    assert_eq!(
      display.with_engine(|app| app.presentation(*id.as_uuid()).unwrap().native_objects),
      [id]
    );
  }
}
