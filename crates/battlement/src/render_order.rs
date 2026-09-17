//! Relative visual ordering for world renderers and sorting groups.

/// Ordering inside the nearest enclosing sorting group, or the scene when ungrouped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderOrder {
  /// Render this object's visual descendants together at the declared relative order.
  Group(i16),
  /// Place this object's renderer within its enclosing sorting group.
  Layer(i16),
}
