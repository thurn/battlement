use std::{
  cell::{Cell, RefCell},
  num::NonZeroU64,
  rc::Rc,
  time::Duration,
};

use battlement::{
  GeometryGeneration, GeometryObservationBatch, GeometryObservationId, GeometryObservationResult,
  GeometryObservationTarget, GeometryObservationValue, GeometryValue, LocalTransform, ObjectId,
  ParentScene, Quaternion, Rect, Vector3, WorldRestBoundsGeometry, object_id,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{animation_controls, prelude::*, testing::App, world};
use reactant_testing::{Display, temporal::Clock};

const FIRST: ObjectId = object_id!("323a0000-0000-4000-8000-000000000001");
const SECOND: ObjectId = object_id!("323a0000-0000-4000-8000-000000000002");
const THIRD: ObjectId = object_id!("323a0000-0000-4000-8000-000000000003");
const OUTER: ObjectId = object_id!("323a0000-0000-4000-8000-000000000004");
const VISUAL: ObjectId = object_id!("323a0000-0000-4000-8000-000000000005");
const FLOOR: ObjectId = object_id!("323a0000-0000-4000-8000-000000000006");
const MEASURE_A: ObjectId = object_id!("323a0000-0000-4000-8000-000000000007");
const MEASURE_B: ObjectId = object_id!("323a0000-0000-4000-8000-000000000008");
const MEASURE_C: ObjectId = object_id!("323a0000-0000-4000-8000-000000000009");
const REVEAL: ObjectId = object_id!("323a0000-0000-4000-8000-00000000000a");
const PARENT_A: ObjectId = object_id!("323a0000-0000-4000-8000-00000000000b");
const PARENT_B: ObjectId = object_id!("323a0000-0000-4000-8000-00000000000c");

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
  Clock::advance(&mut display, Duration::from_millis(500));
  close(
    display.object(VISUAL).unwrap().local_transform().scale.x,
    1.5,
  );
  assert_eq!(display.object(FIRST).unwrap().local_transform(), first_pose);
  assert_eq!(destinations.first.latest().unwrap().transform, first_pose);
}

#[derive(Clone)]
struct MovingLayout {
  width: DisplayStore<f64>,
  destination: world::LayoutDestination,
}

impl Component for MovingLayout {
  fn render(&self) -> impl Render {
    let width = use_external_store(self.width.clone());
    let child = world::LayoutChild::new(
      self.destination.clone(),
      world::LayoutBox::new(1.0, 1.0),
      world::Group::new(),
    );
    world::SceneRoot::new(ParentScene::PrimaryScene)
      .child(world::Flex::new().extent((width, 2.0)).child(child))
  }
}

#[derive(Clone)]
struct ConfiguredMovingLayout(MovingLayout);

impl Component for ConfiguredMovingLayout {
  fn render(&self) -> impl Render {
    let width = use_external_store(self.0.width.clone());
    let layout = world::Flex::new()
      .extent((width, 2.0))
      .child(world::LayoutChild::new(
        self.0.destination.clone(),
        world::LayoutBox::new(1.0, 1.0),
        world::Group::new(),
      ));
    world::SceneRoot::new(ParentScene::PrimaryScene).child(
      MotionConfig::new(layout)
        .transition(Transition::tween().duration_secs(1.0).ease(Easing::Linear)),
    )
  }
}

#[test]
fn default_layout_movement_retargets_from_the_displayed_pose_without_a_jump() {
  let width = DisplayStore::new(4.0);
  let destination = world::LayoutDestination::new(*FIRST.as_uuid());
  let app = App::new("layout/moving-scene").ui(MovingLayout {
    width: width.clone(),
    destination,
  });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("layout/moving-scene");
  let mut display = Display::connect(app, assets);

  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    2.0,
  );
  width.set(10.0);
  display.poll();
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    2.0,
  );
  Clock::advance(&mut display, Duration::from_millis(80));
  let before_retarget = display.object(FIRST).unwrap().local_transform().position.x;
  assert!((2.0..5.0).contains(&before_retarget));

  width.set(14.0);
  display.poll();
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    before_retarget,
  );
  display.settle();
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    7.0,
  );
  assert_eq!(display.frame(), 0);
}

