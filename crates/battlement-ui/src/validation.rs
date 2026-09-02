use std::collections::HashSet;

use battlement_types::ObjectId;

use crate::{
  LengthOrAuto, PanelScaleMode, PanelScreenMatchMode, PanelSettings, Position, Prop, Style,
  StyleValue, UiDocument, UiElement, UiNode, UiVisualElementProperties, elements::parts,
};

const MAXIMUM_HIERARCHY_DEPTH: usize = 256;
const MAXIMUM_IDENTITIES: usize = 100_000;
const MAXIMUM_STRING_BYTES: usize = 65_536;

/// The category of invariant violated by authored UI state.
///
/// Validation deliberately reports stable categories rather than exposing
/// implementation paths or Unity exceptions. Callers should correct the
/// authored document or panel settings before submitting them to a client.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiValidationError {
  /// A document host, document root, or element identity appears more than once.
  DuplicateObject,
  /// An identity does not resolve to the required object or relationship.
  InvalidReference,
  /// A hierarchy is too deep, too wide, or gives children to a leaf element.
  InvalidHierarchy,
  /// A property is nonfinite, out of range, duplicated, or incompatible with its mode.
  InvalidProperty,
}

/// Validates complete UI document trees and returns all reserved identities.
///
/// The returned set contains each document host ID, document root ID, and node
/// ID. Validation rejects duplicate identities across the complete collection;
/// empty or duplicate USS classes; duplicate event subscriptions; nonfinite
/// style numbers or colors; labels or buttons with children; more than 100,000
/// children on one node; and hierarchy depth beyond 256 edges.
///
/// # Errors
///
/// Returns the first [`UiValidationError`] encountered in document and child
/// order. No input value is modified.
pub fn validate_documents(
  documents: &[UiDocument],
) -> Result<HashSet<ObjectId>, UiValidationError> {
  let mut identities = HashSet::new();
  for document in documents {
    insert_identity(&mut identities, document.document_id)?;
    insert_identity(&mut identities, document.root_id)?;
    if document.element.usage_hints.is_some() {
      return Err(UiValidationError::InvalidProperty);
    }
    validate_visual(&document.element)?;
    for child in &document.children {
      validate_node(child, &mut identities, 1, None, false, false)?;
    }
  }
  if documents.iter().map(auto_focus_count).sum::<usize>() > 1 {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(identities)
}

fn auto_focus_count(document: &UiDocument) -> usize {
  usize::from(matches!(document.element.auto_focus, Prop::Set(true)))
    + document
      .children
      .iter()
      .map(node_auto_focus_count)
      .sum::<usize>()
}

fn node_auto_focus_count(node: &UiNode) -> usize {
  usize::from(matches!(
    node.element.visual_element().auto_focus,
    Prop::Set(true)
  )) + node
    .children
    .iter()
    .map(node_auto_focus_count)
    .sum::<usize>()
}

/// Validates a detached element subtree before a create command is executed.
///
/// The returned identities are unique within the subtree. Live-session identity
/// conflicts and the final depth beneath the selected parent must be checked by
/// the executor because they depend on current client state.
///
/// # Errors
///
/// Returns the first [`UiValidationError`] in preorder without modifying the
/// subtree.
pub fn validate_create_subtree(node: &UiNode) -> Result<HashSet<ObjectId>, UiValidationError> {
  let mut identities = HashSet::new();
  validate_node(node, &mut identities, 0, None, true, false)?;
  Ok(identities)
}

/// Validates sparse properties before applying an update to a live element.
///
/// Usage hints are rejected because Unity makes them read-only after an element
/// is attached to a panel.
///
/// # Errors
///
/// Returns [`UiValidationError::InvalidProperty`] for invalid common or
/// element-specific values and leaves the input unchanged.
pub fn validate_element_update(value: &UiElement) -> Result<(), UiValidationError> {
  if value.visual_element().usage_hints.is_some() {
    return Err(UiValidationError::InvalidProperty);
  }
  validate_element(value, false)
}

/// Validates one complete element value independently of hierarchy placement.
///
/// Unlike [`validate_element_update`], this accepts create-time usage hints and
/// is intended for executors that have merged a sparse update into live state.
///
/// # Errors
///
/// Returns the first invalid common or element-specific property without
/// modifying the supplied value.
pub fn validate_element_state(value: &UiElement) -> Result<(), UiValidationError> {
  validate_element(value, true)
}

/// Validates panel settings before Unity creates or configures a runtime panel.
///
/// Numeric fields must be finite, dimensions and density values must be
/// positive, normalized values must fall in `0..=1`, and `target_display` must
/// fall in `0..=7`. Each scale mode accepts nondefault values only for its own
/// fields. Dynamic-atlas sizes must be ordered nonzero powers of two, and atlas
/// filters must be unique.
///
/// # Errors
///
/// Returns [`UiValidationError::InvalidProperty`] when any setting violates
/// these requirements. No input value is modified.
pub fn validate_panel_settings(value: &PanelSettings) -> Result<(), UiValidationError> {
  let floats = [
    value.reference_sprite_pixels_per_unit,
    value.scale,
    value.reference_dpi,
    value.fallback_dpi,
    value.match_factor,
  ];
  if floats.iter().any(|number| !number.is_finite())
    || value.reference_sprite_pixels_per_unit <= 0.0
    || value.scale <= 0.0
    || value.reference_dpi <= 0.0
    || value.fallback_dpi <= 0.0
    || !(0.0..=1.0).contains(&value.match_factor)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if value.reference_resolution.width == 0
    || value.reference_resolution.height == 0
    || value.target_display > 7
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if value.target_texture.is_some()
    && (value.target_display != 0
      || value.render_mode != crate::PanelRenderMode::ScreenSpaceOverlay)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if value.scale_mode != PanelScaleMode::ConstantPixelSize && value.scale != 1.0 {
    return Err(UiValidationError::InvalidProperty);
  }
  if value.scale_mode != PanelScaleMode::ConstantPhysicalSize
    && (value.reference_dpi != 96.0 || value.fallback_dpi != 96.0)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  let reference_resolution_is_default =
    value.reference_resolution.width == 1200 && value.reference_resolution.height == 800;
  let screen_scaling_is_default = reference_resolution_is_default
    && value.screen_match_mode == PanelScreenMatchMode::MatchWidthOrHeight
    && value.match_factor == 0.0;
  if value.scale_mode != PanelScaleMode::ScaleWithScreenSize && !screen_scaling_is_default {
    return Err(UiValidationError::InvalidProperty);
  }
  for color in [
    value.color_clear_value.r,
    value.color_clear_value.g,
    value.color_clear_value.b,
    value.color_clear_value.a,
  ] {
    if !color.is_finite() || !(0.0..=1.0).contains(&color) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  let atlas = &value.dynamic_atlas;
  let atlas_sizes_are_powers = atlas.min_atlas_size.is_power_of_two()
    && atlas.max_atlas_size.is_power_of_two()
    && atlas.max_sub_texture_size.is_power_of_two();
  let atlas_sizes_are_ordered = atlas.min_atlas_size <= atlas.max_atlas_size
    && atlas.max_sub_texture_size <= atlas.max_atlas_size;
  if !atlas_sizes_are_powers || !atlas_sizes_are_ordered {
    return Err(UiValidationError::InvalidProperty);
  }
  if atlas.filters.iter().collect::<HashSet<_>>().len() != atlas.filters.len() {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_node(
  node: &UiNode,
  identities: &mut HashSet<ObjectId>,
  depth: usize,
  parent_kind: Option<crate::UiElementKind>,
  unplaced_root: bool,
  has_scroll_ancestor: bool,
) -> Result<(), UiValidationError> {
  insert_identity(identities, node.object_id)?;
  if depth > MAXIMUM_HIERARCHY_DEPTH || node.children.len() > MAXIMUM_IDENTITIES {
    return Err(UiValidationError::InvalidHierarchy);
  }
  if matches!(
    node.element,
    UiElement::Label(_)
      | UiElement::TextElement(_)
      | UiElement::TextField(_)
      | UiElement::Toggle(_)
      | UiElement::RadioButton(_)
      | UiElement::RadioButtonGroup(_)
      | UiElement::DropdownField(_)
      | UiElement::Slider(_)
      | UiElement::SliderInt(_)
      | UiElement::MinMaxSlider(_)
      | UiElement::ProgressBar(_)
      | UiElement::Button(_)
      | UiElement::RepeatButton(_)
      | UiElement::Image(_)
  ) && !node.children.is_empty()
  {
    return Err(UiValidationError::InvalidHierarchy);
  }
  let kind = node.element.kind();
  validate_layout_context(node, parent_kind, unplaced_root)?;
  if matches!(node.element.visual_element().sticky, Prop::Set(_))
    && !has_scroll_ancestor
    && !unplaced_root
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if parent_kind == Some(crate::UiElementKind::TabView) && kind != crate::UiElementKind::Tab {
    return Err(UiValidationError::InvalidHierarchy);
  }
  if kind == crate::UiElementKind::Tab
    && parent_kind != Some(crate::UiElementKind::TabView)
    && !unplaced_root
  {
    return Err(UiValidationError::InvalidHierarchy);
  }
  if parent_kind == Some(crate::UiElementKind::ToggleButtonGroup)
    && kind != crate::UiElementKind::Button
  {
    return Err(UiValidationError::InvalidHierarchy);
  }
  if let UiElement::ToggleButtonGroup(value) = &node.element {
    validate_toggle_button_group(value, node.children.len())?;
  }
  if let UiElement::TabView(value) = &node.element
    && matches!(value.selected_tab_index, Prop::Set(index) if index as usize >= node.children.len())
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if let UiElement::RepeatButton(value) = &node.element
    && (!matches!(value.delay_ms, Prop::Set(_)) || !matches!(value.interval_ms, Prop::Set(_)))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  validate_element(&node.element, true)?;
  if let Some(descriptor) = node.element.visual_element().motion.set_value()
    && descriptor.host_id != node.object_id
  {
    return Err(UiValidationError::InvalidReference);
  }
  for child in &node.children {
    validate_node(
      child,
      identities,
      depth + 1,
      Some(kind),
      false,
      has_scroll_ancestor || kind == crate::UiElementKind::ScrollView,
    )?;
  }
  Ok(())
}

fn validate_layout_context(
  node: &UiNode,
  parent_kind: Option<crate::UiElementKind>,
  unplaced_root: bool,
) -> Result<(), UiValidationError> {
  let visual = node.element.visual_element();
  let in_grid = parent_kind == Some(crate::UiElementKind::Grid);
  let in_stack = parent_kind == Some(crate::UiElementKind::Stack);
  if matches!(visual.grid_item, Prop::Set(_)) && !in_grid && !unplaced_root {
    return Err(UiValidationError::InvalidProperty);
  }
  if matches!(visual.stack_item, Prop::Set(_)) && !in_stack && !unplaced_root {
    return Err(UiValidationError::InvalidProperty);
  }
  if !in_grid && !in_stack {
    return Ok(());
  }
  let position_is_absolute = matches!(
    visual.style.position,
    Prop::Set(StyleValue::Value(Position::Absolute))
  );
  let offsets_are_automatic = [
    &visual.style.top,
    &visual.style.right,
    &visual.style.bottom,
    &visual.style.left,
  ]
  .into_iter()
  .all(layout_offset_is_automatic);
  if position_is_absolute || !offsets_are_automatic {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn layout_offset_is_automatic(value: &Prop<StyleValue<LengthOrAuto>>) -> bool {
  matches!(
    value,
    Prop::Unset
      | Prop::Reset
      | Prop::Set(StyleValue::Value(LengthOrAuto::Auto))
      | Prop::Set(StyleValue::Keyword { .. })
  )
}

fn validate_visual(visual: &crate::UiVisualElement) -> Result<(), UiValidationError> {
  validate_optional_string(visual.name.set_value().map(String::as_str), true)?;
  let mut classes = HashSet::new();
  if let Some(values) = visual.classes.set_value() {
    for class_name in values {
      validate_optional_string(Some(class_name), false)?;
      if !classes.insert(class_name) {
        return Err(UiValidationError::InvalidProperty);
      }
    }
  }
  if let Some(values) = visual.events.set_value() {
    if values.iter().collect::<HashSet<_>>().len() != values.len() {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(values) = visual.event_subscriptions.set_value() {
    if values.iter().collect::<HashSet<_>>().len() != values.len() {
      return Err(UiValidationError::InvalidProperty);
    }
    if values
      .iter()
      .any(|value| value.phase != crate::UiEventPhase::Target && !value.kind.propagates())
    {
      return Err(UiValidationError::InvalidProperty);
    }
    if visual.events.set_value().is_some_and(|shorthand| {
      values
        .iter()
        .any(|value| value.phase == crate::UiEventPhase::Target && shorthand.contains(&value.kind))
    }) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(values) = &visual.usage_hints {
    if values.iter().collect::<HashSet<_>>().len() != values.len() {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if visual
    .motion
    .set_value()
    .is_some_and(|descriptor| descriptor.validate().is_err())
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if visual
    .grid_item
    .set_value()
    .is_some_and(|value| !valid_grid_item(value))
    || visual
      .stack_item
      .set_value()
      .is_some_and(|value| !valid_stack_item(value))
    || visual
      .sticky
      .set_value()
      .is_some_and(|value| !valid_sticky(value))
    || visual
      .overlay_placement
      .set_value()
      .is_some_and(|value| !valid_overlay(value))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  let has_sticky = matches!(visual.sticky, Prop::Set(_));
  if has_sticky
    && (matches!(visual.stack_item, Prop::Set(_))
      || matches!(visual.overlay_placement, Prop::Set(_)))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if has_sticky
    && matches!(
      visual.style.position,
      Prop::Set(StyleValue::Value(Position::Absolute))
    )
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if let Some(overlay) = visual.overlay_placement.set_value()
    && !valid_overlay_style(&visual.style, overlay)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  validate_style(&visual.style)
}

fn validate_element(value: &UiElement, require_complete: bool) -> Result<(), UiValidationError> {
  validate_visual(value.visual_element())?;
  validate_parts(value, require_complete)?;
  match value {
    UiElement::Flex(value) => {
      validate_gap(value.row_gap.set_value())?;
      validate_gap(value.column_gap.set_value())?;
      validate_container_align(value.align_items.set_value())?;
    }
    UiElement::Grid(value) => {
      for track in value
        .columns
        .set_value()
        .into_iter()
        .flatten()
        .chain(value.rows.set_value().into_iter().flatten())
        .chain(value.auto_columns.set_value())
        .chain(value.auto_rows.set_value())
      {
        validate_grid_track(*track)?;
      }
      validate_gap(value.row_gap.set_value())?;
      validate_gap(value.column_gap.set_value())?;
      validate_container_align(value.align_items.set_value())?;
      validate_container_align(value.justify_items.set_value())?;
    }
    UiElement::Stack(value) => {
      validate_container_align(value.align_items.set_value())?;
      validate_container_align(value.justify_items.set_value())?;
    }
    _ => {}
  }
  if let UiElement::Image(image) = value {
    validate_image(image)?;
  }
  if let UiElement::ScrollView(scroll) = value {
    let values = [
      scroll.scroll_offset.set_value().map(|value| value.x),
      scroll.scroll_offset.set_value().map(|value| value.y),
      scroll.horizontal_page_size.set_value().copied(),
      scroll.vertical_page_size.set_value().copied(),
      scroll.mouse_wheel_scroll_size.set_value().copied(),
      scroll.scroll_deceleration_rate.set_value().copied(),
      scroll.elasticity.set_value().copied(),
    ];
    if values.into_iter().flatten().any(|value| !value.is_finite()) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let UiElement::Scroller(scroller) = value {
    let values = [
      scroller.low_value.set_value().copied(),
      scroller.high_value.set_value().copied(),
      scroller.value.set_value().copied(),
    ];
    if values.into_iter().flatten().any(|value| !value.is_finite()) {
      return Err(UiValidationError::InvalidProperty);
    }
    let supplied_reversed = scroller
      .low_value
      .set_value()
      .zip(scroller.high_value.set_value())
      .is_some_and(|(low, high)| low > high);
    let complete_reversed = require_complete && {
      let low = scroller.low_value.set_value().copied().unwrap_or(0.0);
      let high = scroller.high_value.set_value().copied().unwrap_or(0.0);
      low > high
    };
    if supplied_reversed || complete_reversed {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let UiElement::Slider(slider) = value {
    validate_optional_string(slider.label.set_value().map(String::as_str), true)?;
    let values = [
      slider.low_value.set_value().copied(),
      slider.high_value.set_value().copied(),
      slider.value.set_value().copied(),
      slider.page_size.set_value().copied(),
    ];
    if values.into_iter().flatten().any(|value| !value.is_finite())
      || slider
        .page_size
        .set_value()
        .is_some_and(|value| *value < 0.0)
    {
      return Err(UiValidationError::InvalidProperty);
    }
    let reversed = slider
      .low_value
      .set_value()
      .zip(slider.high_value.set_value())
      .is_some_and(|(low, high)| low > high);
    let complete_invalid = require_complete && {
      let low = slider.low_value.set_value().copied().unwrap_or(0.0);
      let high = slider.high_value.set_value().copied().unwrap_or(10.0);
      !(low..=high).contains(&slider.value.set_value().copied().unwrap_or(0.0))
    };
    if reversed || complete_invalid {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let UiElement::SliderInt(slider) = value {
    validate_optional_string(slider.label.set_value().map(String::as_str), true)?;
    if slider
      .page_size
      .set_value()
      .is_some_and(|value| !value.is_finite() || *value < 0.0)
    {
      return Err(UiValidationError::InvalidProperty);
    }
    let reversed = slider
      .low_value
      .set_value()
      .zip(slider.high_value.set_value())
      .is_some_and(|(low, high)| low > high);
    let complete_invalid = require_complete && {
      let low = slider.low_value.set_value().copied().unwrap_or(0);
      let high = slider.high_value.set_value().copied().unwrap_or(10);
      !(low..=high).contains(&slider.value.set_value().copied().unwrap_or(0))
    };
    if reversed || complete_invalid {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let UiElement::MinMaxSlider(slider) = value {
    validate_optional_string(slider.label.set_value().map(String::as_str), true)?;
    let low_limit = slider.low_limit.set_value().map(|value| match value {
      crate::LowerLimit::Unbounded => f32::MIN,
      crate::LowerLimit::Inclusive(value) => *value,
    });
    let high_limit = slider.high_limit.set_value().map(|value| match value {
      crate::UpperLimit::Unbounded => f32::MAX,
      crate::UpperLimit::Inclusive(value) => *value,
    });
    let values = [
      slider.min_value.set_value().copied(),
      slider.max_value.set_value().copied(),
      low_limit,
      high_limit,
    ];
    if values.into_iter().flatten().any(|value| !value.is_finite()) {
      return Err(UiValidationError::InvalidProperty);
    }
    let reversed_values = slider
      .min_value
      .set_value()
      .zip(slider.max_value.set_value())
      .is_some_and(|(min, max)| min > max);
    let reversed_limits = low_limit
      .zip(high_limit)
      .is_some_and(|(low, high)| low > high);
    let supplied_outside = slider
      .min_value
      .set_value()
      .copied()
      .zip(low_limit)
      .is_some_and(|(min, low)| min < low)
      || slider
        .max_value
        .set_value()
        .copied()
        .zip(high_limit)
        .is_some_and(|(max, high)| max > high);
    let complete_invalid = require_complete && {
      let low = low_limit.unwrap_or(f32::MIN);
      let high = high_limit.unwrap_or(f32::MAX);
      let min = slider.min_value.set_value().copied().unwrap_or(0.0);
      let max = slider.max_value.set_value().copied().unwrap_or(10.0);
      low > high || min > max || min < low || max > high
    };
    if reversed_values || reversed_limits || supplied_outside || complete_invalid {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let UiElement::ProgressBar(progress) = value {
    validate_optional_string(progress.title.set_value().map(String::as_str), true)?;
    let values = [
      progress.low_value.set_value().copied(),
      progress.high_value.set_value().copied(),
      progress.value.set_value().copied(),
    ];
    if values.into_iter().flatten().any(|value| !value.is_finite()) {
      return Err(UiValidationError::InvalidProperty);
    }
    let reversed = progress
      .low_value
      .set_value()
      .zip(progress.high_value.set_value())
      .is_some_and(|(low, high)| low > high);
    let supplied_outside = progress
      .value
      .set_value()
      .zip(progress.low_value.set_value())
      .is_some_and(|(selected, low)| selected < low)
      || progress
        .value
        .set_value()
        .zip(progress.high_value.set_value())
        .is_some_and(|(selected, high)| selected > high);
    let complete_invalid = require_complete && {
      let low = progress.low_value.set_value().copied().unwrap_or(0.0);
      let high = progress.high_value.set_value().copied().unwrap_or(100.0);
      !(low..=high).contains(&progress.value.set_value().copied().unwrap_or(0.0))
    };
    if reversed || supplied_outside || complete_invalid {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let UiElement::TextField(field) = value {
    validate_optional_string(field.label.set_value().map(String::as_str), true)?;
    validate_optional_string(field.value.set_value().map(String::as_str), true)?;
    validate_optional_string(field.placeholder.set_value().map(String::as_str), true)?;
    let text = field
      .value
      .set_value()
      .map(String::as_str)
      .or_else(|| (require_complete || matches!(field.value, Prop::Reset)).then_some(""));
    if let Some(text) = text {
      let length = text.encode_utf16().count();
      if field
        .cursor_index
        .set_value()
        .is_some_and(|index| *index as usize > length)
        || field
          .select_index
          .set_value()
          .is_some_and(|index| *index as usize > length)
      {
        return Err(UiValidationError::InvalidProperty);
      }
    }
  }
  if let UiElement::Toggle(toggle) = value {
    validate_optional_string(toggle.label.set_value().map(String::as_str), true)?;
    validate_optional_string(toggle.text.set_value().map(String::as_str), true)?;
  }
  if let UiElement::RadioButton(radio) = value {
    validate_optional_string(radio.label.set_value().map(String::as_str), true)?;
    validate_optional_string(radio.text.set_value().map(String::as_str), true)?;
  }
  if let UiElement::RadioButtonGroup(group) = value {
    validate_optional_string(group.label.set_value().map(String::as_str), true)?;
    let choices = group.choices.set_value().map_or(&[][..], Vec::as_slice);
    for choice in choices {
      validate_optional_string(Some(choice), true)?;
    }
    if (require_complete || !matches!(group.choices, Prop::Unset))
      && group
        .selected_index
        .set_value()
        .is_some_and(|index| *index as usize >= choices.len())
    {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let UiElement::ToggleButtonGroup(group) = value {
    validate_optional_string(group.label.set_value().map(String::as_str), true)?;
    validate_selected_indices(
      group
        .selected_indices
        .set_value()
        .map_or(&[][..], Vec::as_slice),
    )?;
    if matches!(group.multiple_selection, Prop::Set(false) | Prop::Reset)
      && group
        .selected_indices
        .set_value()
        .is_some_and(|values| values.len() > 1)
    {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let UiElement::DropdownField(field) = value {
    validate_optional_string(field.label.set_value().map(String::as_str), true)?;
    let choices = field.choices.set_value().map_or(&[][..], Vec::as_slice);
    for choice in choices {
      validate_optional_string(Some(choice), true)?;
    }
    if choices.iter().collect::<HashSet<_>>().len() != choices.len() {
      return Err(UiValidationError::InvalidProperty);
    }
    if let Some(selection) = field.selection.set_value() {
      validate_dropdown_choice(
        selection,
        choices,
        require_complete || !matches!(field.choices, Prop::Unset),
      )?;
    }
  }
  let text = match value {
    UiElement::Label(value) => value.text.set_value().map(String::as_str),
    UiElement::TextElement(value) => value.text.set_value().map(String::as_str),
    UiElement::TextField(value) => value.value.set_value().map(String::as_str),
    UiElement::Toggle(value) => value
      .text
      .set_value()
      .or(value.label.set_value())
      .map(String::as_str),
    UiElement::RadioButton(value) => value
      .text
      .set_value()
      .or(value.label.set_value())
      .map(String::as_str),
    UiElement::DropdownField(value) => value.label.set_value().map(String::as_str),
    UiElement::Slider(value) => value.label.set_value().map(String::as_str),
    UiElement::SliderInt(value) => value.label.set_value().map(String::as_str),
    UiElement::ProgressBar(value) => value.title.set_value().map(String::as_str),
    UiElement::Button(value) => value.text.set_value().map(String::as_str),
    UiElement::RepeatButton(value) => value.text.set_value().map(String::as_str),
    UiElement::GroupBox(value) => value.text.set_value().map(String::as_str),
    UiElement::PopupWindow(value) => value.text.set_value().map(String::as_str),
    UiElement::Tab(value) => value.text.set_value().map(String::as_str),
    _ => None,
  };
  validate_optional_string(text, true).and(match value {
    UiElement::RepeatButton(value)
      if require_complete
        && (matches!(value.delay_ms, Prop::Unset) || matches!(value.interval_ms, Prop::Unset)) =>
    {
      Err(UiValidationError::InvalidProperty)
    }
    _ => Ok(()),
  })
}

fn validate_gap(value: Option<&f32>) -> Result<(), UiValidationError> {
  if value.is_some_and(|value| !value.is_finite() || *value < 0.0) {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_container_align(value: Option<&crate::Align>) -> Result<(), UiValidationError> {
  if value == Some(&crate::Align::Auto) {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_grid_track(value: crate::GridTrack) -> Result<(), UiValidationError> {
  let valid = match value {
    crate::GridTrack::Px(value) => value.is_finite() && value >= 0.0,
    crate::GridTrack::Fraction(value) => value.is_finite() && value > 0.0,
    crate::GridTrack::Auto => true,
  };
  if !valid {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn valid_grid_item(value: &crate::GridItem) -> bool {
  let starts_are_positive =
    value.row.is_none_or(|value| value > 0) && value.column.is_none_or(|value| value > 0);
  let spans_are_positive = value.row_span > 0 && value.column_span > 0;
  if !starts_are_positive || !spans_are_positive {
    return false;
  }
  let rows_are_finite = value
    .row
    .is_none_or(|start| start.checked_add(value.row_span - 1).is_some());
  let columns_are_finite = value
    .column
    .is_none_or(|start| start.checked_add(value.column_span - 1).is_some());
  rows_are_finite && columns_are_finite
}

fn valid_stack_item(value: &crate::StackItem) -> bool {
  [value.top, value.right, value.bottom, value.left]
    .into_iter()
    .flatten()
    .all(|value| value.is_finite() && value >= 0.0)
}

fn valid_sticky(value: &crate::Sticky) -> bool {
  let horizontal_edges = usize::from(value.left.is_some()) + usize::from(value.right.is_some());
  let vertical_edges = usize::from(value.top.is_some()) + usize::from(value.bottom.is_some());
  let has_edge = horizontal_edges + vertical_edges > 0;
  let compatible_edges = horizontal_edges <= 1 && vertical_edges <= 1;
  let finite = [value.top, value.right, value.bottom, value.left]
    .into_iter()
    .flatten()
    .all(f32::is_finite);
  has_edge && compatible_edges && finite
}

fn valid_overlay(value: &crate::OverlayPlacement) -> bool {
  let crate::OverlayPlacement::Popover { placement, .. } = value else {
    return true;
  };
  placement.main_offset.is_finite()
    && placement.cross_offset.is_finite()
    && placement.collision_padding.is_finite()
    && placement.collision_padding >= 0.0
}

fn valid_overlay_style(style: &Style, overlay: &crate::OverlayPlacement) -> bool {
  let forbidden = [
    &style.margin_top,
    &style.margin_right,
    &style.margin_bottom,
    &style.margin_left,
    &style.top,
    &style.right,
    &style.bottom,
    &style.left,
  ];
  if forbidden
    .into_iter()
    .any(|value| matches!(value, Prop::Set(_)))
    || matches!(style.position, Prop::Set(_))
    || matches!(style.display, Prop::Set(_))
    || matches!(style.visibility, Prop::Set(_))
  {
    return false;
  }
  if matches!(overlay, crate::OverlayPlacement::Popover { .. }) {
    return true;
  }
  [
    &style.width,
    &style.height,
    &style.min_width,
    &style.min_height,
    &style.max_width,
    &style.max_height,
  ]
  .into_iter()
  .all(|value| !matches!(value, Prop::Set(_)))
}

fn validate_parts(value: &UiElement, require_complete: bool) -> Result<(), UiValidationError> {
  let Some(part_styles) = parts::styles(value) else {
    return Ok(());
  };
  let mut keys = HashSet::new();
  for part in part_styles {
    let indexed = matches!(
      part.part,
      parts::Part::RadioButtonGroupOption
        | parts::Part::RadioButtonGroupOptionCheckmarkBackground
        | parts::Part::RadioButtonGroupOptionCheckmark
        | parts::Part::RadioButtonGroupOptionText
    );
    if !keys.insert((part.part, part.index))
      || !parts::belongs_to(value, part.part)
      || indexed != part.index.is_some()
      || part.index.is_some_and(|index| {
        !matches!(
            value,
            UiElement::RadioButtonGroup(group)
                if group
                    .choices
                    .set_value()
                    .is_some_and(|choices| (index as usize) < choices.len())
        )
      })
      || (require_complete && !parts::exists_in_complete_state(value, part.part))
    {
      return Err(UiValidationError::InvalidProperty);
    }
    validate_style(&part.style)?;
  }
  Ok(())
}

fn validate_dropdown_choice(
  selection: &crate::Choice,
  choices: &[String],
  validate_against_choices: bool,
) -> Result<(), UiValidationError> {
  match (selection.index, selection.value.as_deref()) {
    (None, None) => Ok(()),
    (Some(index), Some(value))
      if !validate_against_choices
        || choices
          .get(index as usize)
          .is_some_and(|choice| choice == value) =>
    {
      Ok(())
    }
    _ => Err(UiValidationError::InvalidProperty),
  }
}

fn validate_toggle_button_group(
  value: &crate::UiToggleButtonGroup,
  child_count: usize,
) -> Result<(), UiValidationError> {
  if child_count > 64 {
    return Err(UiValidationError::InvalidHierarchy);
  }
  let default_selected = [0];
  let selected: &[u32] = match value.selected_indices.set_value() {
    Some(values) => values,
    None if child_count == 0 || matches!(value.allow_empty_selection, Prop::Set(true)) => &[],
    None => &default_selected,
  };
  validate_selected_indices(selected)?;
  if selected.iter().any(|index| *index as usize >= child_count) {
    return Err(UiValidationError::InvalidProperty);
  }
  if !matches!(value.multiple_selection, Prop::Set(true)) && selected.len() > 1 {
    return Err(UiValidationError::InvalidProperty);
  }
  if child_count > 0
    && !matches!(value.allow_empty_selection, Prop::Set(true))
    && selected.is_empty()
  {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_selected_indices(values: &[u32]) -> Result<(), UiValidationError> {
  if values.windows(2).any(|pair| pair[0] >= pair[1]) {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_image(value: &crate::UiImage) -> Result<(), UiValidationError> {
  if matches!(value.source, Prop::Set(crate::ImageSource::Sprite(_)))
    && matches!(value.source_rect, Prop::Set(_))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if let Prop::Set(rect) = value.source_rect {
    validate_rect(rect, false)?;
  }
  if let Prop::Set(rect) = value.uv {
    validate_rect(rect, true)?;
  }
  if let Prop::Set(color) = value.tint_color {
    if [color.r, color.g, color.b, color.a]
      .into_iter()
      .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(&channel))
    {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  Ok(())
}

fn validate_rect(value: battlement_types::Rect, normalized: bool) -> Result<(), UiValidationError> {
  let fields = [value.x, value.y, value.width, value.height];
  if fields.into_iter().any(|field| !field.is_finite())
    || value.x < 0.0
    || value.y < 0.0
    || value.width < 0.0
    || value.height < 0.0
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if normalized
    && (value.x < 0.0
      || value.y < 0.0
      || value.x + value.width > 1.0
      || value.y + value.height > 1.0)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_optional_string(
  value: Option<&str>,
  allow_empty: bool,
) -> Result<(), UiValidationError> {
  let too_long = value.is_some_and(|text| text.len() > MAXIMUM_STRING_BYTES);
  let invalid_empty = value.is_some_and(str::is_empty) && !allow_empty;
  if too_long || invalid_empty {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn insert_identity(
  identities: &mut HashSet<ObjectId>,
  object_id: ObjectId,
) -> Result<(), UiValidationError> {
  if object_id.as_uuid().is_nil() {
    return Err(UiValidationError::InvalidReference);
  }
  if !identities.insert(object_id) {
    return Err(UiValidationError::DuplicateObject);
  }
  if identities.len() > MAXIMUM_IDENTITIES {
    return Err(UiValidationError::InvalidHierarchy);
  }
  Ok(())
}

fn validate_style(value: &Style) -> Result<(), UiValidationError> {
  validate_prop_length(&value.font_size, true)?;
  if prop_concrete(&value.font_size).is_some_and(|length| match length {
    crate::Length::Px(number) | crate::Length::Percent(number) => *number <= 0.0,
  }) {
    return Err(UiValidationError::InvalidProperty);
  }
  for property in [
    &value.letter_spacing,
    &value.unity_paragraph_spacing,
    &value.word_spacing,
  ] {
    validate_prop_length(property, false)?;
  }
  for property in [
    &value.width,
    &value.height,
    &value.min_width,
    &value.min_height,
  ] {
    validate_prop_length_or_auto(property, true)?;
  }
  for property in [&value.max_width, &value.max_height] {
    validate_prop_length_or_auto(property, true)?;
  }
  for property in [
    &value.bottom,
    &value.flex_basis,
    &value.left,
    &value.margin_bottom,
    &value.margin_left,
    &value.margin_right,
    &value.margin_top,
    &value.right,
    &value.top,
  ] {
    validate_prop_length_or_auto(property, false)?;
  }
  for property in [
    &value.padding_bottom,
    &value.padding_left,
    &value.padding_right,
    &value.padding_top,
  ] {
    validate_prop_length(property, true)?;
  }
  for property in [
    &value.border_bottom_left_radius,
    &value.border_bottom_right_radius,
    &value.border_top_left_radius,
    &value.border_top_right_radius,
  ] {
    validate_prop_length(property, true)?;
  }
  for property in [
    &value.border_bottom_width,
    &value.border_left_width,
    &value.border_right_width,
    &value.border_top_width,
    &value.flex_grow,
    &value.flex_shrink,
  ] {
    if prop_concrete(property).is_some_and(|number| !number.0.is_finite() || number.0 < 0.0) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if prop_concrete(&value.opacity)
    .is_some_and(|number| !number.0.is_finite() || !(0.0..=1.0).contains(&number.0))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if prop_concrete(&value.unity_slice_scale)
    .is_some_and(|number| !number.0.is_finite() || number.0 <= 0.0)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if prop_concrete(&value.unity_text_outline_width)
    .is_some_and(|number| !number.0.is_finite() || number.0 < 0.0)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if let Some(crate::TextShadow {
    x,
    y,
    blur_radius,
    color,
  }) = prop_concrete(&value.text_shadow)
  {
    if !x.is_finite() || !y.is_finite() || !blur_radius.is_finite() || *blur_radius < 0.0 {
      return Err(UiValidationError::InvalidProperty);
    }
    validate_color(color)?;
  }
  if let Some(crate::TextAutoSize::BestFit { min_size, max_size }) =
    prop_concrete(&value.unity_text_auto_size)
  {
    if !min_size.is_finite() || !max_size.is_finite() || *min_size <= 0.0 || min_size > max_size {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  for property in [
    &value.unity_slice_bottom,
    &value.unity_slice_left,
    &value.unity_slice_right,
    &value.unity_slice_top,
  ] {
    if prop_concrete(property).is_some_and(|number| *number < 0) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(crate::AspectRatio::Ratio { width, height }) = prop_concrete(&value.aspect_ratio) {
    let valid_components = width.is_finite() && height.is_finite();
    let valid_range = *width > 0.0 && *height > 0.0;
    if !valid_components || !valid_range || !(width / height).is_finite() {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(position) = prop_concrete(&value.background_position_x) {
    validate_concrete_length(&position.offset, false)?;
    if !matches!(
      position.keyword,
      crate::BackgroundPositionKeyword::Left
        | crate::BackgroundPositionKeyword::Center
        | crate::BackgroundPositionKeyword::Right
    ) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(position) = prop_concrete(&value.background_position_y) {
    validate_concrete_length(&position.offset, false)?;
    if !matches!(
      position.keyword,
      crate::BackgroundPositionKeyword::Top
        | crate::BackgroundPositionKeyword::Center
        | crate::BackgroundPositionKeyword::Bottom
    ) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(crate::BackgroundSize::Axes { x, y }) = prop_concrete(&value.background_size) {
    validate_concrete_length_or_auto(x, true)?;
    validate_concrete_length_or_auto(y, true)?;
  }
  if let Some(crate::Cursor::Texture { hotspot, .. }) = prop_concrete(&value.cursor) {
    let finite = hotspot.x.is_finite() && hotspot.y.is_finite();
    let nonnegative = hotspot.x >= 0.0 && hotspot.y >= 0.0;
    if !finite || !nonnegative {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(filters) = prop_concrete(&value.filter) {
    for function in filters.as_slice() {
      match function {
        crate::FilterFunction::Tint(color) => validate_color(color)?,
        crate::FilterFunction::Opacity(number)
        | crate::FilterFunction::Invert(number)
        | crate::FilterFunction::Grayscale(number)
        | crate::FilterFunction::Sepia(number)
        | crate::FilterFunction::Blur(number)
        | crate::FilterFunction::Contrast(number)
        | crate::FilterFunction::HueRotate(number) => {
          if !number.is_finite() {
            return Err(UiValidationError::InvalidProperty);
          }
        }
      }
    }
  }
  if let Some(rotation) = prop_concrete(&value.rotate) {
    let axis = [rotation.x, rotation.y, rotation.z];
    if axis.into_iter().any(|number| !number.is_finite())
      || !rotation.degrees.is_finite()
      || axis == [0.0, 0.0, 0.0]
    {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(scale) = prop_concrete(&value.scale) {
    if !scale.x.is_finite() || !scale.y.is_finite() {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(origin) = prop_concrete(&value.transform_origin) {
    validate_concrete_length(&origin.x, false)?;
    validate_concrete_length(&origin.y, false)?;
    if !origin.z.is_finite() {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(translation) = prop_concrete(&value.translate) {
    validate_concrete_length(&translation.x, false)?;
    validate_concrete_length(&translation.y, false)?;
    if !translation.z.is_finite() {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  validate_transition_times(&value.transition_delay, false)?;
  validate_transition_times(&value.transition_duration, true)?;
  for color in [
    &value.background_color,
    &value.border_bottom_color,
    &value.border_left_color,
    &value.border_right_color,
    &value.border_top_color,
    &value.color,
    &value.unity_background_image_tint_color,
    &value.unity_text_outline_color,
  ]
  .into_iter()
  .filter_map(prop_concrete)
  {
    if [color.r, color.g, color.b, color.a]
      .into_iter()
      .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(&channel))
    {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  Ok(())
}

fn validate_transition_times(
  value: &crate::Prop<crate::StyleValue<crate::TransitionList<crate::TimeValue>>>,
  nonnegative: bool,
) -> Result<(), UiValidationError> {
  let Some(values) = prop_concrete(value) else {
    return Ok(());
  };
  if values
    .as_slice()
    .iter()
    .any(|value| !value.0.is_finite() || nonnegative && value.0 < 0.0)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_color(value: &battlement_types::Color) -> Result<(), UiValidationError> {
  if [value.r, value.g, value.b, value.a]
    .into_iter()
    .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(&channel))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn concrete<T>(value: Option<&crate::StyleValue<T>>) -> Option<&T> {
  match value {
    Some(crate::StyleValue::Value(value)) => Some(value),
    Some(crate::StyleValue::Keyword { .. }) | None => None,
  }
}

fn prop_concrete<T>(value: &crate::Prop<crate::StyleValue<T>>) -> Option<&T> {
  concrete(value.set_value())
}

fn validate_prop_length(
  value: &crate::Prop<crate::StyleValue<crate::Length>>,
  nonnegative: bool,
) -> Result<(), UiValidationError> {
  let Some(value) = prop_concrete(value) else {
    return Ok(());
  };
  validate_concrete_length(value, nonnegative)
}

fn validate_concrete_length(
  value: &crate::Length,
  nonnegative: bool,
) -> Result<(), UiValidationError> {
  let number = match value {
    crate::Length::Px(value) | crate::Length::Percent(value) => *value,
  };
  if !number.is_finite() || nonnegative && number < 0.0 {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_prop_length_or_auto(
  value: &crate::Prop<crate::StyleValue<crate::LengthOrAuto>>,
  nonnegative: bool,
) -> Result<(), UiValidationError> {
  let Some(value) = prop_concrete(value) else {
    return Ok(());
  };
  validate_concrete_length_or_auto(value, nonnegative)
}

fn validate_concrete_length_or_auto(
  value: &crate::LengthOrAuto,
  nonnegative: bool,
) -> Result<(), UiValidationError> {
  let number = match value {
    crate::LengthOrAuto::Px(value) | crate::LengthOrAuto::Percent(value) => Some(*value),
    crate::LengthOrAuto::Auto => None,
  };
  if number.is_some_and(|number| !number.is_finite() || nonnegative && number < 0.0) {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}
