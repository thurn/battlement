use battlement_types::ObjectId;
use serde::{Deserialize, Serialize};

use crate::{
  Align, FlexDirection, FlexWrap, Justify, LanguageDirection, PickingMode, Prop, Style,
  UiVisualElement, UiVisualElementProperties, UsageHint,
};

/// One explicit or implicit Grid track size.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum GridTrack {
  /// A fixed nonnegative pixel track.
  Px(f32),
  /// A positive share of the remaining space.
  Fraction(f32),
  /// A track sized from its items' preferred outer sizes.
  Auto,
}

impl GridTrack {
  /// Creates a fixed pixel track.
  #[must_use]
  pub const fn px(value: f32) -> Self {
    Self::Px(value)
  }

  /// Creates a fractional track.
  #[must_use]
  pub const fn fr(value: f32) -> Self {
    Self::Fraction(value)
  }

  /// Creates an automatic track.
  #[must_use]
  pub const fn auto() -> Self {
    Self::Auto
  }
}

/// Major-axis scan direction used by Grid auto-placement.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum GridAutoFlow {
  /// Fills columns before creating another row.
  #[default]
  Row,
  /// Fills rows before creating another column.
  Column,
}

/// Placement and alignment of one Grid child.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct GridItem {
  /// Optional one-based row start.
  pub row: Option<u32>,
  /// Optional one-based column start.
  pub column: Option<u32>,
  /// Positive number of rows occupied.
  pub row_span: u32,
  /// Positive number of columns occupied.
  pub column_span: u32,
  /// Vertical alignment override, or [`Align::Auto`] to inherit.
  pub align_self: Align,
  /// Horizontal alignment override, or [`Align::Auto`] to inherit.
  pub justify_self: Align,
}

impl Default for GridItem {
  fn default() -> Self {
    Self {
      row: None,
      column: None,
      row_span: 1,
      column_span: 1,
      align_self: Align::Auto,
      justify_self: Align::Auto,
    }
  }
}

impl GridItem {
  /// Creates an automatically placed item spanning one track on each axis.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the one-based row start.
  #[must_use]
  pub const fn row(mut self, value: u32) -> Self {
    self.row = Some(value);
    self
  }

  /// Sets the one-based column start.
  #[must_use]
  pub const fn column(mut self, value: u32) -> Self {
    self.column = Some(value);
    self
  }

  /// Sets the positive row span.
  #[must_use]
  pub const fn span_rows(mut self, value: u32) -> Self {
    self.row_span = value;
    self
  }

  /// Sets the positive column span.
  #[must_use]
  pub const fn span_columns(mut self, value: u32) -> Self {
    self.column_span = value;
    self
  }

  /// Overrides vertical alignment inside the resolved grid area.
  #[must_use]
  pub const fn align_self(mut self, value: Align) -> Self {
    self.align_self = value;
    self
  }

  /// Overrides horizontal alignment inside the resolved grid area.
  #[must_use]
  pub const fn justify_self(mut self, value: Align) -> Self {
    self.justify_self = value;
    self
  }
}

/// Placement and presentation order of one Stack child.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct StackItem {
  /// Presentation order within the isolated Stack context.
  pub order: i32,
  /// Vertical alignment override, or [`Align::Auto`] to inherit.
  pub align_self: Align,
  /// Horizontal alignment override, or [`Align::Auto`] to inherit.
  pub justify_self: Align,
  /// Optional nonnegative top inset.
  pub top: Option<f32>,
  /// Optional nonnegative right inset.
  pub right: Option<f32>,
  /// Optional nonnegative bottom inset.
  pub bottom: Option<f32>,
  /// Optional nonnegative left inset.
  pub left: Option<f32>,
  /// Whether this layer contributes to the Stack's intrinsic size.
  pub contributes_to_size: bool,
}

impl Default for StackItem {
  fn default() -> Self {
    Self {
      order: 0,
      align_self: Align::Auto,
      justify_self: Align::Auto,
      top: None,
      right: None,
      bottom: None,
      left: None,
      contributes_to_size: true,
    }
  }
}

