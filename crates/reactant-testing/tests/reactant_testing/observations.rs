//! Independent synthetic scenes verify that host queries survive harmless tree
//! changes and still expose wrong assets, duplicate objects, and misplaced pieces.
use battlement::{ParentScene, PrefabAddress, PreparedAsset, Vector3};
use reactant::{
  Application,
  world::{Group, Prefab, SceneRoot},
};
use reactant_testing::{Display, assets};

const PAWN: PrefabAddress = PrefabAddress::from_static("white/pawn");
const BISHOP: PrefabAddress = PrefabAddress::from_static("black/bishop");
const DESTINATION: Vector3 = Vector3::new(0.5, 0.0, -0.5);

fn scene(addresses: &[PrefabAddress], parent: Vector3, local: Vector3) -> Display {
  let addresses = addresses.to_vec();
  let mut display = Display::mount(
    move || {
      Application::new("scene").child(
        SceneRoot::new(ParentScene::PrimaryScene).child(
          Group::new().position(parent).child(
            addresses
              .iter()
              .map(|address| {
                Group::new()
                  .position(local)
                  .child(Prefab::at(address.clone()))
              })
              .collect::<Vec<_>>(),
          ),
        ),
      )
    },
    assets::catalog(&[
      PreparedAsset::scene("scene"),
      PreparedAsset::Prefab(PAWN),
      PreparedAsset::Prefab(BISHOP),
    ]),
  );
  display.settle();
  display
}

#[test]
fn wrapper_transforms_preserve_observed_world_positions() {
  let direct = scene(&[PAWN], Vector3::ZERO, DESTINATION);
  let wrapped = scene(&[PAWN], DESTINATION, Vector3::ZERO);
  assert_eq!(direct.prefabs(), vec![(DESTINATION, PAWN)]);
  assert_eq!(wrapped.prefabs(), direct.prefabs());
}

#[test]
fn observations_preserve_asset_identity() {
  let display = scene(&[BISHOP], DESTINATION, Vector3::ZERO);
  assert_eq!(display.prefabs(), vec![(DESTINATION, BISHOP)]);
  assert_ne!(display.prefabs(), vec![(DESTINATION, PAWN)]);
}

#[test]
fn observations_preserve_overlapping_objects() {
  let display = scene(&[PAWN, BISHOP], DESTINATION, Vector3::ZERO);
  assert_eq!(display.prefabs().len(), 2);
  assert_ne!(display.prefabs(), vec![(DESTINATION, PAWN)]);
}

#[test]
fn observations_preserve_incorrect_positions() {
  let display = scene(&[PAWN], Vector3::ZERO, Vector3::ZERO);
  assert_eq!(display.prefabs(), vec![(Vector3::ZERO, PAWN)]);
  assert_ne!(display.prefabs(), vec![(DESTINATION, PAWN)]);
}

#[test]
fn observations_preserve_missing_objects() {
  let display = scene(&[], DESTINATION, Vector3::ZERO);
  assert!(display.prefabs().is_empty());
}
