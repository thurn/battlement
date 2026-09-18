use battlement::{LocalTransform, Quaternion, Vector3};
use taffy::prelude::{
  AlignItems, AvailableSpace, Dimension, Display, FlexDirection as TaffyFlexDirection,
  JustifyContent, Line, Size, Style, TaffyAuto, TaffyTree, fr, length, line,
};

use crate::world_layout::{
  ArcLayout, FanLayout, FlexDirection, FlexLayout, GridLayout, LayoutAlgorithm, LayoutAlignment,
  LayoutExtent, LayoutItem, LayoutOrientation, LayoutPlacement, LayoutPlane, LayoutScaling,
  LayoutTarget, PileLayout,
};

pub(crate) fn validate_extent(extent: LayoutExtent) {
  assert!(
    extent.width.is_finite() && extent.width > 0.0 && extent.width <= f32::MAX.into(),
    "layout width must be positive and finite"
  );
  assert!(
    extent.height.is_finite() && extent.height > 0.0 && extent.height <= f32::MAX.into(),
    "layout height must be positive and finite"
  );
}

pub(crate) fn validate_plane(plane: LayoutPlane) {
  for value in [plane.origin, plane.x_axis, plane.y_axis] {
    assert!(
      [value.x, value.y, value.z].into_iter().all(f64::is_finite),
      "layout plane values must be finite"
    );
  }
  let x = length3(plane.x_axis);
  let y = length3(plane.y_axis);
  assert!(
    (x - 1.0).abs() < 1e-6 && (y - 1.0).abs() < 1e-6,
    "layout plane axes must be unit length"
  );
  assert!(
    dot(plane.x_axis, plane.y_axis).abs() < 1e-6,
    "layout plane axes must be orthogonal"
  );
}

pub(crate) fn validate_item(item: LayoutItem) {
  assert!(!item.id.is_nil(), "layout item IDs cannot be nil");
  assert!(item.depth.is_finite(), "layout depth must be finite");
  assert!(
    item.flex_grow.is_finite() && item.flex_grow >= 0.0,
    "flex grow must be finite and nonnegative"
  );
  assert!(
    item.flex_shrink.is_finite() && item.flex_shrink >= 0.0,
    "flex shrink must be finite and nonnegative"
  );
  assert!(
    item.column_span > 0 && item.row_span > 0,
    "grid spans must be positive"
  );
  for value in [
    item.authored.rotation.x,
    item.authored.rotation.y,
    item.authored.rotation.z,
    item.authored.rotation.w,
    item.authored.scale.x,
    item.authored.scale.y,
    item.authored.scale.z,
  ] {
    assert!(
      value.is_finite(),
      "authored layout transforms must be finite"
    );
  }
}

pub(crate) fn map_target(
  plane: LayoutPlane,
  item: LayoutItem,
  placement: LayoutPlacement,
) -> LayoutTarget {
  for value in [
    placement.x,
    placement.y,
    placement.width,
    placement.height,
    placement.angle,
  ] {
    assert!(value.is_finite(), "layout placements must be finite");
  }
  assert!(
    placement.width > 0.0 && placement.height > 0.0,
    "layout placement sizes must be positive"
  );
  let normal = cross(plane.x_axis, plane.y_axis);
  let position = add(
    add(plane.origin, scale(plane.x_axis, placement.x)),
    add(scale(plane.y_axis, placement.y), scale(normal, item.depth)),
  );
  let rotation = match item.orientation {
    LayoutOrientation::Preserve => item.authored.rotation,
    LayoutOrientation::Plane => plane_rotation(plane, 0.0),
    LayoutOrientation::Arrangement => plane_rotation(plane, placement.angle),
  };
  let scale = match item.scaling {
    LayoutScaling::Preserve => item.authored.scale,
    LayoutScaling::Fit => Vector3::new(
      item.authored.scale.x * placement.width / item.rest.width,
      item.authored.scale.y * placement.height / item.rest.height,
      item.authored.scale.z,
    ),
  };
  LayoutTarget {
    id: item.id,
    transform: LocalTransform {
      position,
      rotation,
      scale,
    },
  }
}

