use battlement_types::ObjectId;
use serde::{Deserialize, Serialize};

use crate::{UiElement, UiNode};

/// Creates a complete native element subtree beneath an existing logical parent.
///
/// Unity constructs `node` and all of its descendants before attaching the new
/// root to `parent_id`. The parent may be a [`UiDocument`] root or a container
/// element. By default the subtree is appended; [`Self::child_index`] requests
/// insertion at a particular position.
///
/// The subtree must pass the same identity, hierarchy, property, and leaf-node
/// rules as a snapshot document. Every ID in it must be new to the live UI.
///
/// [`UiDocument`]: crate::UiDocument
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VisualElementCreate {
  /// Existing document root or container whose content receives the new node.
  pub parent_id: ObjectId,
  /// Zero-based insertion index in the parent's logical child list.
  ///
  /// The index may equal the current child count. Omitting it appends after
  /// all current children.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub child_index: Option<u32>,
  /// Complete identified subtree constructed before native attachment.
  pub node: UiNode,
}

impl VisualElementCreate {
  /// Creates a command value that appends `node` to `parent_id`.
  #[must_use]
  pub const fn new(parent_id: ObjectId, node: UiNode) -> Self {
    Self {
      parent_id,
      child_index: None,
      node,
    }
  }

  /// Inserts the subtree at zero-based `child_index` instead of appending it.
  #[must_use]
  pub const fn child_index(mut self, child_index: u32) -> Self {
    self.child_index = Some(child_index);
    self
  }
}

/// A property, parent, or sibling-order change for one live UI element.
///
/// These operations are deliberately independent. [`Self::Properties`] keeps
/// the current hierarchy, [`Self::Parent`] appends beneath a different logical
/// container, and [`Self::Index`] reorders within the current parent.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum VisualElementUpdate {
  /// Applies sparse visual properties without changing the element class or hierarchy.
  Properties {
    /// Element receiving the property values.
    object_id: ObjectId,
    /// Sparse values whose concrete kind must match the live element.
    ///
    /// Populated fields replace their live counterparts; omitted fields
    /// preserve the current value.
    element: std::boxed::Box<UiElement>,
  },
  /// Moves an element beneath a different logical parent and appends it there.
  Parent {
    /// Element to move.
    object_id: ObjectId,
    /// Destination container or document root in the same document.
    parent_id: ObjectId,
  },
  /// Changes an element's index within its current logical parent.
  Index {
    /// Element to reorder.
    object_id: ObjectId,
    /// Zero-based destination index after removing the element from its old position.
    child_index: u32,
  },
}

impl VisualElementUpdate {
  /// Returns the identity of the element changed by this update.
  #[must_use]
  pub const fn object_id(&self) -> ObjectId {
    match self {
      Self::Properties { object_id, .. }
      | Self::Parent { object_id, .. }
      | Self::Index { object_id, .. } => *object_id,
    }
  }
}

/// Removes one element and its complete logical subtree from the native UI.
///
/// Destruction also releases native event callbacks, pointer capture, and other
/// transient state owned by the removed subtree. Document roots are owned by
/// their host object and cannot be destroyed with this operation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VisualElementDestroy {
  /// Identity of the subtree root to remove; document roots are invalid targets.
  pub object_id: ObjectId,
}

/// Performs one transient native operation without changing authored properties.
///
/// Actions operate on the live element state and are not retained in later
/// snapshots. The target must support the selected [`VisualElementAction`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VisualElementPerformAction {
  /// Live element receiving the action.
  pub object_id: ObjectId,
  /// Native operation and its arguments.
  pub action: VisualElementAction,
}

/// One-shot operations on focus, pointer capture, scrolling, and text selection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum VisualElementAction {
  /// Requests focus for an attached element that Unity allows to receive focus.
  Focus,
  /// Removes focus from the element when it currently owns focus.
  Blur,
  /// Routes subsequent events for one pointer to this element until release.
  CapturePointer {
    /// Unity pointer identity to capture.
    pointer_id: i32,
  },
  /// Stops routing a captured pointer's events to this element.
  ReleasePointer {
    /// Unity pointer identity currently captured by this element.
    pointer_id: i32,
  },
  /// Scrolls a scroll-view target until one of its logical descendants is visible.
  ScrollTo {
    /// Identified descendant in the target scroll view's content tree.
    descendant_id: ObjectId,
  },
  /// Sets cursor and selection endpoints on selectable text or a text input.
  SelectText {
    /// Cursor endpoint as a zero-based UTF-16 code-unit index.
    cursor_index: u32,
    /// Selection endpoint as a zero-based UTF-16 code-unit index.
    selection_index: u32,
  },
}
