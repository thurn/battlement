//! Deterministic world-space layout from explicit rest geometry.

use std::{cell::RefCell, rc::Rc};

use battlement::{LocalTransform, Vector3};
use reactant_core::{
  component::Component,
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
  target: Rc<RefCell<Option<LayoutTarget>>>,
}

impl LayoutDestination {
  /// Creates a destination associated with a stable presentation ID.
  #[must_use]
  pub fn new(id: Uuid) -> Self {
    assert!(!id.is_nil(), "layout destination IDs cannot be nil");
    Self {
      id,
      target: Rc::new(RefCell::new(None)),
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
    *self.target.borrow()
  }

  fn update(&self, target: LayoutTarget) {
    self.target.replace(Some(target));
  }
}

/// Rendered content and stable rest geometry for one layout item.
#[derive(Clone)]
pub struct LayoutChild {
  item: LayoutItem,
  destination: LayoutDestination,
  content: Child,
}

impl LayoutChild {
  /// Creates a child whose wrapper owns the destination's presentation ID.
  #[must_use]
  pub fn new(destination: LayoutDestination, rest: LayoutBox, content: impl Render) -> Self {
    Self {
      item: LayoutItem::new(destination.id(), rest),
      destination,
      content: Child::new(content),
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
    let items: Vec<_> = self.children.iter().map(|child| child.item).collect();
    let targets = self.targets(&items);
    self
      .children
      .iter()
      .zip(targets)
      .map(|(child, target)| {
        child.destination.update(target);
        Node::new(
          Group::new()
            .id(target.id)
            .transform(target.transform)
            .child(child.content.render()),
        )
      })
      .collect::<Vec<_>>()
  }
}
