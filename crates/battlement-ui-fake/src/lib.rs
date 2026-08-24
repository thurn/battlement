//! In-memory execution of Battlement UI documents and commands.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{HashMap, HashSet};

use battlement_types::ObjectId;
use battlement_ui::{
    Style, UiDocument, UiElement, UiElementKind, UiEventKind, UiNode, VisualElementAction,
    VisualElementCreate, VisualElementProperties, VisualElementUpdate,
};

/// A rejected fake UI operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiWorldError {
    /// A referenced identity does not exist.
    UnknownObject,
    /// An identity already belongs to another live UI object.
    DuplicateObject,
    /// A logical parent, index, root rule, or cycle is invalid.
    InvalidHierarchy,
    /// A property or subscription value is invalid.
    InvalidProperty,
    /// This in-memory executor does not support the requested action.
    UnsupportedAction,
}

/// One logical element state stored by [`UiWorld`].
#[derive(Clone, Debug, PartialEq)]
pub struct UiElementState {
    object_id: ObjectId,
    element: UiElement,
    parent_id: Option<ObjectId>,
    children: Vec<ObjectId>,
    is_document_root: bool,
}

impl UiElementState {
    /// Returns the stable element identity.
    #[must_use]
    pub fn object_id(&self) -> ObjectId {
        self.object_id
    }

    /// Returns the concrete UI class.
    #[must_use]
    pub fn kind(&self) -> UiElementKind {
        self.element.kind()
    }

    /// Returns the canonical element value carrying the current resolved properties.
    #[must_use]
    pub const fn element(&self) -> &UiElement {
        &self.element
    }

    /// Returns the logical parent, or `None` for a document root.
    #[must_use]
    pub const fn parent_id(&self) -> Option<ObjectId> {
        self.parent_id
    }

    /// Returns logical child identities in display order.
    #[must_use]
    pub fn children(&self) -> &[ObjectId] {
        &self.children
    }

    /// Returns the current Unity element name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.element.visual_element().name.as_deref()
    }

    /// Returns whether the element is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> Option<bool> {
        self.element.visual_element().enabled
    }

    /// Returns authored USS classes in order.
    #[must_use]
    pub fn classes(&self) -> Option<&[String]> {
        self.element.visual_element().classes.as_deref()
    }

    /// Returns current authored inline style values.
    #[must_use]
    pub fn style(&self) -> &Style {
        &self.element.visual_element().style
    }

    /// Returns event subscriptions in authored order.
    #[must_use]
    pub fn events(&self) -> Option<&[UiEventKind]> {
        self.element.visual_element().events.as_deref()
    }

    /// Returns current display text for labels and buttons.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match &self.element {
            UiElement::Label(value) => value.text.as_deref(),
            UiElement::Button(value) => value.text.as_deref(),
            _ => None,
        }
    }
}

/// One UI command recorded after successful fake execution.
#[derive(Clone, Debug, PartialEq)]
pub enum UiJournalEntry {
    /// A subtree was created under a logical parent.
    Create(std::boxed::Box<VisualElementCreate>),
    /// One sparse element update was committed.
    Update(VisualElementUpdate),
    /// One element subtree was destroyed.
    Destroy(ObjectId),
}

/// Authoritative in-memory UI hierarchy used by `battlement-fake`.
#[derive(Clone, Debug, Default)]
pub struct UiWorld {
    elements: HashMap<ObjectId, UiElementState>,
    document_ids: HashSet<ObjectId>,
    journal: Vec<UiJournalEntry>,
}

impl UiWorld {
    /// Replaces every document and element from authoritative snapshot state.
    pub fn replace(&mut self, documents: Vec<UiDocument>) -> Result<(), UiWorldError> {
        battlement_ui::validate_documents(&documents).map_err(|_| UiWorldError::InvalidProperty)?;
        let mut next = Self::default();
        for document in documents {
            if !next.document_ids.insert(document.document_id) {
                return Err(UiWorldError::DuplicateObject);
            }
            next.insert_subtree(None, document.into_root_node(), true)?;
        }
        *self = next;
        Ok(())
    }

    /// Returns one live element state.
    #[must_use]
    pub fn element(&self, object_id: ObjectId) -> Option<&UiElementState> {
        self.elements.get(&object_id)
    }

    /// Returns successful UI command history.
    #[must_use]
    pub fn journal(&self) -> &[UiJournalEntry] {
        &self.journal
    }

