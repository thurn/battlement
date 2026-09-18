//! Deterministic world-space layout from explicit rest geometry.

use std::{cell::RefCell, rc::Rc};

use battlement::{LocalTransform, ObjectId, Vector3};
use reactant_core::{
  animation_controls::{MotionPositionRef, MotionSelector},
  component::Component,
  geometry::{self, MeasurementStatus, WorldGeometry, WorldRef},
  hooks,
  motion::{MotionProps, StyleTarget, Transition},
  motion_config::MotionConfig,
  prelude::MotionComponent,
  render::{Child, Node, Render},
};
use uuid::Uuid;

use crate::{
  world::Group,
  world_layout_algorithms::{map_target, validate_extent, validate_item, validate_plane},
};

pub use crate::world_layout_builders::{Arc, Fan, Flex, Grid, Pile};

/// A rectangular world-layout extent in plane units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutExtent {
  /// Width along the plane X axis.
  pub width: f64,
  /// Height along the plane Y axis.
  pub height: f64,
}

impl LayoutExtent {
  /// Creates a positive finite extent.
  #[must_use]
  pub fn new(width: f64, height: f64) -> Self {
    let extent = Self { width, height };
    validate_extent(extent);
    extent
  }
}

impl From<(f64, f64)> for LayoutExtent {
  fn from(value: (f64, f64)) -> Self {
    Self::new(value.0, value.1)
  }
}

/// A right-handed plane whose origin is the lower-left layout corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutPlane {
  /// Lower-left point of the layout extent.
  pub origin: Vector3,
  /// Unit vector corresponding to increasing layout X.
  pub x_axis: Vector3,
  /// Unit vector corresponding to increasing layout Y.
  pub y_axis: Vector3,
}

impl LayoutPlane {
  /// Creates a plane from orthogonal unit axes.
  #[must_use]
  pub fn new(origin: Vector3, x_axis: Vector3, y_axis: Vector3) -> Self {
    let plane = Self {
      origin,
      x_axis,
      y_axis,
    };
    validate_plane(plane);
    plane
  }

  /// Creates an XY plane facing negative Z, matching ordinary Unity sprites.
  #[must_use]
  pub const fn xy(origin: Vector3) -> Self {
    Self {
      origin,
      x_axis: Vector3::new(1.0, 0.0, 0.0),
      y_axis: Vector3::new(0.0, 1.0, 0.0),
    }
  }
}

/// Stable authored or rest-metadata geometry used by layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutBox {
  /// Rest width in world units.
  pub width: f64,
  /// Rest height in world units.
  pub height: f64,
  /// Normalized horizontal pivot from the left edge.
  pub pivot_x: f64,
  /// Normalized vertical pivot from the bottom edge.
  pub pivot_y: f64,
}

impl LayoutBox {
  /// Creates a centered rest box.
  #[must_use]
  pub fn new(width: f64, height: f64) -> Self {
    Self::with_pivot(width, height, 0.5, 0.5)
  }

  /// Creates a rest box with an explicit normalized pivot.
  #[must_use]
  pub fn with_pivot(width: f64, height: f64, pivot_x: f64, pivot_y: f64) -> Self {
    let value = Self {
      width,
      height,
      pivot_x,
      pivot_y,
    };
    assert!(
      width.is_finite() && width > 0.0 && width <= f32::MAX.into(),
      "layout box width must be positive and finite"
    );
    assert!(
      height.is_finite() && height > 0.0 && height <= f32::MAX.into(),
      "layout box height must be positive and finite"
    );
    assert!(
      (0.0..=1.0).contains(&pivot_x),
      "layout pivot X must be between zero and one"
    );
    assert!(
      (0.0..=1.0).contains(&pivot_y),
      "layout pivot Y must be between zero and one"
    );
    value
  }
}

/// How layout changes authored geometry orientation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LayoutOrientation {
  /// Keeps the authored rotation exactly.
  #[default]
  Preserve,
  /// Aligns local XY to the declared plane but ignores arrangement rotation.
  Plane,
  /// Aligns local XY to the plane and follows the arrangement's in-plane angle.
  Arrangement,
}

