use std::collections::HashSet;

use battlement_types::ObjectId;

use crate::{
    PanelScaleMode, PanelScreenMatchMode, PanelSettings, Style, UiDocument, UiElement, UiNode,
    VisualElementProperties,
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
            validate_node(child, &mut identities, 1)?;
        }
    }
    Ok(identities)
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
    validate_node(node, &mut identities, 0)?;
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
    validate_element(value)
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
    validate_element(value)
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
) -> Result<(), UiValidationError> {
    insert_identity(identities, node.object_id)?;
    if depth > MAXIMUM_HIERARCHY_DEPTH || node.children.len() > MAXIMUM_IDENTITIES {
        return Err(UiValidationError::InvalidHierarchy);
    }
    if matches!(
        node.element,
        UiElement::Label(_) | UiElement::Button(_) | UiElement::Image(_)
    ) && !node.children.is_empty()
    {
        return Err(UiValidationError::InvalidHierarchy);
    }
    validate_element(&node.element)?;
    for child in &node.children {
        validate_node(child, identities, depth + 1)?;
    }
    Ok(())
}

fn validate_visual(visual: &crate::VisualElement) -> Result<(), UiValidationError> {
    validate_optional_string(visual.name.as_deref(), true)?;
    let mut classes = HashSet::new();
    if let Some(values) = &visual.classes {
        for class_name in values {
            validate_optional_string(Some(class_name), false)?;
            if !classes.insert(class_name) {
                return Err(UiValidationError::InvalidProperty);
            }
        }
    }
    if let Some(values) = &visual.events {
        if values.iter().collect::<HashSet<_>>().len() != values.len() {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    if let Some(values) = &visual.usage_hints {
        if values.iter().collect::<HashSet<_>>().len() != values.len() {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    validate_style(&visual.style)
}

fn validate_element(value: &UiElement) -> Result<(), UiValidationError> {
    validate_visual(value.visual_element())?;
    if let UiElement::Image(image) = value {
        validate_image(image)?;
    }
    let text = match value {
        UiElement::Label(value) => value.text.as_deref(),
        UiElement::Button(value) => value.text.as_deref(),
        _ => None,
    };
    validate_optional_string(text, true)
}

fn validate_image(value: &crate::Image) -> Result<(), UiValidationError> {
    if matches!(value.source, Some(crate::ImageSource::Sprite(_))) && value.source_rect.is_some() {
        return Err(UiValidationError::InvalidProperty);
    }
    if let Some(rect) = value.source_rect {
        validate_rect(rect, false)?;
    }
    if let Some(rect) = value.uv {
        validate_rect(rect, true)?;
    }
    if let Some(color) = value.tint_color {
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
    let floats = [
        value.width,
        value.height,
        value.flex_grow,
        value.padding,
        value.margin,
        value.font_size,
    ];
    if floats
        .into_iter()
        .flatten()
        .any(|number| !number.is_finite())
    {
        return Err(UiValidationError::InvalidProperty);
    }
    for color in [value.background_color, value.color].into_iter().flatten() {
        if [color.r, color.g, color.b, color.a]
            .into_iter()
            .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(&channel))
        {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    Ok(())
}
