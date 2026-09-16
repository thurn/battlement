//! Layout projection, shared-layout groups, and reorder helpers.

use std::{
  any::{TypeId, type_name},
  cell::RefCell,
  hash::{Hash, Hasher},
};

use battlement::{MotionLayoutDescriptor, MotionLayoutIdentity, MotionLayoutMode};

use crate::{
  gesture::DragAxis,
  motion::{MotionProps, Transition},
  render::{Render, RenderSink},
  render_value::Sealed,
};

thread_local! {
  static GROUPS: RefCell<Vec<MotionLayoutIdentity>> = const { RefCell::new(Vec::new()) };
}

/// Axes projected after a state-driven native layout change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layout {
  /// Preserve visual position while native layout catches up.
  Position,
  /// Preserve visual size while native layout catches up.
  Size,
  /// Preserve both visual position and size.
  Both,
}

/// Hostless logical boundary for shared-layout identity.
pub struct LayoutGroup<R = ()> {
  identity: MotionLayoutIdentity,
  child: R,
}

/// Axis configuration used by reorder helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReorderAxis {
  /// Reorder horizontally.
  X,
  /// Reorder vertically.
  Y,
}

struct LayoutGroupMarker;

struct GroupGuard;

struct StableHasher(u64);

impl LayoutGroup<()> {
  /// Creates an empty group with typed stable identity.
  pub fn new<K: Hash + 'static>(value: K) -> Self {
    Self {
      identity: identity(value),
      child: (),
    }
  }
}

impl<R> LayoutGroup<R> {
  /// Replaces the group contents.
  pub fn child<C>(self, child: C) -> LayoutGroup<C> {
    LayoutGroup {
      identity: self.identity,
      child,
    }
  }
}

impl<R: Render> Render for LayoutGroup<R> {}

#[allow(private_interfaces)]
impl<R: Render> Sealed for LayoutGroup<R> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<LayoutGroupMarker>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    with_group(self.identity.clone(), || self.child.render_into(sink));
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    with_group(self.identity, || self.child.render_owned(sink));
  }
}

impl MotionProps {
  /// Enables state-driven layout projection.
  #[must_use]
  pub fn layout(mut self, value: Layout) -> Self {
    self.layout.mode = Some(value);
    self
  }

  /// Assigns typed shared-layout identity in the nearest [`LayoutGroup`].
  #[must_use]
  pub fn layout_id<K: Hash + 'static>(mut self, value: K) -> Self {
    self.layout.layout_id = Some(identity(value));
    self
  }

  /// Marks this host as a projection-aware scroll boundary.
  #[must_use]
  pub fn layout_scroll(mut self, value: bool) -> Self {
    self.layout.scroll = value;
    self
  }

  /// Establishes a fixed projection root.
  #[must_use]
  pub fn layout_root(mut self, value: bool) -> Self {
    self.layout.root = value;
    self
  }

  /// Configures a drag-backed reorder item.
  #[must_use]
  pub fn reorder_item(self, axis: ReorderAxis) -> Self {
    self
      .layout(Layout::Position)
      .drag(match axis {
        ReorderAxis::X => DragAxis::X,
        ReorderAxis::Y => DragAxis::Y,
      })
      .drag_momentum(false)
  }

  pub(crate) fn layout_descriptor(&self) -> Option<MotionLayoutDescriptor> {
    let mode = self.layout.mode?;
    Some(MotionLayoutDescriptor {
      mode: match mode {
        Layout::Position => MotionLayoutMode::Position,
        Layout::Size => MotionLayoutMode::Size,
        Layout::Both => MotionLayoutMode::Both,
      },
      group: current_group(),
      layout_id: self.layout.layout_id.clone(),
      scroll: self.layout.scroll,
      root: self.layout.root,
      pop_layout: false,
      transition: self
        .transition
        .as_ref()
        .and_then(|value| {
          value
            .properties
            .iter()
            .find(|(property, _)| *property == battlement::MotionProperty::Layout)
            .map(|(_, value)| value.clone())
        })
        .unwrap_or_else(|| Transition::spring().default),
    })
  }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LayoutProps {
  pub(crate) mode: Option<Layout>,
  pub(crate) layout_id: Option<MotionLayoutIdentity>,
  pub(crate) scroll: bool,
  pub(crate) root: bool,
}

/// Returns the insertion index reached after dragging across ordered centers.
#[must_use]
pub fn reorder_index(origin: usize, offset: f32, centers: &[f32]) -> usize {
  if centers.is_empty() {
    return 0;
  }
  let origin = origin.min(centers.len() - 1);
  let projected = centers[origin] + offset;
  let insertion = centers.partition_point(|center| *center < projected);
  if insertion > origin {
    insertion - 1
  } else {
    insertion
  }
}

fn with_group<T>(identity: MotionLayoutIdentity, operation: impl FnOnce() -> T) -> T {
  GROUPS.with(|groups| groups.borrow_mut().push(identity));
  let _guard = GroupGuard;
  operation()
}

fn current_group() -> MotionLayoutIdentity {
  GROUPS
    .with(|groups| groups.borrow().last().cloned())
    .unwrap_or_else(|| identity("reactant-default-layout-group"))
}

fn identity<K: Hash + 'static>(value: K) -> MotionLayoutIdentity {
  let mut hasher = StableHasher(0xcbf29ce484222325);
  value.hash(&mut hasher);
  MotionLayoutIdentity {
    value_type: type_name::<K>().to_owned(),
    value_hash: hasher.finish(),
  }
}

impl Hasher for StableHasher {
  fn finish(&self) -> u64 {
    self.0
  }

  fn write(&mut self, bytes: &[u8]) {
    for byte in bytes {
      self.0 ^= u64::from(*byte);
      self.0 = self.0.wrapping_mul(0x100000001b3);
    }
  }

  fn write_u8(&mut self, value: u8) {
    self.write(&value.to_le_bytes());
  }

  fn write_u16(&mut self, value: u16) {
    self.write(&value.to_le_bytes());
  }

  fn write_u32(&mut self, value: u32) {
    self.write(&value.to_le_bytes());
  }

  fn write_u64(&mut self, value: u64) {
    self.write(&value.to_le_bytes());
  }

  fn write_u128(&mut self, value: u128) {
    self.write(&value.to_le_bytes());
  }

  fn write_usize(&mut self, value: usize) {
    self.write_u64(value as u64);
  }

  fn write_i8(&mut self, value: i8) {
    self.write(&value.to_le_bytes());
  }

  fn write_i16(&mut self, value: i16) {
    self.write(&value.to_le_bytes());
  }

  fn write_i32(&mut self, value: i32) {
    self.write(&value.to_le_bytes());
  }

  fn write_i64(&mut self, value: i64) {
    self.write(&value.to_le_bytes());
  }

  fn write_i128(&mut self, value: i128) {
    self.write(&value.to_le_bytes());
  }

  fn write_isize(&mut self, value: isize) {
    self.write_i64(value as i64);
  }
}

impl Drop for GroupGuard {
  fn drop(&mut self) {
    GROUPS.with(|groups| {
      groups.borrow_mut().pop();
    });
  }
}