/// How layout changes authored geometry scale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LayoutScaling {
  /// Keeps the authored scale exactly.
  #[default]
  Preserve,
  /// Multiplies authored X/Y scale to fill the allocated layout rectangle.
  Fit,
}

/// One pure layout input, independent of its rendered content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutItem {
  /// Stable presentation identity for the destination.
  pub id: Uuid,
  /// Authored or rest-metadata box.
  pub rest: LayoutBox,
  /// Authored rotation and scale preserved by default.
  pub authored: LocalTransform,
  /// Signed distance along the plane normal.
  pub depth: f64,
  /// Flex growth factor.
  pub flex_grow: f32,
  /// Flex shrink factor.
  pub flex_shrink: f32,
  /// Number of grid columns occupied.
  pub column_span: u16,
  /// Number of grid rows occupied.
  pub row_span: u16,
  /// Orientation rule for the target pose.
  pub orientation: LayoutOrientation,
  /// Scaling rule for the target pose.
  pub scaling: LayoutScaling,
}

impl LayoutItem {
  /// Creates an item that preserves authored facing and scale.
  #[must_use]
  pub fn new(id: Uuid, rest: LayoutBox) -> Self {
    assert!(!id.is_nil(), "layout item IDs cannot be nil");
    Self {
      id,
      rest,
      authored: LocalTransform::default(),
      depth: 0.0,
      flex_grow: 0.0,
      flex_shrink: 1.0,
      column_span: 1,
      row_span: 1,
      orientation: LayoutOrientation::Preserve,
      scaling: LayoutScaling::Preserve,
    }
  }

  /// Sets authored rotation and scale; authored position is ignored by layout.
  #[must_use]
  pub fn authored(mut self, transform: LocalTransform) -> Self {
    self.authored = transform;
    self
  }

  /// Offsets the target along the plane normal.
  #[must_use]
  pub fn depth(mut self, depth: f64) -> Self {
    assert!(depth.is_finite(), "layout depth must be finite");
    self.depth = depth;
    self
  }

  /// Sets Taffy Flexbox grow and shrink factors.
  #[must_use]
  pub fn flex(mut self, grow: f32, shrink: f32) -> Self {
    assert!(
      grow.is_finite() && grow >= 0.0,
      "flex grow must be finite and nonnegative"
    );
    assert!(
      shrink.is_finite() && shrink >= 0.0,
      "flex shrink must be finite and nonnegative"
    );
    self.flex_grow = grow;
    self.flex_shrink = shrink;
    self
  }

  /// Sets positive Taffy Grid spans.
  #[must_use]
  pub fn grid_span(mut self, columns: u16, rows: u16) -> Self {
    assert!(columns > 0 && rows > 0, "grid spans must be positive");
    self.column_span = columns;
    self.row_span = rows;
    self
  }

  /// Sets the explicit geometry orientation rule.
  #[must_use]
  pub const fn orientation(mut self, orientation: LayoutOrientation) -> Self {
    self.orientation = orientation;
    self
  }

  /// Sets the explicit geometry scaling rule.
  #[must_use]
  pub const fn scaling(mut self, scaling: LayoutScaling) -> Self {
    self.scaling = scaling;
    self
  }
}

/// A pure two-dimensional result before it is mapped into a plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutPlacement {
  /// Pivot position from the plane's lower-left origin.
  pub x: f64,
  /// Pivot position from the plane's lower-left origin.
  pub y: f64,
  /// Allocated width used only by [`LayoutScaling::Fit`].
  pub width: f64,
  /// Allocated height used only by [`LayoutScaling::Fit`].
  pub height: f64,
  /// Counter-clockwise in-plane angle in radians.
  pub angle: f64,
}

/// A computed world-space target pose for one identified child.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutTarget {
  /// Stable child identity.
  pub id: Uuid,
  /// Target local transform relative to the layout's parent.
  pub transform: LocalTransform,
}

