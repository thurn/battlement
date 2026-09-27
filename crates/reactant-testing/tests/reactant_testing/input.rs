//! Gesture success and rules admission are different observations. A host can move
//! a draggable object even when a missing callback prevents any game action.
use battlement::{DragMode, ParentScene, PrefabAddress, PreparedAsset, Quaternion, Vector3};
use reactant::{
  Application,
  world::{BoxHitRegion, Camera, Prefab, SceneRoot},
};
use reactant_testing::{ActionResult, Display, assets};

const PAWN: PrefabAddress = PrefabAddress::from_static("pawn");
const SOURCE: Vector3 = Vector3::new(0.5, 0.0, -2.5);
const DESTINATION: Vector3 = Vector3::new(0.5, 0.0, -0.5);

fn disconnected_drag() -> Display {
  Display::mount(
    || {
      Application::new("scene")
        .camera(|camera| {
          Camera::new()
            .perspective(60.0)
            .position(Vector3::new(0.0, 8.0, -3.75))
            .rotation(Quaternion::new(
              0.58184814,
              -0.001219943,
              0.0008727778,
              0.813296,
            ))
            .into_object(camera.object_id)
        })
        .child(
          SceneRoot::new(ParentScene::PrimaryScene).child(
            BoxHitRegion::new()
              .size(Vector3::new(0.9, 1.5, 0.9))
              .position(SOURCE)
              .draggable(DragMode::SnapToPointer)
              .child(Prefab::at(PAWN)),
          ),
        )
    },
    assets::catalog(&[PreparedAsset::scene("scene"), PreparedAsset::Prefab(PAWN)]),
  )
}

#[test]
fn moving_a_disconnected_draggable_does_not_admit_a_rules_action() {
  let mut display = disconnected_drag();
  let result =
    display.action(|display| display.drag_world(Vector3 { y: 0.6, ..SOURCE }, DESTINATION));
  assert_eq!(result, ActionResult::NoAction);
  assert_eq!(display.prefabs_at(DESTINATION), vec![PAWN]);
}
