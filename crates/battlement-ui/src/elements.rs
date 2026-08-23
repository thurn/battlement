use battlement_types::{Color, ObjectId};
use serde::{Deserialize, Serialize};

macro_rules! impl_common_visual_element_methods {
    () => {
        /// Returns the stable identity used to correlate this element with its
        /// Unity counterpart across document updates and UI events.
        #[must_use]
        pub fn object_id(&self) -> ObjectId {
            self.common().required_object_id()
        }

        /// Assigns the Unity element name used by name-based queries and USS ID
        /// selectors.
        ///
        /// Names are optional, but should be unique within a document when they
        /// are used as selectors or lookup keys.
        #[must_use]
        pub fn name(mut self, value: impl Into<String>) -> Self {
            self.common_mut().name = value.into();
            self
        }

        /// Replaces the element's authored inline style state.
        ///
        /// Inline values take precedence over matching stylesheet rules in
        /// Unity. Properties left unset remain available to USS, inheritance,
        /// and Unity's defaults.
        #[must_use]
        pub fn style(mut self, value: Style) -> Self {
            self.common_mut().style = value;
            self
        }
    };
}

/// A serializable UI Toolkit element in a Battlement document hierarchy.
///
/// Each variant identifies the concrete Unity element that the host creates.
/// The enum is the recursive child type, while [`VisualElement`], [`Box`], and
/// [`Label`] are the concrete builders used to author individual nodes. The
/// variant names are also the stable JSON discriminators consumed by Unity.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum UiElement {
    /// A general-purpose Unity `VisualElement` without control-specific behavior.
    VisualElement(VisualElement),
    /// A Unity `Box` container with the standard box USS class and presentation.
    Box(Box),
    /// A Unity `Label` that displays non-editable text.
    Label(Label),
}

impl UiElement {
    pub(crate) fn object_id(&self) -> ObjectId {
        self.common().required_object_id()
    }

    pub(crate) fn common(&self) -> &CommonVisualElement {
        match self {
            Self::VisualElement(value) => value.common(),
            Self::Box(value) => value.common(),
            Self::Label(value) => value.common(),
        }
    }
}

/// Builds Unity's general-purpose UI Toolkit `VisualElement`.
///
/// A visual element is the base layout, styling, and hierarchy node used by UI
/// Toolkit. It has no control-specific behavior or built-in box presentation,
/// making it suitable for structural containers and custom styled regions. Add
/// children in logical display order; Unity lays them out according to this
/// element's flex properties unless explicit positioning is introduced later.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VisualElement {
    #[serde(flatten)]
    common: CommonVisualElement,
}

impl VisualElement {
    /// Creates a plain visual element with a newly generated stable identity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_id(ObjectId::new_v4())
    }

    /// Creates a plain visual element with an explicit stable identity.
    ///
    /// Use this when application state must retain the same element identity
    /// across successive document snapshots.
    #[must_use]
    pub fn with_id(object_id: ObjectId) -> Self {
        Self {
            common: CommonVisualElement::new(object_id),
        }
    }

    impl_common_visual_element_methods!();

    /// Appends one logical child after the element's existing children.
    ///
    /// The concrete builder is converted into [`UiElement`] automatically, and
    /// child order is preserved in the serialized hierarchy.
    #[must_use]
    pub fn child(mut self, value: impl Into<UiElement>) -> Self {
        self.common_mut().children.push(value.into());
        self
    }

    /// Appends logical children in iterator order after existing children.
    #[must_use]
    pub fn children<I, T>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<UiElement>,
    {
        self.common_mut()
            .children
            .extend(values.into_iter().map(Into::into));
        self
    }

    fn common(&self) -> &CommonVisualElement {
        &self.common
    }

    fn common_mut(&mut self) -> &mut CommonVisualElement {
        &mut self.common
    }
}

impl Default for VisualElement {
    fn default() -> Self {
        Self::new()
    }
}

impl From<VisualElement> for UiElement {
    fn from(value: VisualElement) -> Self {
        Self::VisualElement(value)
    }
}

/// Builds Unity's UI Toolkit `Box` container.
///
/// A box has the same hierarchy and flex-layout capabilities as a plain
/// [`VisualElement`], but Unity also assigns its standard box USS class. The
/// active Unity theme can therefore give it a background, border, and spacing
/// distinct from an unstyled container. Use a box when that semantic and visual
/// grouping is desired; use [`VisualElement`] for neutral structure.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Box {
    #[serde(flatten)]
    common: CommonVisualElement,
}