/// Pure ordered layout algorithm shared by built-in and application layouts.
pub trait LayoutAlgorithm: Clone + 'static {
  /// Computes exactly one placement per ordered item.
  fn placements(&self, extent: LayoutExtent, items: &[LayoutItem]) -> Vec<LayoutPlacement>;
}

/// A cloneable reference to one child's latest computed target.
#[derive(Clone)]
pub struct LayoutDestination {
  id: Uuid,
  target_id: ObjectId,
  state: Rc<RefCell<LayoutDestinationState>>,
}

#[derive(Default)]
struct LayoutDestinationState {
  latest: Option<LayoutTarget>,
  committed: Option<LayoutTarget>,
  layout: Option<ObjectId>,
  base: Option<LocalTransform>,
  moved: bool,
}

/// Stable identity for one optional native rest-bounds request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayoutMeasurement {
  request_id: ObjectId,
}

impl LayoutMeasurement {
  /// Creates a fresh request identity.
  #[must_use]
  pub fn new() -> Self {
    Self {
      request_id: ObjectId::new_v4(),
    }
  }

  /// Creates a request with an application-selected non-nil identity.
  #[must_use]
  pub fn identified(request_id: Uuid) -> Self {
    Self {
      request_id: ObjectId::from_uuid(request_id)
        .expect("layout measurement identities cannot be nil"),
    }
  }

  /// Returns the request identity used to reject late measurements.
  #[must_use]
  pub const fn id(self) -> ObjectId {
    self.request_id
  }
}

impl Default for LayoutMeasurement {
  fn default() -> Self {
    Self::new()
  }
}

impl LayoutDestination {
  /// Creates a destination associated with a stable presentation ID.
  #[must_use]
  pub fn new(id: Uuid) -> Self {
    assert!(!id.is_nil(), "layout destination IDs cannot be nil");
    Self {
      id,
      target_id: ObjectId::new_v4(),
      state: Rc::new(RefCell::new(LayoutDestinationState::default())),
    }
  }

  /// Returns the stable presentation ID used by the layout wrapper.
  #[must_use]
  pub const fn id(&self) -> Uuid {
    self.id
  }

  /// Returns the most recently rendered target, if any.
  #[must_use]
  pub fn latest(&self) -> Option<LayoutTarget> {
    self.state.borrow().latest
  }

  /// Selects the identified object which moves toward this destination.
  #[must_use]
  pub fn selector(&self) -> MotionSelector {
    MotionSelector::identified(
      ObjectId::from_uuid(self.id).expect("layout destination identities cannot be nil"),
    )
  }

  /// Captures the current native destination when a sequence step starts.
  #[must_use]
  pub fn capture_at_start(&self) -> MotionPositionRef {
    MotionPositionRef::identified(self.target_id).capture_at_start()
  }

  /// Follows the latest native destination while a sequence step is active.
  #[must_use]
  pub fn follow(&self) -> MotionPositionRef {
    MotionPositionRef::identified(self.target_id).follow()
  }

  fn update(&self, target: LayoutTarget) {
    self.state.borrow_mut().latest = Some(target);
  }

  fn moving(&self, layout: ObjectId, target: LayoutTarget) -> bool {
    let state = self.state.borrow();
    state.moved
      || state
        .committed
        .is_some_and(|previous| previous != target || state.layout != Some(layout))
  }

  fn base(&self, target: LayoutTarget) -> LocalTransform {
    self.state.borrow().base.unwrap_or(target.transform)
  }

  fn commit(&self, layout: ObjectId, target: LayoutTarget) {
    let mut state = self.state.borrow_mut();
    state.moved |= state
      .committed
      .is_some_and(|previous| previous != target || state.layout != Some(layout));
    state.base.get_or_insert(target.transform);
    state.committed = Some(target);
    state.layout = Some(layout);
  }
}