#[test]
fn inherited_movement_transition_overrides_the_engine_spring() {
  let width = DisplayStore::new(4.0);
  let destination = world::LayoutDestination::new(*FIRST.as_uuid());
  let app = App::new("layout/inherited-motion-scene").ui(ConfiguredMovingLayout(MovingLayout {
    width: width.clone(),
    destination,
  }));
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("layout/inherited-motion-scene");
  let mut display = Display::connect(app, assets);

  width.set(10.0);
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(500));
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    3.5,
  );
  display.settle();
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    5.0,
  );
}

#[derive(Clone)]
struct ObjectMovementOverride {
  width: DisplayStore<f64>,
  destination: world::LayoutDestination,
}

impl Component for ObjectMovementOverride {
  fn render(&self) -> impl Render {
    let width = use_external_store(self.width.clone());
    let child = world::LayoutChild::new(
      self.destination.clone(),
      world::LayoutBox::new(1.0, 1.0),
      world::Group::new(),
    )
    .movement(Transition::tween().duration_secs(1.0).ease(Easing::Linear));
    let layout = world::Flex::new()
      .extent((width, 2.0))
      .movement(Transition::tween().duration_secs(2.0).ease(Easing::Linear))
      .child(child);
    world::SceneRoot::new(ParentScene::PrimaryScene).child(layout)
  }
}

#[test]
fn object_movement_transition_overrides_its_layout_ancestor() {
  let width = DisplayStore::new(4.0);
  let destination = world::LayoutDestination::new(*FIRST.as_uuid());
  let app = App::new("layout/object-motion-scene").ui(ObjectMovementOverride {
    width: width.clone(),
    destination,
  });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("layout/object-motion-scene");
  let mut display = Display::connect(app, assets);

  width.set(10.0);
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(500));
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    3.5,
  );
}

#[derive(Clone)]
struct ReparentedLayout {
  right: DisplayStore<bool>,
  destination: world::LayoutDestination,
}

impl Component for ReparentedLayout {
  fn render(&self) -> impl Render {
    let right = use_external_store(self.right.clone());
    let left = (!right).then(|| positioned_child(self.destination.clone()));
    let right = right.then(|| positioned_child(self.destination.clone()));
    world::SceneRoot::new(ParentScene::PrimaryScene).child((
      world::Group::new()
        .id(*PARENT_A.as_uuid())
        .position(Vector3::new(-5.0, 0.0, 0.0))
        .child(left),
      world::Group::new()
        .id(*PARENT_B.as_uuid())
        .position(Vector3::new(5.0, 0.0, 0.0))
        .child(right),
    ))
  }
}

fn positioned_child(destination: world::LayoutDestination) -> world::Pile {
  world::Pile::new()
    .extent((2.0, 2.0))
    .child(world::LayoutChild::new(
      destination,
      world::LayoutBox::new(1.0, 1.0),
      world::Group::new(),
    ))
}

#[test]
fn moving_between_world_layout_parents_preserves_the_displayed_world_pose() {
  let right = DisplayStore::new(false);
  let destination = world::LayoutDestination::new(*FIRST.as_uuid());
  let app = App::new("layout/reparent-scene").ui(ReparentedLayout {
    right: right.clone(),
    destination,
  });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("layout/reparent-scene");
  let mut display = Display::connect(app, assets);

  assert_eq!(
    display.world_point(FIRST, Vector3::ZERO),
    Vector3::new(-4.0, 1.0, 0.0)
  );
  right.set(true);
  display.poll();
  assert_eq!(
    display.world_point(FIRST, Vector3::ZERO),
    Vector3::new(-4.0, 1.0, 0.0)
  );
  Clock::advance(&mut display, Duration::from_secs(2));
  let point = display.world_point(FIRST, Vector3::ZERO);
  close(point.x, 6.0);
  close(point.y, 1.0);
  close(point.z, 0.0);
}

#[derive(Clone, Default)]
struct SequenceProbe(Rc<RefCell<Option<AnimationScope>>>);

#[derive(Clone)]
struct SequencedLayout {
  width: DisplayStore<f64>,
  destination: world::LayoutDestination,
  probe: SequenceProbe,
}