impl LayoutAlgorithm for FlexLayout {
  fn placements(&self, extent: LayoutExtent, items: &[LayoutItem]) -> Vec<LayoutPlacement> {
    assert!(
      self.gap.is_finite() && self.gap >= 0.0 && self.gap <= f32::MAX.into(),
      "flex gap must be finite and nonnegative"
    );
    assert_ne!(
      self.justify,
      LayoutAlignment::Stretch,
      "stretch is not a main-axis alignment"
    );
    assert_ne!(
      self.align,
      LayoutAlignment::SpaceBetween,
      "space-between is not a cross-axis alignment"
    );
    let direction = match self.direction {
      FlexDirection::Row => TaffyFlexDirection::Row,
      FlexDirection::Column => TaffyFlexDirection::ColumnReverse,
    };
    let root = Style {
      display: Display::Flex,
      flex_direction: direction,
      size: taffy_size(extent),
      gap: Size {
        width: length(self.gap as f32),
        height: length(self.gap as f32),
      },
      justify_content: Some(justify(self.justify)),
      align_items: Some(align(self.align)),
      ..Style::default()
    };
    taffy_placements(root, extent, items, None)
  }
}

impl LayoutAlgorithm for GridLayout {
  fn placements(&self, extent: LayoutExtent, items: &[LayoutItem]) -> Vec<LayoutPlacement> {
    assert!(self.columns > 0, "grid must have at least one column");
    assert!(
      self.columns < i16::MAX as u16,
      "grid columns must fit Taffy line indices"
    );
    let maximum_rows: usize = items.iter().map(|item| usize::from(item.row_span)).sum();
    assert!(
      maximum_rows < i16::MAX as usize,
      "grid rows must fit Taffy line indices"
    );
    assert!(
      self.column_gap.is_finite() && self.column_gap >= 0.0 && self.column_gap <= f32::MAX.into(),
      "grid column gap must be finite and nonnegative"
    );
    assert!(
      self.row_gap.is_finite() && self.row_gap >= 0.0 && self.row_gap <= f32::MAX.into(),
      "grid row gap must be finite and nonnegative"
    );
    assert_ne!(
      self.align,
      LayoutAlignment::SpaceBetween,
      "space-between is not a grid item alignment"
    );
    let positions = grid_positions(self.columns, items);
    let rows = positions
      .iter()
      .zip(items)
      .map(|((_, row), item)| usize::from(*row + item.row_span))
      .max()
      .unwrap_or(1);
    let root = Style {
      display: Display::Grid,
      size: taffy_size(extent),
      grid_template_columns: vec![fr(1.0); usize::from(self.columns)],
      grid_template_rows: vec![fr(1.0); rows],
      gap: Size {
        width: length(self.column_gap as f32),
        height: length(self.row_gap as f32),
      },
      align_items: Some(align(self.align)),
      justify_items: Some(align(self.align)),
      ..Style::default()
    };
    taffy_placements(root, extent, items, Some(&positions))
  }
}

fn taffy_placements(
  root_style: Style,
  extent: LayoutExtent,
  items: &[LayoutItem],
  grid_positions: Option<&[(u16, u16)]>,
) -> Vec<LayoutPlacement> {
  let mut taffy: TaffyTree<()> = TaffyTree::new();
  taffy.disable_rounding();
  let children: Vec<_> = items
    .iter()
    .enumerate()
    .map(|(index, item)| {
      let fit = item.scaling == LayoutScaling::Fit;
      let fixed = Size {
        width: if fit {
          Dimension::AUTO
        } else {
          length(item.rest.width as f32)
        },
        height: if fit {
          Dimension::AUTO
        } else {
          length(item.rest.height as f32)
        },
      };
      let (grid_column, grid_row) = grid_positions.map_or_else(
        || (Line::default(), Line::default()),
        |positions| {
          let (column, row) = positions[index];
          (
            Line {
              start: line((column + 1) as i16),
              end: line((column + item.column_span + 1) as i16),
            },
            Line {
              start: line((row + 1) as i16),
              end: line((row + item.row_span + 1) as i16),
            },
          )
        },
      );
      taffy
        .new_leaf(Style {
          size: fixed,
          min_size: Size {
            width: length(item.rest.width as f32),
            height: length(item.rest.height as f32),
          },
          flex_grow: item.flex_grow,
          flex_shrink: item.flex_shrink,
          grid_column,
          grid_row,
          ..Style::default()
        })
        .expect("valid Taffy leaf")
    })
    .collect();
  let root = taffy
    .new_with_children(root_style, &children)
    .expect("valid Taffy layout tree");
  taffy
    .compute_layout(
      root,
      Size {
        width: AvailableSpace::Definite(extent.width as f32),
        height: AvailableSpace::Definite(extent.height as f32),
      },
    )
    .expect("valid Taffy world layout");
  children
    .iter()
    .zip(items)
    .map(|(node, item)| {
      let layout = taffy.layout(*node).expect("computed Taffy child");
      let width = f64::from(layout.size.width);
      let height = f64::from(layout.size.height);
      LayoutPlacement {
        x: f64::from(layout.location.x) + width * item.rest.pivot_x,
        y: extent.height - f64::from(layout.location.y) - height * (1.0 - item.rest.pivot_y),
        width,
        height,
        angle: 0.0,
      }
    })
    .collect()
}