/// Rendered content and stable rest geometry for one layout item.
#[derive(Clone)]
pub struct LayoutChild {
  item: LayoutItem,
  destination: LayoutDestination,
  measurement: Option<LayoutMeasurement>,
  content: Child,
  movement: Option<Transition>,
}

impl LayoutChild {
  /// Creates a child whose wrapper owns the destination's presentation ID.
  #[must_use]
  pub fn new(destination: LayoutDestination, rest: LayoutBox, content: impl Render) -> Self {
    Self {
      item: LayoutItem::new(destination.id(), rest),
      destination,
      measurement: None,
      content: Child::new(content),
      movement: None,
    }
  }

  /// Creates a child whose rest box is sampled once for the identified request.
  #[must_use]
  pub fn measured(
    destination: LayoutDestination,
    measurement: LayoutMeasurement,
    content: impl Render,
  ) -> Self {
    Self {
      item: LayoutItem::new(destination.id(), LayoutBox::new(1.0, 1.0)),
      destination,
      measurement: Some(measurement),
      content: Child::new(content),
      movement: None,
    }
  }

  /// Replaces the pure layout parameters for this destination.
  #[must_use]
  pub fn item(mut self, item: LayoutItem) -> Self {
    assert_eq!(
      item.id,
      self.destination.id(),
      "layout item and destination IDs must match"
    );
    self.item = item;
    self
  }

  /// Overrides movement timing for this destination and its logical descendants.
  #[must_use]
  pub fn movement(mut self, transition: Transition) -> Self {
    self.movement = Some(transition);
    self
  }
}

/// Horizontal or vertical Taffy Flexbox flow.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlexDirection {
  /// Places children from left to right.
  #[default]
  Row,
  /// Places children from bottom to top.
  Column,
}

/// Main-axis or cross-axis alignment.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LayoutAlignment {
  /// Aligns at the low edge.
  Start,
  /// Centers within available space.
  #[default]
  Center,
  /// Aligns at the high edge.
  End,
  /// Stretches the allocated rectangle on the cross axis.
  Stretch,
  /// Distributes free space between children; valid only on a main axis.
  SpaceBetween,
}

/// Parameters for Taffy Flexbox layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexLayout {
  /// Flow direction.
  pub direction: FlexDirection,
  /// Gap between adjacent children.
  pub gap: f64,
  /// Main-axis distribution.
  pub justify: LayoutAlignment,
  /// Cross-axis alignment.
  pub align: LayoutAlignment,
}

impl Default for FlexLayout {
  fn default() -> Self {
    Self {
      direction: FlexDirection::Row,
      gap: 0.0,
      justify: LayoutAlignment::Center,
      align: LayoutAlignment::Center,
    }
  }
}

/// Parameters for sequential Taffy Grid layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridLayout {
  /// Number of equal-width columns.
  pub columns: u16,
  /// Horizontal gap.
  pub column_gap: f64,
  /// Vertical gap.
  pub row_gap: f64,
  /// Alignment within each cell.
  pub align: LayoutAlignment,
}

impl Default for GridLayout {
  fn default() -> Self {
    Self {
      columns: 1,
      column_gap: 0.0,
      row_gap: 0.0,
      align: LayoutAlignment::Center,
    }
  }
}

/// Parameters for an overlapping fan centered in the extent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FanLayout {
  /// Total horizontal pivot spread.
  pub spread: f64,
  /// Height added at the center of the fan.
  pub rise: f64,
  /// Total in-plane angle spread in radians.
  pub angle: f64,
}

impl Default for FanLayout {
  fn default() -> Self {
    Self {
      spread: 0.0,
      rise: 0.0,
      angle: 0.0,
    }
  }
}

/// Parameters for a centered pile with a constant offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PileLayout {
  /// X offset applied after each child.
  pub step_x: f64,
  /// Y offset applied after each child.
  pub step_y: f64,
}

impl Default for PileLayout {
  fn default() -> Self {
    Self {
      step_x: 0.0,
      step_y: 0.0,
    }
  }
}