impl StackItem {
  /// Creates default Stack placement metadata.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      order: 0,
      align_self: Align::Auto,
      justify_self: Align::Auto,
      top: None,
      right: None,
      bottom: None,
      left: None,
      contributes_to_size: true,
    }
  }

  /// Sets the presentation order within the parent Stack.
  #[must_use]
  pub const fn order(mut self, value: i32) -> Self {
    self.order = value;
    self
  }

  /// Overrides vertical alignment inside the Stack.
  #[must_use]
  pub const fn align_self(mut self, value: Align) -> Self {
    self.align_self = value;
    self
  }

  /// Overrides horizontal alignment inside the Stack.
  #[must_use]
  pub const fn justify_self(mut self, value: Align) -> Self {
    self.justify_self = value;
    self
  }

  /// Sets the top inset in panel pixels.
  #[must_use]
  pub const fn top(mut self, value: f32) -> Self {
    self.top = Some(value);
    self
  }

  /// Sets the right inset in panel pixels.
  #[must_use]
  pub const fn right(mut self, value: f32) -> Self {
    self.right = Some(value);
    self
  }

  /// Sets the bottom inset in panel pixels.
  #[must_use]
  pub const fn bottom(mut self, value: f32) -> Self {
    self.bottom = Some(value);
    self
  }

  /// Sets the left inset in panel pixels.
  #[must_use]
  pub const fn left(mut self, value: f32) -> Self {
    self.left = Some(value);
    self
  }

  /// Selects whether this layer contributes to intrinsic Stack size.
  #[must_use]
  pub const fn contributes_to_size(mut self, value: bool) -> Self {
    self.contributes_to_size = value;
    self
  }
}

/// Sticky positioning metadata for one normal-flow child.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Sticky {
  /// Optional signed top viewport inset.
  pub top: Option<f32>,
  /// Optional signed right viewport inset.
  pub right: Option<f32>,
  /// Optional signed bottom viewport inset.
  pub bottom: Option<f32>,
  /// Optional signed left viewport inset.
  pub left: Option<f32>,
  /// Presentation order among sticky entries.
  pub order: i32,
}

impl Sticky {
  /// Creates leading-edge vertical sticky placement.
  #[must_use]
  pub fn top(value: f32) -> Self {
    Self {
      top: Some(value),
      ..Self::default()
    }
  }

  /// Creates trailing-edge horizontal sticky placement.
  #[must_use]
  pub fn right(value: f32) -> Self {
    Self {
      right: Some(value),
      ..Self::default()
    }
  }

  /// Creates trailing-edge vertical sticky placement.
  #[must_use]
  pub fn bottom(value: f32) -> Self {
    Self {
      bottom: Some(value),
      ..Self::default()
    }
  }

  /// Creates leading-edge horizontal sticky placement.
  #[must_use]
  pub fn left(value: f32) -> Self {
    Self {
      left: Some(value),
      ..Self::default()
    }
  }

  /// Adds a top inset to a horizontally constrained descriptor.
  #[must_use]
  pub fn with_top(mut self, value: f32) -> Self {
    assert!(self.top.is_none() && self.bottom.is_none());
    self.top = Some(value);
    self
  }

  /// Adds a right inset to a vertically constrained descriptor.
  #[must_use]
  pub fn with_right(mut self, value: f32) -> Self {
    assert!(self.left.is_none() && self.right.is_none());
    self.right = Some(value);
    self
  }

  /// Adds a bottom inset to a horizontally constrained descriptor.
  #[must_use]
  pub fn with_bottom(mut self, value: f32) -> Self {
    assert!(self.top.is_none() && self.bottom.is_none());
    self.bottom = Some(value);
    self
  }

  /// Adds a left inset to a vertically constrained descriptor.
  #[must_use]
  pub fn with_left(mut self, value: f32) -> Self {
    assert!(self.left.is_none() && self.right.is_none());
    self.left = Some(value);
    self
  }