fn grid_positions(columns: u16, items: &[LayoutItem]) -> Vec<(u16, u16)> {
  let mut occupied: Vec<Vec<bool>> = Vec::new();
  items
    .iter()
    .map(|item| {
      assert!(
        item.column_span <= columns,
        "grid item span exceeds the declared columns"
      );
      let mut row = 0;
      loop {
        while occupied.len() < usize::from(row + item.row_span) {
          occupied.push(vec![false; usize::from(columns)]);
        }
        for column in 0..=(columns - item.column_span) {
          let free = (row..row + item.row_span).all(|candidate_row| {
            (column..column + item.column_span).all(|candidate_column| {
              !occupied[usize::from(candidate_row)][usize::from(candidate_column)]
            })
          });
          if free {
            for candidate_row in row..row + item.row_span {
              for candidate_column in column..column + item.column_span {
                occupied[usize::from(candidate_row)][usize::from(candidate_column)] = true;
              }
            }
            return (column, row);
          }
        }
        row += 1;
      }
    })
    .collect()
}

impl LayoutAlgorithm for FanLayout {
  fn placements(&self, extent: LayoutExtent, items: &[LayoutItem]) -> Vec<LayoutPlacement> {
    finite([self.spread, self.rise, self.angle]);
    normalized(items)
      .zip(items)
      .map(|(t, item)| LayoutPlacement {
        x: extent.width / 2.0 + self.spread * t / 2.0,
        y: extent.height / 2.0 + self.rise * (1.0 - t * t),
        width: item.rest.width,
        height: item.rest.height,
        angle: self.angle * t / 2.0,
      })
      .collect()
  }
}

impl LayoutAlgorithm for PileLayout {
  fn placements(&self, extent: LayoutExtent, items: &[LayoutItem]) -> Vec<LayoutPlacement> {
    finite([self.step_x, self.step_y]);
    let center = (items.len().saturating_sub(1) as f64) / 2.0;
    items
      .iter()
      .enumerate()
      .map(|(index, item)| {
        let offset = index as f64 - center;
        LayoutPlacement {
          x: extent.width / 2.0 + self.step_x * offset,
          y: extent.height / 2.0 + self.step_y * offset,
          width: item.rest.width,
          height: item.rest.height,
          angle: 0.0,
        }
      })
      .collect()
  }
}

impl LayoutAlgorithm for ArcLayout {
  fn placements(&self, extent: LayoutExtent, items: &[LayoutItem]) -> Vec<LayoutPlacement> {
    finite([
      self.start_angle,
      self.end_angle,
      self.radius_x,
      self.radius_y,
    ]);
    assert!(
      self.radius_x >= 0.0 && self.radius_y >= 0.0,
      "arc radii must be nonnegative"
    );
    normalized_01(items)
      .zip(items)
      .map(|(t, item)| {
        let angle = self.start_angle + (self.end_angle - self.start_angle) * t;
        LayoutPlacement {
          x: extent.width / 2.0 + self.radius_x * angle.cos(),
          y: extent.height / 2.0 + self.radius_y * angle.sin(),
          width: item.rest.width,
          height: item.rest.height,
          angle: angle + std::f64::consts::FRAC_PI_2,
        }
      })
      .collect()
  }
}

fn normalized(items: &[LayoutItem]) -> impl Iterator<Item = f64> + '_ {
  let divisor = items.len().saturating_sub(1) as f64;
  items.indices().map(move |index| {
    if divisor == 0.0 {
      0.0
    } else {
      2.0 * index as f64 / divisor - 1.0
    }
  })
}