/// Parameters for an elliptical arc centered in the extent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcLayout {
  /// First child's angle in radians.
  pub start_angle: f64,
  /// Last child's angle in radians.
  pub end_angle: f64,
  /// Horizontal radius.
  pub radius_x: f64,
  /// Vertical radius.
  pub radius_y: f64,
}

impl Default for ArcLayout {
  fn default() -> Self {
    Self {
      start_angle: 0.0,
      end_angle: std::f64::consts::PI,
      radius_x: 0.0,
      radius_y: 0.0,
    }
  }
}

/// A world-layout component parameterized by a pure algorithm.
#[derive(Clone)]
pub struct WorldLayout<A> {
  pub(crate) algorithm: A,
  plane: LayoutPlane,
  extent: LayoutExtent,
  children: Vec<LayoutChild>,
  movement: Option<Transition>,
}

impl<A> Default for WorldLayout<A>
where
  A: LayoutAlgorithm + Default,
{
  fn default() -> Self {
    Self::custom(A::default())
  }
}

impl<A: LayoutAlgorithm> WorldLayout<A> {
  /// Creates a world layout from an application-defined pure algorithm.
  #[must_use]
  pub fn custom(algorithm: A) -> Self {
    Self {
      algorithm,
      plane: LayoutPlane::xy(Vector3::ZERO),
      extent: LayoutExtent::new(1.0, 1.0),
      children: Vec::new(),
      movement: None,
    }
  }

  /// Sets the explicit layout plane.
  #[must_use]
  pub fn plane(mut self, plane: LayoutPlane) -> Self {
    validate_plane(plane);
    self.plane = plane;
    self
  }

  /// Sets the available world-unit extent.
  #[must_use]
  pub fn extent(mut self, extent: impl Into<LayoutExtent>) -> Self {
    self.extent = extent.into();
    self
  }

  /// Appends one ordered child.
  #[must_use]
  pub fn child(mut self, child: LayoutChild) -> Self {
    self.children.push(child);
    self
  }

  /// Appends ordered children.
  #[must_use]
  pub fn children(mut self, children: impl IntoIterator<Item = LayoutChild>) -> Self {
    self.children.extend(children);
    self
  }

  /// Overrides movement timing for descendant destinations without an object override.
  #[must_use]
  pub fn movement(mut self, transition: Transition) -> Self {
    self.movement = Some(transition);
    self
  }

  /// Computes world targets without rendering children.
  #[must_use]
  pub fn targets(&self, items: &[LayoutItem]) -> Vec<LayoutTarget> {
    validate_extent(self.extent);
    validate_plane(self.plane);
    for item in items {
      validate_item(*item);
    }
    let placements = self.algorithm.placements(self.extent, items);
    assert_eq!(
      placements.len(),
      items.len(),
      "layout algorithms must return one placement per item"
    );
    items
      .iter()
      .zip(placements)
      .map(|(item, placement)| map_target(self.plane, *item, placement))
      .collect()
  }
}

