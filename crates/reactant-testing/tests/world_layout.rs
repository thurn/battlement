use std::time::Duration;

use battlement::{LocalTransform, ObjectId, ParentScene, Quaternion, Vector3, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{app::App, prelude::*, world};
use reactant_testing::Display;

const FIRST: ObjectId = object_id!("323a0000-0000-4000-8000-000000000001");
const SECOND: ObjectId = object_id!("323a0000-0000-4000-8000-000000000002");
const THIRD: ObjectId = object_id!("323a0000-0000-4000-8000-000000000003");
const OUTER: ObjectId = object_id!("323a0000-0000-4000-8000-000000000004");
const VISUAL: ObjectId = object_id!("323a0000-0000-4000-8000-000000000005");
const FLOOR: ObjectId = object_id!("323a0000-0000-4000-8000-000000000006");

fn item(id: ObjectId, width: f64, height: f64) -> world::LayoutItem {
  world::LayoutItem::new(*id.as_uuid(), world::LayoutBox::new(width, height))
}

fn close(actual: f64, expected: f64) {
  assert!(
    (actual - expected).abs() < 0.00001,
    "{actual} != {expected}"
  );
}

fn position(targets: &[world::LayoutTarget], index: usize) -> Vector3 {
  targets[index].transform.position
}

#[derive(Clone)]
struct Diagonal;

impl world::LayoutAlgorithm for Diagonal {
  fn placements(
    &self,
    _extent: world::LayoutExtent,
    items: &[world::LayoutItem],
  ) -> Vec<world::LayoutPlacement> {
    items
      .iter()
      .enumerate()
      .map(|(index, item)| world::LayoutPlacement {
        x: index as f64,
        y: index as f64,
        width: item.rest.width,
        height: item.rest.height,
        angle: 0.0,
      })
      .collect()
  }
}

#[test]
fn built_in_algorithms_produce_known_ordered_targets() {
  let items = [
    item(FIRST, 2.0, 2.0),
    item(SECOND, 2.0, 2.0),
    item(THIRD, 2.0, 2.0),
  ];

  let flex = world::Flex::new()
    .extent((10.0, 4.0))
    .gap(1.0)
    .targets(&items[..2]);
  assert_eq!(position(&flex, 0), Vector3::new(3.5, 2.0, 0.0));
  assert_eq!(position(&flex, 1), Vector3::new(6.5, 2.0, 0.0));

  let grid = world::Grid::new()
    .extent((10.0, 6.0))
    .columns(2)
    .gaps(2.0, 0.0)
    .align(world::LayoutAlignment::Center)
    .targets(&items);
  assert_eq!(position(&grid, 0), Vector3::new(2.0, 4.5, 0.0));
  assert_eq!(position(&grid, 1), Vector3::new(8.0, 4.5, 0.0));
  assert_eq!(position(&grid, 2), Vector3::new(2.0, 1.5, 0.0));

  let fan = world::Fan::new()
    .extent((10.0, 6.0))
    .curve(6.0, 2.0)
    .angle(1.0)
    .targets(&items);
  assert_eq!(position(&fan, 0), Vector3::new(2.0, 3.0, 0.0));
  assert_eq!(position(&fan, 1), Vector3::new(5.0, 5.0, 0.0));
  assert_eq!(position(&fan, 2), Vector3::new(8.0, 3.0, 0.0));

  let pile = world::Pile::new()
    .extent((10.0, 6.0))
    .step(1.0, -0.5)
    .targets(&items);
  assert_eq!(position(&pile, 0), Vector3::new(4.0, 3.5, 0.0));
  assert_eq!(position(&pile, 1), Vector3::new(5.0, 3.0, 0.0));
  assert_eq!(position(&pile, 2), Vector3::new(6.0, 2.5, 0.0));

  let arc = world::Arc::new()
    .extent((10.0, 6.0))
    .angles(0.0, std::f64::consts::PI)
    .radii(4.0, 2.0)
    .targets(&items);
  assert_eq!(position(&arc, 0), Vector3::new(9.0, 3.0, 0.0));
  close(position(&arc, 1).x, 5.0);
  close(position(&arc, 1).y, 5.0);
  close(position(&arc, 2).x, 1.0);
  close(position(&arc, 2).y, 3.0);

  assert_eq!(
    world::Fan::new()
      .extent((10.0, 6.0))
      .curve(6.0, 2.0)
      .angle(1.0)
      .targets(&items),
    fan
  );

  assert_eq!(
    position(
      &world::Fan::new()
        .extent((10.0, 6.0))
        .curve(6.0, 2.0)
        .targets(&items[..1]),
      0
    ),
    Vector3::new(5.0, 5.0, 0.0)
  );
  let single_arc = world::Arc::new()
    .extent((10.0, 6.0))
    .angles(0.0, std::f64::consts::PI)
    .radii(4.0, 2.0)
    .targets(&items[..1]);
  close(position(&single_arc, 0).x, 5.0);
  close(position(&single_arc, 0).y, 5.0);
}

#[test]
fn application_algorithms_use_the_same_pure_target_mapping() {
  let targets = world::WorldLayout::custom(Diagonal)
    .extent((4.0, 4.0))
    .targets(&[item(FIRST, 1.0, 1.0), item(SECOND, 1.0, 1.0)]);
  assert_eq!(position(&targets, 0), Vector3::ZERO);
  assert_eq!(position(&targets, 1), Vector3::new(1.0, 1.0, 0.0));
}

#[test]
fn explicit_plane_geometry_rules_and_rest_boxes_are_independent() {
  let authored = LocalTransform {
    position: Vector3::new(99.0, 99.0, 99.0),
    rotation: Quaternion::new(0.0, 0.0, 1.0, 0.0),
    scale: Vector3::new(2.0, 3.0, 4.0),
  };
  let plane = world::LayoutPlane::new(
    Vector3::new(10.0, 20.0, 30.0),
    Vector3::new(1.0, 0.0, 0.0),
    Vector3::new(0.0, 0.0, 1.0),
  );
  let preserved = item(FIRST, 2.0, 1.0).authored(authored).depth(2.0);
  let aligned = item(SECOND, 2.0, 1.0)
    .authored(authored)
    .orientation(world::LayoutOrientation::Arrangement)
    .scaling(world::LayoutScaling::Fit);
  let targets = world::Grid::new()
    .plane(plane)
    .extent((8.0, 4.0))
    .algorithm(world::GridLayout {
      columns: 2,
      align: world::LayoutAlignment::Stretch,
      ..world::GridLayout::default()
    })
    .targets(&[preserved, aligned]);

  assert_eq!(
    targets[0].transform.position,
    Vector3::new(11.0, 18.0, 33.5)
  );
  assert_eq!(targets[0].transform.rotation, authored.rotation);
  assert_eq!(targets[0].transform.scale, authored.scale);
  assert_ne!(targets[1].transform.rotation, authored.rotation);
  assert_eq!(targets[1].transform.scale, Vector3::new(4.0, 12.0, 4.0));

  let small_scale = item(FIRST, 2.0, 1.0).authored(LocalTransform {
    scale: Vector3::new(0.5, 0.5, 0.5),
    ..LocalTransform::default()
  });
  let large_scale = item(FIRST, 2.0, 1.0).authored(LocalTransform {
    scale: Vector3::new(5.0, 5.0, 5.0),
    ..LocalTransform::default()
  });
  let layout = world::Flex::new().extent((8.0, 4.0));
  assert_eq!(
    layout.targets(&[small_scale])[0].transform.position,
    layout.targets(&[large_scale])[0].transform.position
  );
  assert_ne!(
    layout.targets(&[item(FIRST, 2.0, 1.0), item(SECOND, 2.0, 1.0)])[1]
      .transform
      .position
      .x,
    layout.targets(&[item(FIRST, 4.0, 1.0), item(SECOND, 2.0, 1.0)])[1]
      .transform
      .position
      .x
  );
}

#[derive(Clone)]
struct Destinations {
  outer: world::LayoutDestination,
  first: world::LayoutDestination,
  second: world::LayoutDestination,
  floor: world::LayoutDestination,
}

struct NestedScene(Destinations);

impl Component for NestedScene {
  fn render(&self) -> impl Render {
    let transition = Transition::tween().duration_secs(1.0).ease(Easing::Linear);
    let inner = world::Pile::new()
      .extent((6.0, 4.0))
      .algorithm(world::PileLayout {
        step_x: 2.0,
        step_y: 0.0,
      })
      .children([
        world::LayoutChild::new(
          self.0.first.clone(),
          world::LayoutBox::new(1.0, 2.0),
          world::Group::new()
            .id(*VISUAL.as_uuid())
            .initial(StyleTarget::new().local_scale_x(1.0))
            .animate(StyleTarget::new().local_scale_x(2.0))
            .transition(transition),
        ),
        world::LayoutChild::new(
          self.0.second.clone(),
          world::LayoutBox::new(1.0, 2.0),
          world::Group::new(),
        ),
      ]);
    let table = world::Grid::new()
      .extent((10.0, 6.0))
      .child(world::LayoutChild::new(
        self.0.outer.clone(),
        world::LayoutBox::new(6.0, 4.0),
        inner,
      ));
    let floor = world::Pile::new()
      .plane(world::LayoutPlane::new(
        Vector3::new(10.0, 20.0, 30.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
      ))
      .extent((8.0, 4.0))
      .child(world::LayoutChild::new(
        self.0.floor.clone(),
        world::LayoutBox::new(2.0, 2.0),
        world::Group::new(),
      ));
    world::SceneRoot::new(ParentScene::PrimaryScene).child((table, floor))
  }
}

#[test]
fn public_display_keeps_nested_targets_stable_while_visual_scale_animates() {
  let destinations = Destinations {
    outer: world::LayoutDestination::new(*OUTER.as_uuid()),
    first: world::LayoutDestination::new(*FIRST.as_uuid()),
    second: world::LayoutDestination::new(*SECOND.as_uuid()),
    floor: world::LayoutDestination::new(*FLOOR.as_uuid()),
  };
  let app = App::new("layout/scene").ui(NestedScene(destinations.clone()));
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("layout/scene");
  let mut display = Display::connect(app, assets);

  assert_eq!(
    display.object(OUTER).unwrap().local_transform().position,
    Vector3::new(5.0, 3.0, 0.0)
  );
  assert_eq!(
    display.object(FIRST).unwrap().local_transform().position,
    Vector3::new(2.0, 2.0, 0.0)
  );
  assert_eq!(
    display.object(SECOND).unwrap().local_transform().position,
    Vector3::new(4.0, 2.0, 0.0)
  );
  assert_eq!(
    display.object(FLOOR).unwrap().local_transform().position,
    Vector3::new(14.0, 20.0, 32.0)
  );
  assert_eq!(
    destinations.first.latest().unwrap().transform,
    display.object(FIRST).unwrap().local_transform()
  );

  let first_pose = display.object(FIRST).unwrap().local_transform();
  display.advance_time(Duration::from_millis(500));
  close(
    display.object(VISUAL).unwrap().local_transform().scale.x,
    1.5,
  );
  assert_eq!(display.object(FIRST).unwrap().local_transform(), first_pose);
  assert_eq!(destinations.first.latest().unwrap().transform, first_pose);
}
