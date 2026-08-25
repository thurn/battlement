use battlement_types::ObjectId;
use enum_dispatch::enum_dispatch;
use enum_kinds::EnumKind;
use serde::{Deserialize, Serialize};

pub use box_element::Box;
pub use button::Button;
pub use image::{Image, ImageScaleMode, ImageSource};
pub use label::Label;
pub use style::{
    Align, AspectRatio, FlexDirection, FlexWrap, FloatValue, InlineKeyword, IntoStyleSides,
    Justify, Length, LengthOrAuto, LengthUnits, Position, Style, StyleValue,
};
pub use visual_element::{LanguageDirection, PickingMode, UsageHint, VisualElement};

macro_rules! impl_common_visual_element_methods {
    () => {
        /// Sets the name used by Unity queries and `#name` USS selectors.
        ///
        /// The name is independent of the object ID stored by the enclosing
        /// [`UiNode`](crate::UiNode); commands and events address that ID instead.
        #[must_use]
        pub fn name(mut self, value: impl Into<String>) -> Self {
            self.visual_element_mut().name = Some(value.into());
            self
        }

        /// Sets whether this element is locally enabled for interaction.
        ///
        /// An enabled element remains disabled in the hierarchy when any
        /// ancestor is disabled. Unity applies its disabled USS state and does
        /// not deliver ordinary input events to disabled elements.
        #[must_use]
        pub fn enabled(mut self, value: bool) -> Self {
            self.visual_element_mut().enabled = Some(value);
            self
        }

        /// Sets whether pointer hit testing may select this element.
        ///
        /// Ignoring this element also prevents its hover pseudo-state, but does
        /// not prevent independently pickable descendants from being selected.
        #[must_use]
        pub fn picking_mode(mut self, value: PickingMode) -> Self {
            self.visual_element_mut().picking_mode = Some(value);
            self
        }

        /// Sets text directionality for this element's inheriting subtree.
        ///
        /// This changes text direction rather than flex layout direction.
        #[must_use]
        pub fn language_direction(mut self, value: LanguageDirection) -> Self {
            self.visual_element_mut().language_direction = Some(value);
            self
        }

        /// Sets whether this element may receive focus.
        ///
        /// The element must also be attached, enabled in its hierarchy, and
        /// accepted by Unity's focus controller to acquire focus.
        #[must_use]
        pub fn focusable(mut self, value: bool) -> Self {
            self.visual_element_mut().focusable = Some(value);
            self
        }

        /// Sets this element's position in Unity's keyboard focus ring.
        ///
        /// Negative values exclude the element from tab navigation without
        /// disabling programmatic focus eligibility.
        #[must_use]
        pub fn tab_index(mut self, value: i32) -> Self {
            self.visual_element_mut().tab_index = Some(value);
            self
        }

        /// Sets whether focus requested here transfers to an eligible descendant.
        ///
        /// Unity selects the delegated target from focus-ring order; a specific
        /// descendant cannot be named.
        #[must_use]
        pub fn delegates_focus(mut self, value: bool) -> Self {
            self.visual_element_mut().delegates_focus = Some(value);
            self
        }

        /// Appends one USS class name used by `.class-name` selectors.
        ///
        /// Calls preserve insertion order. Empty or duplicate class names make
        /// the containing document invalid.
        #[must_use]
        pub fn class(mut self, value: impl Into<String>) -> Self {
            self.visual_element_mut()
                .classes
                .get_or_insert_with(Vec::new)
                .push(value.into());
            self
        }

        /// Adds create-time rendering optimization hints for this element.
        ///
        /// Hints do not change observable behavior. Repeating a hint makes the
        /// containing document or create command invalid, and usage hints are
        /// rejected in sparse property updates because Unity makes them
        /// read-only after panel attachment.
        #[must_use]
        pub fn usage_hints(mut self, values: impl IntoIterator<Item = UsageHint>) -> Self {
            self.visual_element_mut()
                .usage_hints
                .get_or_insert_with(Vec::new)
                .extend(values);
            self
        }

        /// Subscribes the Rust rules engine to the supplied native event kinds.
        ///
        /// Calling this method repeatedly appends subscriptions. Duplicate
        /// kinds make the containing document invalid.
        #[must_use]
        pub fn events(mut self, values: impl IntoIterator<Item = crate::UiEventKind>) -> Self {
            self.visual_element_mut()
                .events
                .get_or_insert_with(Vec::new)
                .extend(values);
            self
        }

        /// Replaces this value's collection of inline style declarations.
        ///
        /// Inline declarations take precedence over matching USS rules. When
        /// this element is used as an update, only populated [`Style`] fields
        /// alter the live element.
        #[must_use]
        pub fn style(mut self, value: Style) -> Self {
            self.visual_element_mut().style = value;
            self
        }
    };
}

mod box_element;
mod button;
mod image;
mod label;
mod style;
mod visual_element;

