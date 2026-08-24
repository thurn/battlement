use battlement_types::Color;
use serde::{Deserialize, Serialize};

/// Style values applied directly to a visual element.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Style {
    /// Background color of the element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<Color>,
    /// Foreground color inherited by text rendered by this element and its
    /// descendants unless a descendant overrides it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Fixed width of the element in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    /// Fixed height of the element in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    /// Proportion of remaining space assigned to this item relative to sibling
    /// items with a positive growth factor in the same flex container.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flex_grow: Option<f32>,
    /// Main-axis direction used to arrange this element's children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flex_direction: Option<FlexDirection>,
    /// Space in pixels inserted on every side between the element's border and
    /// its content, reducing the area available to children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<f32>,
    /// Space in pixels reserved on every side outside the element's border,
    /// separating it from neighboring layout items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<f32>,
    /// Font size in pixels inherited by descendant text unless overridden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
}

impl Style {
    /// Creates a set of style values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges the values from `value` into this style.
    ///
    /// A property set by `value` replaces the corresponding property in this
    /// style. Properties left unset by `value` preserve their existing values.
    #[must_use]
    pub fn merge(mut self, value: Self) -> Self {
        self.background_color = value.background_color.or(self.background_color);
        self.color = value.color.or(self.color);
        self.width = value.width.or(self.width);
        self.height = value.height.or(self.height);
        self.flex_grow = value.flex_grow.or(self.flex_grow);
        self.flex_direction = value.flex_direction.or(self.flex_direction);
        self.padding = value.padding.or(self.padding);
        self.margin = value.margin.or(self.margin);
        self.font_size = value.font_size.or(self.font_size);
        self
    }

    /// Paints `value` behind the element's content and padding area.
    #[must_use]
    pub fn background_color(mut self, value: Color) -> Self {
        self.background_color = Some(value);
        self
    }

    /// Sets the text color inherited by this element and its descendants unless
    /// a descendant supplies its own color.
    #[must_use]
    pub fn color(mut self, value: Color) -> Self {
        self.color = Some(value);
        self
    }

    /// Constrains the element's layout-box width to `value` pixels.
    #[must_use]
    pub fn width(mut self, value: f32) -> Self {
        self.width = Some(value);
        self
    }

    /// Constrains the element's layout-box height to `value` pixels.
    #[must_use]
    pub fn height(mut self, value: f32) -> Self {
        self.height = Some(value);
        self
    }

    /// Specifies how this item grows relative to siblings with positive growth
    /// factors when their flex container has remaining main-axis space.
    #[must_use]
    pub fn flex_grow(mut self, value: f32) -> Self {
        self.flex_grow = Some(value);
        self
    }

    /// Selects the main axis along which this element arranges its children.
    #[must_use]
    pub fn flex_direction(mut self, value: FlexDirection) -> Self {
        self.flex_direction = Some(value);
        self
    }

    /// Inserts `value` pixels on every side between the element's border and
    /// its content.
    #[must_use]
    pub fn padding(mut self, value: f32) -> Self {
        self.padding = Some(value);
        self
    }

    /// Reserves `value` pixels on every side outside the element's border.
    #[must_use]
    pub fn margin(mut self, value: f32) -> Self {
        self.margin = Some(value);
        self
    }

    /// Sets the inherited text size to `value` pixels.
    #[must_use]
    pub fn font_size(mut self, value: f32) -> Self {
        self.font_size = Some(value);
        self
    }

    /// Returns whether the style contributes no inline overrides to the wire
    /// payload.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Main-axis direction used by a flex container to arrange its children.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FlexDirection {
    /// Places children vertically from top to bottom along the main axis.
    Column,
    /// Places children horizontally from left to right along the main axis.
    Row,
}