    /// Returns whether the target requested an event.
    #[must_use]
    pub fn has_subscription(&self, object_id: ObjectId, event: UiEventKind) -> bool {
        self.elements.get(&object_id).is_some_and(|element| {
            element
                .element
                .visual_element()
                .events
                .as_ref()
                .is_some_and(|events| events.contains(&event))
        })
    }

    /// Creates and attaches one detached subtree.
    pub fn create(&mut self, command: VisualElementCreate) -> Result<(), UiWorldError> {
        let parent = self
            .elements
            .get(&command.parent_id)
            .ok_or(UiWorldError::UnknownObject)?;
        require_container(parent.element.kind())?;
        let index = command
            .child_index
            .map_or(parent.children.len(), |value| value as usize);
        if index > parent.children.len() {
            return Err(UiWorldError::InvalidHierarchy);
        }
        let mut identities = HashSet::new();
        collect_ids(&command.node, &mut identities)?;
        if identities.iter().any(|id| self.elements.contains_key(id)) {
            return Err(UiWorldError::DuplicateObject);
        }
        self.insert_subtree(Some(command.parent_id), command.node.clone(), false)?;
        self.elements
            .get_mut(&command.parent_id)
            .expect("validated parent disappeared")
            .children
            .insert(index, command.node.object_id);
        self.journal
            .push(UiJournalEntry::Create(std::boxed::Box::new(command)));
        Ok(())
    }

    /// Applies one sparse property or hierarchy update.
    pub fn update(&mut self, update: VisualElementUpdate) -> Result<(), UiWorldError> {
        let object_id = update.object_id();
        if !self.elements.contains_key(&object_id) {
            return Err(UiWorldError::UnknownObject);
        }
        match &update {
            VisualElementUpdate::Properties { element, .. } => {
                self.elements
                    .get_mut(&object_id)
                    .expect("validated element disappeared")
                    .element
                    .apply_update(element);
            }
            VisualElementUpdate::Parent { parent_id, .. } => {
                self.place(object_id, *parent_id, None)?;
            }
            VisualElementUpdate::Index { child_index, .. } => {
                let Some(parent_id) = self.elements[&object_id].parent_id else {
                    return Err(UiWorldError::InvalidHierarchy);
                };
                self.place(object_id, parent_id, Some(*child_index))?;
            }
        }
        self.journal.push(UiJournalEntry::Update(update));
        Ok(())
    }

    /// Destroys one non-root element and all logical descendants.
    pub fn destroy(&mut self, object_id: ObjectId) -> Result<(), UiWorldError> {
        let target = self
            .elements
            .get(&object_id)
            .ok_or(UiWorldError::UnknownObject)?;
        if target.is_document_root {
            return Err(UiWorldError::InvalidHierarchy);
        }
        let parent_id = target.parent_id.expect("non-root element has no parent");
        self.elements
            .get_mut(&parent_id)
            .expect("parent disappeared")
            .children
            .retain(|value| *value != object_id);
        self.remove_subtree(object_id);
        self.journal.push(UiJournalEntry::Destroy(object_id));
        Ok(())
    }

    /// Returns unsupported because this executor has no native UI focus state.
    pub const fn perform_action(
        &mut self,
        _object_id: ObjectId,
        _action: &VisualElementAction,
    ) -> Result<(), UiWorldError> {
        Err(UiWorldError::UnsupportedAction)
    }

    fn insert_subtree(
        &mut self,
        parent_id: Option<ObjectId>,
        node: UiNode,
        is_document_root: bool,
    ) -> Result<(), UiWorldError> {
        if self.elements.contains_key(&node.object_id) {
            return Err(UiWorldError::DuplicateObject);
        }
        let object_id = node.object_id;
        let child_ids = node.children.iter().map(|child| child.object_id).collect();
        let state = UiElementState {
            object_id,
            element: node.element,
            parent_id,
            children: child_ids,
            is_document_root,
        };
        self.elements.insert(object_id, state);
        for child in node.children {
            self.insert_subtree(Some(object_id), child, false)?;
        }
        Ok(())
    }

