use battlement_types::ObjectId;
use serde::{Deserialize, Serialize};

use crate::{UiElement, UiNode};

/// Creates one detached element subtree and attaches it to a logical parent.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VisualElementCreate {
    /// Existing document root or container that receives the element.
    pub parent_id: ObjectId,
    /// Zero-based insertion index; omission appends after current children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_index: Option<u32>,
    /// Complete node subtree to construct before attachment.
    pub node: UiNode,
}

impl VisualElementCreate {
    /// Creates an append placement.
    #[must_use]
    pub const fn new(parent_id: ObjectId, node: UiNode) -> Self {
        Self {
            parent_id,
            child_index: None,
            node,
        }
    }

    /// Inserts the subtree at `child_index` instead of appending it.
    #[must_use]
    pub const fn child_index(mut self, child_index: u32) -> Self {
        self.child_index = Some(child_index);
        self
    }
}

/// One sparse property or hierarchy update for a live UI element.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum VisualElementUpdate {
    /// Applies the supplied visual properties without changing hierarchy.
    Properties {
        /// Element receiving the property values.
        object_id: ObjectId,
        /// Concrete sparse values of the same kind as the live element.
        element: std::boxed::Box<UiElement>,
    },
    /// Moves an element beneath a different logical parent and appends it.
    Parent {
        /// Element to move.
        object_id: ObjectId,
        /// Destination container or document root.
        parent_id: ObjectId,
    },
    /// Changes an element's index within its current logical parent.
    Index {
        /// Element to reorder.
        object_id: ObjectId,
        /// Zero-based destination index.
        child_index: u32,
    },
}

impl VisualElementUpdate {
    /// Returns the target element identity.
    #[must_use]
    pub const fn object_id(&self) -> ObjectId {
        match self {
            Self::Properties { object_id, .. }
            | Self::Parent { object_id, .. }
            | Self::Index { object_id, .. } => *object_id,
        }
    }
}

/// Destroys one element and all of its logical descendants.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VisualElementDestroy {
    /// Element identity to remove; document roots are not valid targets.
    pub object_id: ObjectId,
}

/// Performs one transient operation without changing authored state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VisualElementPerformAction {
    /// Element receiving the action.
    pub object_id: ObjectId,
    /// Exact operation and its arguments.
    pub action: VisualElementAction,
}

/// One-shot operations that affect native UI state without changing authored properties.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum VisualElementAction {
    /// Requests native focus.
    Focus,
    /// Removes native focus.
    Blur,
    /// Captures one pointer.
    CapturePointer {
        /// Pointer identity to capture.
        pointer_id: i32,
    },
    /// Releases one captured pointer.
    ReleasePointer {
        /// Pointer identity to release.
        pointer_id: i32,
    },
    /// Scrolls a scroll view to a logical descendant.
    ScrollTo {
        /// Logical descendant to reveal.
        descendant_id: ObjectId,
    },
    /// Sets UTF-16 cursor and selection endpoints.
    SelectText {
        /// UTF-16 cursor endpoint.
        cursor_index: u32,
        /// UTF-16 selection endpoint.
        selection_index: u32,
    },
}