fn normalized_01(items: &[LayoutItem]) -> impl Iterator<Item = f64> + '_ {
  let divisor = items.len().saturating_sub(1) as f64;
  items.indices().map(move |index| {
    if divisor == 0.0 {
      0.5
    } else {
      index as f64 / divisor
    }
  })
}

trait Indices {
  fn indices(&self) -> std::ops::Range<usize>;
}

impl<T> Indices for [T] {
  fn indices(&self) -> std::ops::Range<usize> {
    0..self.len()
  }
}

fn taffy_size(extent: LayoutExtent) -> Size<Dimension> {
  Size {
    width: length(extent.width as f32),
    height: length(extent.height as f32),
  }
}

fn justify(value: LayoutAlignment) -> JustifyContent {
  match value {
    LayoutAlignment::Start => JustifyContent::FLEX_START,
    LayoutAlignment::Center => JustifyContent::CENTER,
    LayoutAlignment::End => JustifyContent::FLEX_END,
    LayoutAlignment::SpaceBetween => JustifyContent::SPACE_BETWEEN,
    LayoutAlignment::Stretch => unreachable!("validated main-axis alignment"),
  }
}

fn align(value: LayoutAlignment) -> AlignItems {
  match value {
    LayoutAlignment::Start => AlignItems::FLEX_START,
    LayoutAlignment::Center => AlignItems::CENTER,
    LayoutAlignment::End => AlignItems::FLEX_END,
    LayoutAlignment::Stretch => AlignItems::STRETCH,
    LayoutAlignment::SpaceBetween => unreachable!("validated cross-axis alignment"),
  }
}

fn finite<const N: usize>(values: [f64; N]) {
  assert!(
    values.into_iter().all(f64::is_finite),
    "layout parameters must be finite"
  );
}

fn length3(value: Vector3) -> f64 {
  dot(value, value).sqrt()
}

fn dot(left: Vector3, right: Vector3) -> f64 {
  left.x * right.x + left.y * right.y + left.z * right.z
}

fn cross(left: Vector3, right: Vector3) -> Vector3 {
  Vector3::new(
    left.y * right.z - left.z * right.y,
    left.z * right.x - left.x * right.z,
    left.x * right.y - left.y * right.x,
  )
}

fn add(left: Vector3, right: Vector3) -> Vector3 {
  Vector3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn scale(value: Vector3, factor: f64) -> Vector3 {
  Vector3::new(value.x * factor, value.y * factor, value.z * factor)
}

fn plane_rotation(plane: LayoutPlane, angle: f64) -> Quaternion {
  let cosine = angle.cos();
  let sine = angle.sin();
  let x = add(scale(plane.x_axis, cosine), scale(plane.y_axis, sine));
  let y = add(scale(plane.x_axis, -sine), scale(plane.y_axis, cosine));
  let z = cross(x, y);
  quaternion_from_basis(x, y, z)
}

fn quaternion_from_basis(x: Vector3, y: Vector3, z: Vector3) -> Quaternion {
  let trace = x.x + y.y + z.z;
  if trace > 0.0 {
    let scale = (trace + 1.0).sqrt() * 2.0;
    return Quaternion::new(
      (y.z - z.y) / scale,
      (z.x - x.z) / scale,
      (x.y - y.x) / scale,
      scale / 4.0,
    );
  }
  if x.x > y.y && x.x > z.z {
    let scale = (1.0 + x.x - y.y - z.z).sqrt() * 2.0;
    return Quaternion::new(
      scale / 4.0,
      (y.x + x.y) / scale,
      (z.x + x.z) / scale,
      (y.z - z.y) / scale,
    );
  }
  if y.y > z.z {
    let scale = (1.0 + y.y - x.x - z.z).sqrt() * 2.0;
    return Quaternion::new(
      (y.x + x.y) / scale,
      scale / 4.0,
      (z.y + y.z) / scale,
      (z.x - x.z) / scale,
    );
  }
  let scale = (1.0 + z.z - x.x - y.y).sqrt() * 2.0;
  Quaternion::new(
    (z.x + x.z) / scale,
    (z.y + y.z) / scale,
    scale / 4.0,
    (x.y - y.x) / scale,
  )
}