    fn place(
        &mut self,
        object_id: ObjectId,
        parent_id: ObjectId,
        child_index: Option<u32>,
    ) -> Result<(), UiWorldError> {
        if self.elements[&object_id].is_document_root || object_id == parent_id {
            return Err(UiWorldError::InvalidHierarchy);
        }
        let parent = self
            .elements
            .get(&parent_id)
            .ok_or(UiWorldError::UnknownObject)?;
        require_container(parent.element.kind())?;
        let destination_len = parent.children.len()
            - usize::from(self.elements[&object_id].parent_id == Some(parent_id));
        let index = child_index.map_or(destination_len, |value| value as usize);
        if index > destination_len || self.is_descendant(parent_id, object_id) {
            return Err(UiWorldError::InvalidHierarchy);
        }
        let old_parent = self.elements[&object_id]
            .parent_id
            .expect("non-root has no parent");
        self.elements
            .get_mut(&old_parent)
            .expect("old parent disappeared")
            .children
            .retain(|value| *value != object_id);
        self.elements
            .get_mut(&parent_id)
            .expect("new parent disappeared")
            .children
            .insert(index, object_id);
        self.elements
            .get_mut(&object_id)
            .expect("element disappeared")
            .parent_id = Some(parent_id);
        Ok(())
    }

    fn is_descendant(&self, candidate: ObjectId, ancestor: ObjectId) -> bool {
        let mut cursor = Some(candidate);
        while let Some(value) = cursor {
            if value == ancestor {
                return true;
            }
            cursor = self
                .elements
                .get(&value)
                .and_then(|element| element.parent_id);
        }
        false
    }

    fn remove_subtree(&mut self, object_id: ObjectId) {
        let children = self.elements[&object_id].children.clone();
        for child in children {
            self.remove_subtree(child);
        }
        self.elements.remove(&object_id);
    }
}

fn require_container(kind: UiElementKind) -> Result<(), UiWorldError> {
    if matches!(kind, UiElementKind::Label | UiElementKind::Button) {
        Err(UiWorldError::InvalidHierarchy)
    } else {
        Ok(())
    }
}

fn collect_ids(node: &UiNode, identities: &mut HashSet<ObjectId>) -> Result<(), UiWorldError> {
    if !identities.insert(node.object_id) {
        return Err(UiWorldError::DuplicateObject);
    }
    require_container(node.element.kind()).or_else(|error| {
        if node.children.is_empty() {
            Ok(())
        } else {
            Err(error)
        }
    })?;
    for child in &node.children {
        collect_ids(child, identities)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use battlement_types::ObjectId;
    use battlement_ui::{
        Button, Label, UiDocument, UiEventKind, UiNode, VisualElement, VisualElementUpdate,
    };

    use crate::UiWorld;

    #[test]
    fn subscriptions_match_only_the_logical_target_until_routing_is_supported() {
        let document_id = ObjectId::new_v4();
        let root_id = ObjectId::new_v4();
        let container_id = ObjectId::new_v4();
        let button_id = ObjectId::new_v4();
        let document = UiDocument::with_root_id(document_id, root_id).child(
            UiNode::new(
                container_id,
                VisualElement::new().events([UiEventKind::Click]),
            )
            .child(UiNode::new(button_id, Button::new("Button"))),
        );
        let mut world = UiWorld::default();

        world.replace(vec![document]).unwrap();

        assert!(world.has_subscription(container_id, UiEventKind::Click));
        assert!(!world.has_subscription(button_id, UiEventKind::Click));
    }

    #[test]
    fn updates_the_canonical_element_value() {
        let document_id = ObjectId::new_v4();
        let root_id = ObjectId::new_v4();
        let label_id = ObjectId::new_v4();
        let mut world = UiWorld::default();
        world
            .replace(vec![
                UiDocument::with_root_id(document_id, root_id)
                    .child(UiNode::new(label_id, Label::new("Before"))),
            ])
            .unwrap();

        world
            .update(VisualElementUpdate::Properties {
                object_id: label_id,
                element: std::boxed::Box::new(Label::new("After").into()),
            })
            .unwrap();

        let state = world.element(label_id).unwrap();
        assert_eq!(state.text(), Some("After"));
    }

    #[test]
    fn rejects_reordering_a_document_root() {
        let document_id = ObjectId::new_v4();
        let root_id = ObjectId::new_v4();
        let mut world = UiWorld::default();
        world
            .replace(vec![UiDocument::with_root_id(document_id, root_id)])
            .unwrap();

        assert_eq!(
            world.update(VisualElementUpdate::Index {
                object_id: root_id,
                child_index: 0,
            }),
            Err(crate::UiWorldError::InvalidHierarchy)
        );
    }
}