impl Box {
    /// Creates a box with a newly generated stable identity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_id(ObjectId::new_v4())
    }

    /// Creates a box with an explicit stable identity.
    ///
    /// Use this when application state must retain the same element identity
    /// across successive document snapshots.
    #[must_use]
    pub fn with_id(object_id: ObjectId) -> Self {
        Self {
            common: CommonVisualElement::new(object_id),
        }
    }

    impl_common_visual_element_methods!();

    /// Appends one logical child after the box's existing children.
    ///
    /// The concrete builder is converted into [`UiElement`] automatically, and
    /// child order is preserved in the serialized hierarchy.
    #[must_use]
    pub fn child(mut self, value: impl Into<UiElement>) -> Self {
        self.common_mut().children.push(value.into());
        self
    }

    /// Appends logical children in iterator order after existing children.
    #[must_use]
    pub fn children<I, T>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<UiElement>,
    {
        self.common_mut()
            .children
            .extend(values.into_iter().map(Into::into));
        self
    }

    fn common(&self) -> &CommonVisualElement {
        &self.common
    }

    fn common_mut(&mut self) -> &mut CommonVisualElement {
        &mut self.common
    }
}

impl Default for Box {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Box> for UiElement {
    fn from(value: Box) -> Self {
        Self::Box(value)
    }
}

/// Builds Unity's UI Toolkit `Label` for displaying non-editable text.
///
/// Labels participate in the surrounding flex layout and derive their natural
/// size from the authored text and font styling. Use [`Style::color`] and
/// [`Style::font_size`] to control the most common text presentation. A label
/// is display-only; interactive or editable text belongs to a dedicated control.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Label {
    #[serde(flatten)]
    common: CommonVisualElement,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    text: String,
}

impl Label {
    /// Creates a label containing `text` with a newly generated stable identity.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self::with_id(ObjectId::new_v4(), text)
    }

    /// Creates a label containing `text` with an explicit stable identity.
    ///
    /// Use this when application state must retain the same label identity
    /// across successive document snapshots.
    #[must_use]
    pub fn with_id(object_id: ObjectId, text: impl Into<String>) -> Self {
        Self {
            common: CommonVisualElement::new(object_id),
            text: text.into(),
        }
    }

    impl_common_visual_element_methods!();

    /// Returns the text that Unity displays for this label.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    fn common(&self) -> &CommonVisualElement {
        &self.common
    }

    fn common_mut(&mut self) -> &mut CommonVisualElement {
        &mut self.common
    }
}

impl From<Label> for UiElement {
    fn from(value: Label) -> Self {
        Self::Label(value)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct CommonVisualElement {
    #[serde(rename = "object_id", skip_serializing_if = "Option::is_none")]
    object_id_option: Option<ObjectId>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) name: String,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) classes: Vec<String>,
    #[serde(default, skip_serializing_if = "Style::is_empty")]
    pub(crate) style: Style,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) children: Vec<UiElement>,
}

impl Default for CommonVisualElement {
    fn default() -> Self {
        Self {
            object_id_option: None,
            name: String::new(),
            enabled: true,
            classes: Vec::new(),
            style: Style::default(),
            children: Vec::new(),
        }
    }
}

impl CommonVisualElement {
    fn new(object_id: ObjectId) -> Self {
        Self {
            object_id_option: Some(object_id),
            enabled: true,
            ..Self::default()
        }
    }

    pub(crate) fn required_object_id(&self) -> ObjectId {
        self.object_id_option
            .expect("element builders always have IDs")
    }
}

/// Inline style values applied directly to a UI element.
///
/// Each optional field represents an authored override of the corresponding
/// Unity UI Toolkit style property. An unset field is omitted from the wire
/// payload, allowing USS rules, inherited values, or Unity defaults to determine
/// the resolved style. Length values in this type are expressed in pixels.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Style {
    /// Color painted behind the element's content and padding area.
    ///
    /// When unset, the background remains controlled by USS or the Unity theme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<Color>,
    /// Foreground color inherited by text rendered by this element and its
    /// descendants unless a descendant overrides it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Width of the element's layout box in pixels.
    ///
    /// An authored width constrains flex layout instead of relying solely on
    /// content measurement and available space.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    /// Height of the element's layout box in pixels.
    ///
    /// An authored height constrains flex layout instead of relying solely on
    /// content measurement and available space.
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
    /// Creates style state with no authored overrides.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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

fn default_true() -> bool {
    true
}
fn is_true(value: &bool) -> bool {
    *value
}