/// Accesses the [`VisualElement`] properties composed into every concrete element.
///
/// Generic code can use this trait to inspect or edit names, classes, inline
/// styles, enabled state, and event subscriptions without matching on
/// [`UiElement`]. It does not expose element-specific properties such as label
/// or button text.
#[enum_dispatch]
pub trait VisualElementProperties {
    /// Returns this element's shared visual properties.
    fn visual_element(&self) -> &VisualElement;

    /// Returns this element's shared visual properties for mutation.
    fn visual_element_mut(&mut self) -> &mut VisualElement;
}

/// The supported native UI Toolkit element classes.
///
/// Each variant serializes its concrete class name and properties. The Unity
/// host uses the variant to create the corresponding native element, and
/// [`Self::kind`] provides the same discriminator without borrowing the inner
/// value. Convert an element builder into `UiElement` implicitly by passing it
/// to [`UiNode::new`].
#[enum_dispatch(VisualElementProperties)]
#[derive(Clone, Debug, Deserialize, EnumKind, PartialEq, Serialize)]
#[enum_kind(UiElementKind, derive(Deserialize, Serialize))]
pub enum UiElement {
    /// A neutral container for grouping and styling child elements.
    VisualElement(VisualElement),
    /// A container with Unity's themed box background and border.
    Box(Box),
    /// A leaf text element for titles, captions, and descriptions.
    Label(Label),
    /// A leaf control that can forward pointer or navigation activation.
    Button(Button),
    /// A leaf graphic displaying one prepared texture, sprite, vector image, or render texture.
    Image(Image),
}

impl UiElement {
    /// Returns the concrete element class used for native creation and updates.
    #[must_use]
    pub fn kind(&self) -> UiElementKind {
        self.into()
    }

    /// Applies populated properties from `update` to this element.
    ///
    /// Shared visual properties and element-specific properties are sparse:
    /// populated values replace their counterparts and omitted values preserve
    /// the current state.
    ///
    /// # Panics
    ///
    /// Panics when `update` has a different [`UiElementKind`]. Changing the
    /// native class of an existing object is a caller invariant violation; use
    /// a destroy followed by a create operation instead.
    pub fn apply_update(&mut self, update: &Self) {
        assert_eq!(self.kind(), update.kind(), "UI element update kind changed");
        match (self, update) {
            (Self::VisualElement(target), Self::VisualElement(value)) => {
                target.apply_update(value);
            }
            (Self::Box(target), Self::Box(value)) => target.apply_update(value),
            (Self::Label(target), Self::Label(value)) => target.apply_update(value),
            (Self::Button(target), Self::Button(value)) => target.apply_update(value),
            (Self::Image(target), Self::Image(value)) => target.apply_update(value),
            _ => unreachable!("validated UI element kinds diverged"),
        }
    }
}

/// One identified element and its logical children in a UI document tree.
///
/// `object_id` is the stable address used by commands and events. Children are
/// stored in visual order and are added to the native element's logical content
/// container. [`VisualElement`] and [`Box`] are containers; [`Label`] and
/// [`Button`] and [`Image`] are leaves and make a document invalid when given children.
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{Box, Label, UiNode};
///
/// let card = UiNode::new(ObjectId::new_v4(), Box::new())
///     .child(UiNode::new(ObjectId::new_v4(), Label::new("Summary")))
///     .children([
///         UiNode::new(ObjectId::new_v4(), Label::new("Ready")),
///         UiNode::new(ObjectId::new_v4(), Label::new("Waiting")),
///     ]);
///
/// assert_eq!(card.children.len(), 3);
/// ```
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UiNode {
    /// Stable identity used to address this element in commands and events.
    ///
    /// IDs share one namespace with document hosts and roots and must be unique
    /// across a validated document collection.
    pub object_id: ObjectId,
    /// Concrete native element class and its authored properties.
    pub element: UiElement,
    /// Logical children in native insertion and layout order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<UiNode>,
}

impl UiNode {
    /// Creates an identified node with no logical children.
    #[must_use]
    pub fn new(object_id: ObjectId, element: impl Into<UiElement>) -> Self {
        Self {
            object_id,
            element: element.into(),
            children: Vec::new(),
        }
    }

    /// Appends one child after the node's existing logical children.
    #[must_use]
    pub fn child(mut self, value: UiNode) -> Self {
        self.children.push(value);
        self
    }

    /// Appends children in iterator order after existing logical children.
    #[must_use]
    pub fn children(mut self, values: impl IntoIterator<Item = UiNode>) -> Self {
        self.children.extend(values);
        self
    }

    /// Appends `value` when present and otherwise leaves the hierarchy unchanged.
    #[must_use]
    pub fn optional_child(mut self, value: Option<UiNode>) -> Self {
        if let Some(value) = value {
            self.children.push(value);
        }
        self
    }

    /// Appends `values` in iterator order only when `condition` is true.
    #[must_use]
    pub fn children_if(
        mut self,
        condition: bool,
        values: impl IntoIterator<Item = UiNode>,
    ) -> Self {
        if condition {
            self.children.extend(values);
        }
        self
    }
}