impl Component for SequencedLayout {
  fn render(&self) -> impl Render {
    let width = use_external_store(self.width.clone());
    let scope = animation_controls::use_animation_scope();
    self.probe.0.replace(Some(scope.clone()));
    let layout = world::Flex::new()
      .extent((width, 2.0))
      .child(world::LayoutChild::new(
        self.destination.clone(),
        world::LayoutBox::new(1.0, 1.0),
        world::Group::new(),
      ));
    world::SceneRoot::new(ParentScene::PrimaryScene).child(
      world::Group::new()
        .child((
          world::Group::new()
            .id(*REVEAL.as_uuid())
            .position(Vector3::new(-4.0, 1.0, 0.0))
            .animate(StyleTarget::new()),
          layout,
        ))
        .motion(MotionProps::new().animation_scope(scope)),
    )
  }
}

#[test]
fn sequence_keeps_its_anchor_while_layout_reflows_then_arrives_at_the_live_destination() {
  let width = DisplayStore::new(4.0);
  let destination = world::LayoutDestination::new(*FIRST.as_uuid());
  let probe = SequenceProbe::default();
  let app = App::new("layout/sequence-scene").ui(SequencedLayout {
    width: width.clone(),
    destination: destination.clone(),
    probe: probe.clone(),
  });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("layout/sequence-scene");
  let mut display = Display::connect(app, assets);
  let scope = probe.0.borrow().as_ref().unwrap().clone();
  let playback = scope.start(
    AnimationSequence::new()
      .animate(
        destination.selector(),
        SequenceTarget::new(StyleTarget::new()).position(MotionPositionRef::identified(REVEAL)),
        Transition::tween().duration_secs(0.4).ease(Easing::Linear),
      )
      .then(
        destination.selector(),
        SequenceTarget::new(StyleTarget::new()).position(destination.clone()),
        Transition::tween().duration_secs(1.0).ease(Easing::Linear),
      ),
  );
  let completed = Rc::new(Cell::new(0));
  let count = completed.clone();
  playback.on_complete(move || count.set(count.get() + 1));
  display.poll();

  Clock::advance(&mut display, Duration::from_millis(200));
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    -1.0,
  );
  width.set(16.0);
  display.poll();
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    -1.0,
  );
  Clock::advance(&mut display, Duration::from_millis(200));
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    -4.0,
  );

  Clock::advance(&mut display, Duration::from_millis(500));
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    2.0,
  );
  width.set(20.0);
  display.poll();
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    2.0,
  );
  assert_eq!(completed.get(), 0);
  Clock::advance(&mut display, Duration::from_millis(999));
  assert_eq!(completed.get(), 0);
  Clock::advance(&mut display, Duration::from_millis(1));
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    10.0,
  );
  assert_eq!(completed.get(), 1);
  Clock::advance(&mut display, Duration::from_secs(1));
  close(
    display.object(FIRST).unwrap().local_transform().position.x,
    10.0,
  );
  assert_eq!(completed.get(), 1);
}

#[derive(Clone)]
struct MeasuredLayout {
  request: DisplayStore<world::LayoutMeasurement>,
  first: world::LayoutDestination,
  second: world::LayoutDestination,
}

impl Component for MeasuredLayout {
  fn render(&self) -> impl Render {
    let request = use_external_store(self.request.clone());
    let transition = Transition::tween().duration_secs(1.0).ease(Easing::Linear);
    world::Flex::new().extent((10.0, 4.0)).gap(1.0).children([
      world::LayoutChild::measured(
        self.first.clone(),
        request,
        world::Group::new()
          .id(*VISUAL.as_uuid())
          .initial(StyleTarget::new().local_scale_x(1.0))
          .animate(StyleTarget::new().local_scale_x(2.0))
          .transition(transition),
      ),
      world::LayoutChild::new(
        self.second.clone(),
        world::LayoutBox::new(2.0, 2.0),
        world::Group::new(),
      ),
    ])
  }
}

struct UnrelatedLayout;

impl Component for UnrelatedLayout {
  fn render(&self) -> impl Render {
    world::Pile::new()
      .extent((2.0, 2.0))
      .child(world::LayoutChild::new(
        world::LayoutDestination::new(*THIRD.as_uuid()),
        world::LayoutBox::new(1.0, 1.0),
        world::Group::new(),
      ))
  }
}