  /// Sets presentation order among sticky items in one ScrollView.
  #[must_use]
  pub const fn order(mut self, value: i32) -> Self {
    self.order = value;
    self
  }
}

/// Overlay presentation tier.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OverlayLayer {
  /// An application or anchored layer below modal layers.
  Popover,
  /// A viewport-filling modal focus scope.
  Modal,
}

/// Physical side of an anchor used for popover placement.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PlacementSide {
  /// Places the popover above the anchor.
  Top,
  /// Places the popover to the anchor's right.
  Right,
  /// Places the popover below the anchor.
  Bottom,
  /// Places the popover to the anchor's left.
  Left,
}

/// Cross-axis alignment used for popover placement.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PlacementAlign {
  /// Aligns leading edges.
  Start,
  /// Aligns centers.
  Center,
  /// Aligns trailing edges.
  End,
}

/// Complete anchored-popover placement policy.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct PopoverPlacement {
  /// Requested physical side.
  pub side: PlacementSide,
  /// Cross-axis alignment against the anchor.
  pub align: PlacementAlign,
  /// Signed distance away from the selected side.
  pub main_offset: f32,
  /// Signed physical cross-axis displacement.
  pub cross_offset: f32,
  /// Nonnegative host-edge collision padding.
  pub collision_padding: f32,
  /// Whether a less-overflowing opposite side may be selected.
  pub flip: bool,
  /// Whether cross-axis overflow may be shifted into bounds.
  pub shift: bool,
}

impl Default for PopoverPlacement {
  fn default() -> Self {
    Self {
      side: PlacementSide::Bottom,
      align: PlacementAlign::Start,
      main_offset: 0.0,
      cross_offset: 0.0,
      collision_padding: 8.0,
      flip: true,
      shift: true,
    }
  }
}

impl PopoverPlacement {
  /// Places a popover above its anchor with leading edges aligned.
  #[must_use]
  pub fn top_start() -> Self {
    Self {
      side: PlacementSide::Top,
      ..Self::default()
    }
  }

  /// Places a popover to the right of its anchor with leading edges aligned.
  #[must_use]
  pub fn right_start() -> Self {
    Self {
      side: PlacementSide::Right,
      ..Self::default()
    }
  }

  /// Places a popover below its anchor with leading edges aligned.
  #[must_use]
  pub fn bottom_start() -> Self {
    Self::default()
  }

  /// Places a popover to the left of its anchor with leading edges aligned.
  #[must_use]
  pub fn left_start() -> Self {
    Self {
      side: PlacementSide::Left,
      ..Self::default()
    }
  }

  /// Selects cross-axis alignment against the anchor.
  #[must_use]
  pub const fn align(mut self, value: PlacementAlign) -> Self {
    self.align = value;
    self
  }

  /// Sets the signed distance away from the selected side.
  #[must_use]
  pub const fn offset(self, value: f32) -> Self {
    self.main_offset(value)
  }

  /// Sets the signed distance away from the selected side.
  #[must_use]
  pub const fn main_offset(mut self, value: f32) -> Self {
    self.main_offset = value;
    self
  }

  /// Sets the signed physical cross-axis displacement.
  #[must_use]
  pub const fn cross_offset(mut self, value: f32) -> Self {
    self.cross_offset = value;
    self
  }

  /// Sets the host-edge collision padding.
  #[must_use]
  pub const fn collision_padding(mut self, value: f32) -> Self {
    self.collision_padding = value;
    self
  }

  /// Enables or disables opposite-side collision selection.
  #[must_use]
  pub const fn flip(mut self, value: bool) -> Self {
    self.flip = value;
    self
  }

  /// Enables or disables cross-axis collision shifting.
  #[must_use]
  pub const fn shift(mut self, value: bool) -> Self {
    self.shift = value;
    self
  }
}

