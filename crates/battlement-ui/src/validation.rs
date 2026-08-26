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
        UiElement::Label(_)
            | UiElement::TextElement(_)
            | UiElement::Button(_)
            | UiElement::RepeatButton(_)
            | UiElement::Image(_)
    ) && !node.children.is_empty()
    {
        return Err(UiValidationError::InvalidHierarchy);
    }
    validate_element(&node.element, true)?;
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

fn validate_element(value: &UiElement, require_complete: bool) -> Result<(), UiValidationError> {
    validate_visual(value.visual_element())?;
    if let UiElement::Image(image) = value {
        validate_image(image)?;
    }
    let text = match value {
        UiElement::Label(value) => value.text.as_deref(),
        UiElement::TextElement(value) => value.text.as_deref(),
        UiElement::Button(value) => value.text.as_deref(),
        UiElement::RepeatButton(value) => value.text.as_deref(),
        UiElement::GroupBox(value) => value.text.as_deref(),
        UiElement::PopupWindow(value) => value.text.as_deref(),
        _ => None,
    };
    validate_optional_string(text, true).and_then(|()| match value {
        UiElement::RepeatButton(value)
            if require_complete && (value.delay_ms.is_none() || value.interval_ms.is_none()) =>
        {
            Err(UiValidationError::InvalidProperty)
        }
        _ => Ok(()),
    })
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
    validate_length(value.font_size.as_ref(), true)?;
    if concrete(value.font_size.as_ref()).is_some_and(|length| match length {
        crate::Length::Px(number) | crate::Length::Percent(number) => *number <= 0.0,
    }) {
        return Err(UiValidationError::InvalidProperty);
    }
    for property in [
        &value.letter_spacing,
        &value.unity_paragraph_spacing,
        &value.word_spacing,
    ] {
        validate_length(property.as_ref(), false)?;
    }
    for property in [
        &value.width,
        &value.height,
        &value.min_width,
        &value.min_height,
    ] {
        validate_length_or_auto(property.as_ref(), true)?;
    }
    for property in [&value.max_width, &value.max_height] {
        validate_length_or_auto(property.as_ref(), true)?;
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
        validate_length_or_auto(property.as_ref(), false)?;
    }
    for property in [
        &value.border_bottom_left_radius,
        &value.border_bottom_right_radius,
        &value.border_top_left_radius,
        &value.border_top_right_radius,
        &value.padding_bottom,
        &value.padding_left,
        &value.padding_right,
        &value.padding_top,
    ] {
        validate_length(property.as_ref(), true)?;
    }
    for property in [
        &value.border_bottom_width,
        &value.border_left_width,
        &value.border_right_width,
        &value.border_top_width,
        &value.flex_grow,
        &value.flex_shrink,
    ] {
        if concrete(property.as_ref()).is_some_and(|number| !number.0.is_finite() || number.0 < 0.0)
        {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    if concrete(value.opacity.as_ref())
        .is_some_and(|number| !number.0.is_finite() || !(0.0..=1.0).contains(&number.0))
    {
        return Err(UiValidationError::InvalidProperty);
    }
    if concrete(value.unity_slice_scale.as_ref())
        .is_some_and(|number| !number.0.is_finite() || number.0 <= 0.0)
    {
        return Err(UiValidationError::InvalidProperty);
    }
    if concrete(value.unity_text_outline_width.as_ref())
        .is_some_and(|number| !number.0.is_finite() || number.0 < 0.0)
    {
        return Err(UiValidationError::InvalidProperty);
    }
    if let Some(crate::TextShadow {
        x,
        y,
        blur_radius,
        color,
    }) = concrete(value.text_shadow.as_ref())
    {
        if !x.is_finite() || !y.is_finite() || !blur_radius.is_finite() || *blur_radius < 0.0 {
            return Err(UiValidationError::InvalidProperty);
        }
        validate_color(color)?;
    }
    if let Some(crate::TextAutoSize::BestFit { min_size, max_size }) =
        concrete(value.unity_text_auto_size.as_ref())
    {
        if !min_size.is_finite() || !max_size.is_finite() || *min_size <= 0.0 || min_size > max_size
        {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    for property in [
        &value.unity_slice_bottom,
        &value.unity_slice_left,
        &value.unity_slice_right,
        &value.unity_slice_top,
    ] {
        if concrete(property.as_ref()).is_some_and(|number| *number < 0) {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    if let Some(crate::AspectRatio::Ratio { width, height }) = concrete(value.aspect_ratio.as_ref())
    {
        let valid_components = width.is_finite() && height.is_finite();
        let valid_range = *width > 0.0 && *height > 0.0;
        if !valid_components || !valid_range || !(width / height).is_finite() {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    if let Some(position) = concrete(value.background_position_x.as_ref()) {
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
    if let Some(position) = concrete(value.background_position_y.as_ref()) {
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
    if let Some(crate::BackgroundSize::Axes { x, y }) = concrete(value.background_size.as_ref()) {
        validate_concrete_length_or_auto(x, true)?;
        validate_concrete_length_or_auto(y, true)?;
    }
    if let Some(crate::Cursor::Texture { hotspot, .. }) = concrete(value.cursor.as_ref()) {
        let finite = hotspot.x.is_finite() && hotspot.y.is_finite();
        let nonnegative = hotspot.x >= 0.0 && hotspot.y >= 0.0;
        if !finite || !nonnegative {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    if let Some(filters) = concrete(value.filter.as_ref()) {
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
    if let Some(rotation) = concrete(value.rotate.as_ref()) {
        let axis = [rotation.x, rotation.y, rotation.z];
        if axis.into_iter().any(|number| !number.is_finite())
            || !rotation.degrees.is_finite()
            || axis == [0.0, 0.0, 0.0]
        {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    if let Some(scale) = concrete(value.scale.as_ref()) {
        if !scale.x.is_finite() || !scale.y.is_finite() {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    if let Some(origin) = concrete(value.transform_origin.as_ref()) {
        validate_concrete_length(&origin.x, false)?;
        validate_concrete_length(&origin.y, false)?;
        if !origin.z.is_finite() {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    if let Some(translation) = concrete(value.translate.as_ref()) {
        validate_concrete_length(&translation.x, false)?;
        validate_concrete_length(&translation.y, false)?;
        if !translation.z.is_finite() {
            return Err(UiValidationError::InvalidProperty);
        }
    }
    validate_transition_times(value.transition_delay.as_ref(), false)?;
    validate_transition_times(value.transition_duration.as_ref(), true)?;
    for color in [
        &value.background_color,
        &value.border_bottom_color,
        &value.border_left_color,
        &value.border_right_color,
        &value.border_top_color,
        &value.color,
        &value.unity_background_image_tint_color,
    ]
    .into_iter()
    .filter_map(|value| concrete(value.as_ref()))
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
    value: Option<&crate::StyleValue<crate::TransitionList<crate::TimeValue>>>,
    nonnegative: bool,
) -> Result<(), UiValidationError> {
    let Some(values) = concrete(value) else {
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

fn validate_length(
    value: Option<&crate::StyleValue<crate::Length>>,
    nonnegative: bool,
) -> Result<(), UiValidationError> {
    let Some(value) = concrete(value) else {
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

fn validate_length_or_auto(
    value: Option<&crate::StyleValue<crate::LengthOrAuto>>,
    nonnegative: bool,
) -> Result<(), UiValidationError> {
    let Some(value) = concrete(value) else {
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
