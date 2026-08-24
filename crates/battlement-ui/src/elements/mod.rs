use battlement_types::ObjectId;
use enum_dispatch::enum_dispatch;
use enum_kinds::EnumKind;
use serde::{Deserialize, Serialize};

pub use box_element::Box;
pub use button::Button;
pub use label::Label;
pub use style::{FlexDirection, Style};
pub use visual_element::VisualElement;

macro_rules! impl_common_visual_element_methods {
    () => {
        /// Sets the name of this visual element.
        #[must_use]
        pub fn name(mut self, value: impl Into<String>) -> Self {
            self.visual_element_mut().name = Some(value.into());
            self
        }

        /// Changes the visual element's enabled state.
        #[must_use]
        pub fn enabled(mut self, value: bool) -> Self {
            self.visual_element_mut().enabled = Some(value);
            self
        }

        /// Adds a class to the class list of this visual element.
        #[must_use]
        pub fn class(mut self, value: impl Into<String>) -> Self {
            self.visual_element_mut()
                .classes
                .get_or_insert_with(Vec::new)
                .push(value.into());
            self
        }

        /// Requests forwarding for each supplied UI event kind.
        #[must_use]
        pub fn events(mut self, values: impl IntoIterator<Item = crate::UiEventKind>) -> Self {
            self.visual_element_mut()
                .events
                .get_or_insert_with(Vec::new)
                .extend(values);
            self
        }

        /// Sets the style values on this visual element.
        #[must_use]
        pub fn style(mut self, value: Style) -> Self {
            self.visual_element_mut().style = value;
            self
        }
    };
}

mod box_element;
mod button;
mod label;
mod style;
mod visual_element;

/// Shared access to the visual properties composed into every UI element.
#[enum_dispatch]
pub trait VisualElementProperties {
    /// Returns the shared visual properties.
    fn visual_element(&self) -> &VisualElement;

    /// Returns the shared visual properties for mutation.
    fn visual_element_mut(&mut self) -> &mut VisualElement;
}

/// A concrete serializable UI Toolkit element value.
#[enum_dispatch(VisualElementProperties)]
#[derive(Clone, Debug, Deserialize, EnumKind, PartialEq, Serialize)]
#[enum_kind(UiElementKind, derive(Deserialize, Serialize))]
pub enum UiElement {
    /// The base class for objects in the UI Toolkit visual tree.
    VisualElement(VisualElement),
    /// A Unity UI Toolkit `Box`.
    Box(Box),
    /// A text element that displays text.
    Label(Label),
    /// A clickable button with a text label element.
    Button(Button),
}

impl UiElement {
    /// Returns the concrete protocol class.
    #[must_use]
    pub fn kind(&self) -> UiElementKind {
        self.into()
    }

    /// Applies the supplied sparse values to an element of the same concrete kind.
    pub fn apply_update(&mut self, update: &Self) {
        assert_eq!(self.kind(), update.kind(), "UI element update kind changed");
        match (self, update) {
            (Self::VisualElement(target), Self::VisualElement(value)) => {
                target.apply_update(value);
            }
            (Self::Box(target), Self::Box(value)) => target.apply_update(value),
            (Self::Label(target), Self::Label(value)) => target.apply_update(value),
            (Self::Button(target), Self::Button(value)) => target.apply_update(value),
            _ => unreachable!("validated UI element kinds diverged"),
        }
    }
}

/// One identified node in a UI document hierarchy.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UiNode {
    /// Stable identity used by commands and events.
    pub object_id: ObjectId,
    /// Concrete visual state for this node.
    pub element: UiElement,
    /// Logical children in authored order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<UiNode>,
}

impl UiNode {
    /// Creates a leaf node with a stable identity.
    #[must_use]
    pub fn new(object_id: ObjectId, element: impl Into<UiElement>) -> Self {
        Self {
            object_id,
            element: element.into(),
            children: Vec::new(),
        }
    }

    /// Appends one logical child.
    #[must_use]
    pub fn child(mut self, value: UiNode) -> Self {
        self.children.push(value);
        self
    }

    /// Appends logical children in iterator order.
    #[must_use]
    pub fn children(mut self, values: impl IntoIterator<Item = UiNode>) -> Self {
        self.children.extend(values);
        self
    }

    /// Appends a logical child when present.
    #[must_use]
    pub fn optional_child(mut self, value: Option<UiNode>) -> Self {
        if let Some(value) = value {
            self.children.push(value);
        }
        self
    }

    /// Appends logical children when `condition` is true.
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