#[test]
fn public_display_caches_identified_rest_measurements_and_rejects_stale_results() {
  let request = DisplayStore::new(world::LayoutMeasurement::identified(*MEASURE_A.as_uuid()));
  let first = world::LayoutDestination::new(*FIRST.as_uuid());
  let second = world::LayoutDestination::new(*SECOND.as_uuid());
  let app =
    App::new("layout/measured-scene").ui(world::SceneRoot::new(ParentScene::PrimaryScene).child((
      MeasuredLayout {
        request: request.clone(),
        first: first.clone(),
        second: second.clone(),
      },
      UnrelatedLayout,
    )));
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("layout/measured-scene");
  let mut display = Display::connect(app, assets);

  assert!(first.latest().is_none());
  assert_eq!(
    display.object(FIRST).unwrap().local_transform().position,
    Vector3::ZERO
  );
  let first_observation = rest_observation(&display, MEASURE_A);
  display.clear_commands();
  display.deliver_geometry(rest_batch(1, first_observation, 2.0, 2.0));
  assert_eq!(
    display.object(SECOND).unwrap().local_transform().position,
    Vector3::new(6.5, 2.0, 0.0)
  );
  assert!(!changed_position(&display, THIRD));
  assert!(
    display
      .geometry_registry()
      .iter()
      .all(|(_, target)| !matches!(
        target,
        GeometryObservationTarget::WorldRestBounds { request_id, .. } if *request_id == MEASURE_A
      ))
  );

  let held = display.object(SECOND).unwrap().local_transform();
  Clock::advance(&mut display, Duration::from_millis(500));
  close(
    display.object(VISUAL).unwrap().local_transform().scale.x,
    1.5,
  );
  assert_eq!(display.object(SECOND).unwrap().local_transform(), held);
  assert!(!changed_position(&display, THIRD));

  request.set(world::LayoutMeasurement::identified(*MEASURE_B.as_uuid()));
  display.poll();
  let stale_observation = rest_observation(&display, MEASURE_B);
  assert_eq!(display.object(SECOND).unwrap().local_transform(), held);

  request.set(world::LayoutMeasurement::identified(*MEASURE_C.as_uuid()));
  display.deliver_geometry(rest_batch(2, stale_observation, 8.0, 2.0));
  assert_eq!(display.object(SECOND).unwrap().local_transform(), held);
  assert!(!changed_position(&display, THIRD));
  let current_observation = rest_observation(&display, MEASURE_C);

  display.deliver_geometry(rest_batch(3, current_observation, 4.0, 2.0));
  assert_eq!(display.object(SECOND).unwrap().local_transform(), held);
  display.settle();
  assert_eq!(
    display.object(SECOND).unwrap().local_transform().position,
    Vector3::new(7.5, 2.0, 0.0)
  );
  assert!(!changed_position(&display, THIRD));
}

fn changed_position<E>(display: &Display<E>, object_id: ObjectId) -> bool
where
  E: battlement_native::Engine,
{
  display.commands().iter().any(|executed| {
    matches!(
      &executed.command.body,
      battlement::CommandBody::TransformSetLocalPosition(command)
        if command.payload.object_id == object_id
    )
  })
}

fn rest_observation<E>(display: &Display<E>, request_id: ObjectId) -> GeometryObservationId
where
  E: battlement_native::Engine,
{
  display
    .geometry_registry()
    .iter()
    .find_map(|(observation_id, target)| match target {
      GeometryObservationTarget::WorldRestBounds {
        request_id: current,
        ..
      } if *current == request_id => Some(*observation_id),
      _ => None,
    })
    .unwrap_or_else(|| panic!("missing rest measurement request {request_id}"))
}

fn rest_batch(
  generation: u64,
  observation_id: GeometryObservationId,
  width: f64,
  height: f64,
) -> GeometryObservationBatch {
  GeometryObservationBatch {
    generation: GeometryGeneration(NonZeroU64::new(generation).unwrap()),
    changed: vec![GeometryObservationValue {
      observation_id,
      result: GeometryObservationResult::Current(GeometryValue::WorldRestBounds(
        WorldRestBoundsGeometry {
          bound: Rect::new(-width / 2.0, -height / 2.0, width, height),
        },
      )),
    }],
  }
}