/// Placement metadata for one top-level overlay portal attachment.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum OverlayPlacement {
  /// An unanchored host-filling layer.
  Layer(OverlayLayer),
  /// A popover anchored to one public host identity.
  Popover {
    /// Public anchor identity.
    anchor: ObjectId,
    /// Placement and collision policy.
    placement: PopoverPlacement,
  },
  /// A modal layer with optional focus targets.
  Modal {
    /// Preferred initial focus target inside the modal.
    initial_focus: Option<ObjectId>,
    /// Preferred focus target after the final modal closes.
    restore_focus: Option<ObjectId>,
  },
}

/// A native flex container with independent row and column gaps.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiFlex {
  /// Shared visual properties and child placement descriptors.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Main-axis direction.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub direction: Prop<FlexDirection>,
  /// Line wrapping policy.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub wrap: Prop<FlexWrap>,
  /// Default cross-axis child alignment.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub align_items: Prop<Align>,
  /// Main-axis distribution.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub justify_content: Prop<Justify>,
  /// Gap between wrapped rows.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub row_gap: Prop<f32>,
  /// Gap between columns.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub column_gap: Prop<f32>,
}

impl UiFlex {
  /// Creates a Flex descriptor with native constructor defaults.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    apply_prop(&mut self.direction, value.direction);
    apply_prop(&mut self.wrap, value.wrap);
    apply_prop(&mut self.align_items, value.align_items);
    apply_prop(&mut self.justify_content, value.justify_content);
    apply_prop(&mut self.row_gap, value.row_gap);
    apply_prop(&mut self.column_gap, value.column_gap);
  }
}

/// A native deterministic Grid container.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiGrid {
  /// Shared visual properties and child placement descriptors.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Explicit column tracks.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub columns: Prop<Vec<GridTrack>>,
  /// Explicit row tracks.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub rows: Prop<Vec<GridTrack>>,
  /// Track size used for implicit columns.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub auto_columns: Prop<GridTrack>,
  /// Track size used for implicit rows.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub auto_rows: Prop<GridTrack>,
  /// Auto-placement scan direction.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub auto_flow: Prop<GridAutoFlow>,
  /// Gap between rows.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub row_gap: Prop<f32>,
  /// Gap between columns.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub column_gap: Prop<f32>,
  /// Default vertical item alignment.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub align_items: Prop<Align>,
  /// Default horizontal item alignment.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub justify_items: Prop<Align>,
}

impl UiGrid {
  /// Creates a Grid descriptor with automatic implicit tracks.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    apply_clone_prop(&mut self.columns, &value.columns);
    apply_clone_prop(&mut self.rows, &value.rows);
    apply_prop(&mut self.auto_columns, value.auto_columns);
    apply_prop(&mut self.auto_rows, value.auto_rows);
    apply_prop(&mut self.auto_flow, value.auto_flow);
    apply_prop(&mut self.row_gap, value.row_gap);
    apply_prop(&mut self.column_gap, value.column_gap);
    apply_prop(&mut self.align_items, value.align_items);
    apply_prop(&mut self.justify_items, value.justify_items);
  }
}

/// A native isolated stacking container.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiStack {
  /// Shared visual properties and child placement descriptors.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Default vertical item alignment.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub align_items: Prop<Align>,
  /// Default horizontal item alignment.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub justify_items: Prop<Align>,
}

impl UiStack {
  /// Creates a Stack descriptor using stretching container defaults.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    apply_prop(&mut self.align_items, value.align_items);
    apply_prop(&mut self.justify_items, value.justify_items);
  }
}

impl UiVisualElementProperties for UiFlex {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}

impl UiVisualElementProperties for UiGrid {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}

impl UiVisualElementProperties for UiStack {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}

fn apply_prop<T: Copy>(target: &mut Prop<T>, value: Prop<T>) {
  if !value.is_unset() {
    *target = value;
  }
}

fn apply_clone_prop<T: Clone>(target: &mut Prop<T>, value: &Prop<T>) {
  if !value.is_unset() {
    *target = value.clone();
  }
}