impl<A: LayoutAlgorithm> Component for WorldLayout<A> {
  fn render(&self) -> impl Render {
    let layout_id = hooks::use_memo(ObjectId::new_v4, ());
    let requests = self
      .children
      .iter()
      .filter_map(|child| {
        child.measurement.map(|measurement| {
          WorldRef::rest_bounds(
            ObjectId::from_uuid(child.destination.id()).expect("layout destinations cannot be nil"),
            measurement.id(),
          )
        })
      })
      .collect::<Vec<_>>();
    let snapshot = geometry::use_geometry(requests);
    let mut measured = snapshot.measurements.into_iter();
    let items = self
      .children
      .iter()
      .map(|child| {
        let Some(_) = child.measurement else {
          return Some(child.item);
        };
        let measurement = measured
          .next()
          .expect("one result per measured layout child");
        let (MeasurementStatus::Current, Some(WorldGeometry::RestBounds(value))) =
          (measurement.status, measurement.latest)
        else {
          return None;
        };
        let pivot_x = -value.bound.x / value.bound.width;
        let pivot_y = -value.bound.y / value.bound.height;
        let mut item = child.item;
        item.rest = LayoutBox::with_pivot(value.bound.width, value.bound.height, pivot_x, pivot_y);
        Some(item)
      })
      .collect::<Option<Vec<_>>>();
    assert!(
      measured.next().is_none(),
      "every rest measurement is consumed"
    );
    let targets = items.as_ref().map(|items| self.targets(items));
    let mut commits = Vec::with_capacity(self.children.len());
    let rendered = self
      .children
      .iter()
      .enumerate()
      .flat_map(|(index, child)| {
        let target = targets
          .as_ref()
          .map(|targets| targets[index])
          .or_else(|| child.destination.latest());
        if let Some(target) = target {
          child.destination.update(target);
          commits.push((child.destination.clone(), target));
        }
        let anchor = Group::new()
          .id(*child.destination.target_id.as_uuid())
          .transform(target.map_or_else(LocalTransform::default, |value| value.transform));
        let anchor = if target.is_some() {
          anchor.with_motion(MotionProps::new().animate(StyleTarget::new()))
        } else {
          anchor
        };
        let group = Group::new()
          .id(child.destination.id())
          .preserve_world_on_reparent()
          .child(child.content.render());
        let group = match target {
          Some(target) => {
            let group = group.transform(child.destination.base(target));
            if child.destination.moving(layout_id, target) {
              group.with_motion(MotionProps::new().animate(placement_target(target.transform)))
            } else {
              group.with_motion(MotionProps::new().animate(StyleTarget::new()))
            }
          }
          None => group,
        };
        let movement = child.movement.as_ref().or(self.movement.as_ref());
        let visual = match movement {
          Some(transition) => Node::new(MotionConfig::new(group).transition(transition.clone())),
          None => Node::new(group),
        };
        [Node::new(anchor), visual]
      })
      .collect::<Vec<_>>();
    hooks::use_effect_always(move || {
      for (destination, target) in commits {
        destination.commit(layout_id, target);
      }
    });
    rendered
  }
}

impl From<LayoutDestination> for MotionPositionRef {
  fn from(value: LayoutDestination) -> Self {
    value.follow()
  }
}

impl From<&LayoutDestination> for MotionPositionRef {
  fn from(value: &LayoutDestination) -> Self {
    value.follow()
  }
}

fn placement_target(value: LocalTransform) -> StyleTarget {
  let angles = quaternion_angles(value.rotation);
  StyleTarget::new()
    .local_position_x(value.position.x as f32)
    .local_position_y(value.position.y as f32)
    .local_position_z(value.position.z as f32)
    .local_rotation_x(angles[0] as f32)
    .local_rotation_y(angles[1] as f32)
    .local_rotation_z(angles[2] as f32)
    .local_scale_x(value.scale.x as f32)
    .local_scale_y(value.scale.y as f32)
    .local_scale_z(value.scale.z as f32)
}

fn quaternion_angles(value: battlement::Quaternion) -> [f64; 3] {
  let length =
    (value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w).sqrt();
  assert!(length > f64::EPSILON, "layout rotations cannot be zero");
  let x = value.x / length;
  let y = value.y / length;
  let z = value.z / length;
  let w = value.w / length;
  let sine = (2.0 * (w * x - y * z)).clamp(-1.0, 1.0);
  let x_angle = sine.asin();
  let (y_angle, z_angle) = if sine.abs() < 0.999_999_9 {
    (
      (2.0 * (x * z + w * y)).atan2(1.0 - 2.0 * (x * x + y * y)),
      (2.0 * (x * y + w * z)).atan2(1.0 - 2.0 * (x * x + z * z)),
    )
  } else {
    (
      (2.0 * (w * y - x * z)).atan2(1.0 - 2.0 * (y * y + z * z)),
      0.0,
    )
  };
  [x_angle, y_angle, z_angle].map(|angle| angle.to_degrees().rem_euclid(360.0))
}
