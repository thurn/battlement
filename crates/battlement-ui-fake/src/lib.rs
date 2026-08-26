//! In-memory execution of Battlement UI documents and commands.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod choice_groups;
mod hierarchy;

use std::collections::{HashMap, HashSet};

use battlement_types::{MaterialAddress, ObjectId, TextureAddress};
use battlement_ui::{
    BackgroundSource, Choice, Cursor, IconSource, ImageSource, LanguageDirection, PickingMode,
    Style, StyleValue, UiDocument, UiElement, UiElementKind, UiEventKind, UiNode, UsageHint,
    VisualElementAction, VisualElementCreate, VisualElementProperties, VisualElementUpdate,
    authored_private_part_styles,
};

const MAXIMUM_HIERARCHY_DEPTH: usize = 256;
const MAXIMUM_IDENTITIES: usize = 100_000;

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
    document_root_id: ObjectId,
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

    /// Returns the document root that owns this logical element.
    #[must_use]
    pub const fn document_root_id(&self) -> ObjectId {
        self.document_root_id
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

    /// Returns current pointer hit-testing behavior.
    #[must_use]
    pub fn picking_mode(&self) -> Option<PickingMode> {
        self.element.visual_element().picking_mode
    }

    /// Returns current inheritable text directionality.
    #[must_use]
    pub fn language_direction(&self) -> Option<LanguageDirection> {
        self.element.visual_element().language_direction
    }

    /// Returns whether the element is eligible to receive focus.
    #[must_use]
    pub fn is_focusable(&self) -> Option<bool> {
        self.element.visual_element().focusable
    }

    /// Returns current keyboard focus-ring ordering.
    #[must_use]
    pub fn tab_index(&self) -> Option<i32> {
        self.element.visual_element().tab_index
    }

    /// Returns whether focus requested here transfers to a descendant.
    #[must_use]
    pub fn delegates_focus(&self) -> Option<bool> {
        self.element.visual_element().delegates_focus
    }

    /// Returns authored create-time rendering optimization hints.
    #[must_use]
    pub fn usage_hints(&self) -> Option<&[UsageHint]> {
        self.element.visual_element().usage_hints.as_deref()
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
            UiElement::TextElement(value) => value.text.as_deref(),
            UiElement::TextField(value) => value.value.as_deref(),
            UiElement::Toggle(value) => value.text.as_deref(),
            UiElement::RadioButton(value) => value.text.as_deref(),
            UiElement::Button(value) => value.text.as_deref(),
            UiElement::RepeatButton(value) => value.text.as_deref(),
            UiElement::GroupBox(value) => value.text.as_deref(),
            UiElement::PopupWindow(value) => value.text.as_deref(),
            UiElement::Tab(value) => value.text.as_deref(),
            _ => None,
        }
    }

    /// Returns the authored value for a controlled Boolean control.
    #[must_use]
    pub fn bool_value(&self) -> Option<bool> {
        match &self.element {
            UiElement::Toggle(value) => value.value,
            UiElement::RadioButton(value) => value.value,
            _ => None,
        }
    }

    /// Returns the authored index for a controlled radio group.
    #[must_use]
    pub fn selected_index(&self) -> Option<u32> {
        match &self.element {
            UiElement::RadioButtonGroup(value) => value.selected_index,
            _ => None,
        }
    }

    /// Returns the authored indices for a controlled toggle-button group.
    #[must_use]
    pub fn selected_indices(&self) -> Option<&[u32]> {
        match &self.element {
            UiElement::ToggleButtonGroup(value) => value.selected_indices.as_deref(),
            _ => None,
        }
    }

    /// Returns the authored dropdown selection when this is a dropdown field.
    #[must_use]
    pub fn choice(&self) -> Option<&Choice> {
        match &self.element {
            UiElement::DropdownField(value) => value.selection.as_ref(),
            _ => None,
        }
    }

    /// Returns the prepared graphical source displayed by an image element.
    #[must_use]
    pub fn image_source(&self) -> Option<&ImageSource> {
        match &self.element {
            UiElement::Image(value) => value.source.as_ref(),
            _ => None,
        }
    }

    /// Returns the prepared graphical source displayed by a button or tab icon.
    #[must_use]
    pub fn icon_source(&self) -> Option<&IconSource> {
        match &self.element {
            UiElement::Button(value) => value.icon.as_ref(),
            UiElement::Tab(value) => value.icon.as_ref(),
            _ => None,
        }
    }

    /// Returns the prepared graphical source painted by the inline background style.
    #[must_use]
    pub fn background_source(&self) -> Option<&BackgroundSource> {
        match &self.element.visual_element().style.background_image {
            Some(StyleValue::Value(value)) => Some(value),
            Some(StyleValue::Keyword { .. }) | None => None,
        }
    }

    /// Returns the prepared material retained by the element's inline style.
    #[must_use]
    pub fn material_source(&self) -> Option<&MaterialAddress> {
        match &self.element.visual_element().style.unity_material {
            Some(StyleValue::Value(value)) => Some(value),
            Some(StyleValue::Keyword { .. }) | None => None,
        }
    }

    /// Returns the prepared texture retained by the inline cursor style.
    #[must_use]
    pub fn cursor_source(&self) -> Option<&TextureAddress> {
        match &self.element.visual_element().style.cursor {
            Some(StyleValue::Value(Cursor::Texture { address, .. })) => Some(address),
            Some(StyleValue::Value(Cursor::Default)) | Some(StyleValue::Keyword { .. }) | None => {
                None
            }
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
    asset_usage: HashMap<ImageSource, usize>,
    icon_usage: HashMap<IconSource, usize>,
    background_usage: HashMap<BackgroundSource, usize>,
    cursor_usage: HashMap<TextureAddress, usize>,
    material_usage: HashMap<MaterialAddress, usize>,
}

impl UiWorld {
    /// Replaces every document and element from authoritative snapshot state.
    pub fn replace(&mut self, documents: Vec<UiDocument>) -> Result<(), UiWorldError> {
        battlement_ui::validate_documents(&documents).map_err(map_validation_error)?;
        let mut next = Self::default();
        for document in documents {
            if !next.document_ids.insert(document.document_id) {
                return Err(UiWorldError::DuplicateObject);
            }
            let root_id = document.root_id;
            next.insert_subtree(None, document.into_root_node(), true, root_id)?;
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

    /// Returns the number of live image properties retaining one prepared source.
    #[must_use]
    pub fn asset_usage_count(&self, source: &ImageSource) -> usize {
        self.asset_usage.get(source).copied().unwrap_or(0)
    }

    /// Iterates over prepared image sources and their positive live usage counts.
    pub fn asset_usage(&self) -> impl Iterator<Item = (&ImageSource, &usize)> {
        self.asset_usage.iter()
    }

    /// Returns the number of live button properties retaining one prepared icon.
    #[must_use]
    pub fn icon_usage_count(&self, source: &IconSource) -> usize {
        self.icon_usage.get(source).copied().unwrap_or(0)
    }

    /// Returns the number of live inline styles retaining a prepared background source.
    #[must_use]
    pub fn background_usage_count(&self, source: &BackgroundSource) -> usize {
        self.background_usage.get(source).copied().unwrap_or(0)
    }

    /// Returns the number of live inline cursors retaining a prepared texture.
    #[must_use]
    pub fn cursor_usage_count(&self, source: &TextureAddress) -> usize {
        self.cursor_usage.get(source).copied().unwrap_or(0)
    }

    /// Returns the number of live inline styles retaining a prepared material.
    #[must_use]
    pub fn material_usage_count(&self, source: &MaterialAddress) -> usize {
        self.material_usage.get(source).copied().unwrap_or(0)
    }

    /// Returns whether the target requested an event.
    #[must_use]
    pub fn has_subscription(&self, object_id: ObjectId, event: UiEventKind) -> bool {
        self.has_phase_subscription(object_id, event, battlement_ui::UiEventPhase::Target)
    }

    /// Returns whether the target requested one event at an explicit route phase.
    #[must_use]
    pub fn has_phase_subscription(
        &self,
        object_id: ObjectId,
        event: UiEventKind,
        phase: battlement_ui::UiEventPhase,
    ) -> bool {
        self.elements.get(&object_id).is_some_and(|element| {
            let visual = element.element.visual_element();
            let shorthand = phase == battlement_ui::UiEventPhase::Target
                && visual
                    .events
                    .as_ref()
                    .is_some_and(|events| events.contains(&event));
            shorthand
                || visual.event_subscriptions.as_ref().is_some_and(|events| {
                    events.contains(&battlement_ui::UiEventSubscription::new(event, phase))
                })
        })
    }

    /// Routes one native event through the current fake hierarchy.
    #[must_use]
    pub fn route_event(
        &self,
        event: &battlement_ui::UiEvent,
    ) -> Vec<battlement_ui::routing::UiEventDelivery> {
        let mut ids = Vec::new();
        let mut current = Some(event.target_id);
        while let Some(object_id) = current {
            let Some(element) = self.elements.get(&object_id) else {
                return Vec::new();
            };
            ids.push(object_id);
            current = element.parent_id;
        }
        let route = ids
            .iter()
            .map(|object_id| {
                let visual = self.elements[object_id].element.visual_element();
                (
                    *object_id,
                    visual
                        .events
                        .iter()
                        .flatten()
                        .copied()
                        .map(battlement_ui::UiEventSubscription::target)
                        .chain(visual.event_subscriptions.iter().flatten().copied())
                        .collect(),
                )
            })
            .collect::<Vec<_>>();
        battlement_ui::routing::route_subscriptions(&route, event)
    }

    /// Returns the nearest target-or-ancestor subscription on a logical route.
    #[must_use]
    pub fn first_subscription(&self, object_id: ObjectId, event: UiEventKind) -> Option<ObjectId> {
        let mut current = Some(object_id);
        while let Some(value) = current {
            if self.has_subscription(value, event) {
                return Some(value);
            }
            current = self
                .elements
                .get(&value)
                .and_then(|element| element.parent_id);
        }
        None
    }

    /// Creates and attaches one detached subtree.
    pub fn create(&mut self, command: VisualElementCreate) -> Result<(), UiWorldError> {
        let parent = self
            .elements
            .get(&command.parent_id)
            .ok_or(UiWorldError::UnknownObject)?;
        hierarchy::require_container(parent.element.kind())?;
        hierarchy::require_placement(command.node.element.kind(), parent.element.kind())?;
        if parent.kind() == UiElementKind::ToggleButtonGroup && parent.children.len() >= 64 {
            return Err(UiWorldError::InvalidHierarchy);
        }
        let index = command
            .child_index
            .map_or(parent.children.len(), |value| value as usize);
        if index > parent.children.len() {
            return Err(UiWorldError::InvalidHierarchy);
        }
        let mut identities = HashSet::new();
        battlement_ui::validate_create_subtree(&command.node).map_err(map_validation_error)?;
        hierarchy::collect_ids(&command.node, &mut identities)?;
        if identities.iter().any(|id| self.elements.contains_key(id)) {
            return Err(UiWorldError::DuplicateObject);
        }
        if self.elements.len() + identities.len() > MAXIMUM_IDENTITIES {
            return Err(UiWorldError::InvalidHierarchy);
        }
        let subtree_depth = hierarchy::subtree_depth(&command.node);
        if self.depth(command.parent_id) + subtree_depth + 1 > MAXIMUM_HIERARCHY_DEPTH {
            return Err(UiWorldError::InvalidHierarchy);
        }
        let document_root_id = parent.document_root_id;
        self.insert_subtree(
            Some(command.parent_id),
            command.node.clone(),
            false,
            document_root_id,
        )?;
        self.elements
            .get_mut(&command.parent_id)
            .expect("validated parent disappeared")
            .children
            .insert(index, command.node.object_id);
        self.clamp_tab_selection(command.parent_id);
        let child_count = self.elements[&command.parent_id].children.len();
        choice_groups::insert(
            &mut self
                .elements
                .get_mut(&command.parent_id)
                .expect("parent disappeared")
                .element,
            index,
            child_count,
        );
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
                battlement_ui::validate_element_update(element).map_err(map_validation_error)?;
                let mut next = self.elements[&object_id].element.clone();
                next.apply_update(element);
                battlement_ui::validate_element_state(&next).map_err(map_validation_error)?;
                if let UiElement::TabView(value) = &next
                    && value.selected_tab_index.is_some_and(|index| {
                        index as usize >= self.elements[&object_id].children.len()
                    })
                {
                    return Err(UiWorldError::InvalidProperty);
                }
                choice_groups::validate_state(&next, self.elements[&object_id].children.len())?;
                let previous = self.elements[&object_id].image_source().cloned();
                let previous_icon = self.elements[&object_id].icon_source().cloned();
                let previous_background = self.elements[&object_id].background_source().cloned();
                let previous_cursor = self.elements[&object_id].cursor_source().cloned();
                let previous_material = self.elements[&object_id].material_source().cloned();
                let previous_part_assets = part_assets(self.elements[&object_id].element());
                self.elements
                    .get_mut(&object_id)
                    .expect("validated element disappeared")
                    .element
                    .apply_update(element);
                let current = self.elements[&object_id].image_source().cloned();
                let current_icon = self.elements[&object_id].icon_source().cloned();
                let current_background = self.elements[&object_id].background_source().cloned();
                let current_cursor = self.elements[&object_id].cursor_source().cloned();
                let current_material = self.elements[&object_id].material_source().cloned();
                let current_part_assets = part_assets(self.elements[&object_id].element());
                if previous != current {
                    if let Some(source) = previous {
                        self.release_source(&source);
                    }
                    if let Some(source) = current {
                        self.retain_source(source);
                    }
                }
                if previous_icon != current_icon {
                    if let Some(source) = previous_icon {
                        self.release_icon(&source);
                    }
                    if let Some(source) = current_icon {
                        self.retain_icon(source);
                    }
                }
                if previous_material != current_material {
                    if let Some(source) = previous_material {
                        self.release_material(&source);
                    }
                    if let Some(source) = current_material {
                        self.retain_material(source);
                    }
                }
                if previous_background != current_background {
                    if let Some(source) = previous_background {
                        self.release_background(&source);
                    }
                    if let Some(source) = current_background {
                        self.retain_background(source);
                    }
                }
                if previous_cursor != current_cursor {
                    if let Some(source) = previous_cursor {
                        self.release_cursor(&source);
                    }
                    if let Some(source) = current_cursor {
                        self.retain_cursor(source);
                    }
                }
                self.release_part_assets(previous_part_assets);
                self.retain_part_assets(current_part_assets);
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
        let removed_index = self.elements[&parent_id]
            .children
            .iter()
            .position(|value| *value == object_id)
            .expect("parent did not contain child");
        self.elements
            .get_mut(&parent_id)
            .expect("parent disappeared")
            .children
            .retain(|value| *value != object_id);
        self.remove_subtree(object_id);
        self.clamp_tab_selection(parent_id);
        let child_count = self.elements[&parent_id].children.len();
        choice_groups::remove(
            &mut self
                .elements
                .get_mut(&parent_id)
                .expect("parent disappeared")
                .element,
            removed_index,
            child_count,
        );
        self.journal.push(UiJournalEntry::Destroy(object_id));
        Ok(())
    }

    /// Validates actions whose observable result belongs to the native UI runtime.
    pub fn perform_action(
        &mut self,
        object_id: ObjectId,
        action: &VisualElementAction,
    ) -> Result<(), UiWorldError> {
        let target = self
            .elements
            .get(&object_id)
            .ok_or(UiWorldError::UnknownObject)?;
        let VisualElementAction::ScrollTo { descendant_id } = action else {
            return Err(UiWorldError::UnsupportedAction);
        };
        if target.kind() != UiElementKind::ScrollView {
            return Err(UiWorldError::InvalidProperty);
        }
        let mut cursor = Some(*descendant_id);
        while let Some(value) = cursor {
            if value == object_id {
                return Ok(());
            }
            cursor = self
                .elements
                .get(&value)
                .ok_or(UiWorldError::UnknownObject)?
                .parent_id;
        }
        Err(UiWorldError::InvalidHierarchy)
    }

    fn insert_subtree(
        &mut self,
        parent_id: Option<ObjectId>,
        node: UiNode,
        is_document_root: bool,
        document_root_id: ObjectId,
    ) -> Result<(), UiWorldError> {
        if self.elements.contains_key(&node.object_id) {
            return Err(UiWorldError::DuplicateObject);
        }
        let object_id = node.object_id;
        let child_ids = node.children.iter().map(|child| child.object_id).collect();
        let source = match &node.element {
            UiElement::Image(value) => value.source.clone(),
            _ => None,
        };
        let icon = match &node.element {
            UiElement::Button(value) => value.icon.clone(),
            UiElement::Tab(value) => value.icon.clone(),
            _ => None,
        };
        let background = match &node.element.visual_element().style.background_image {
            Some(StyleValue::Value(value)) => Some(value.clone()),
            Some(StyleValue::Keyword { .. }) | None => None,
        };
        let material = match &node.element.visual_element().style.unity_material {
            Some(StyleValue::Value(value)) => Some(value.clone()),
            Some(StyleValue::Keyword { .. }) | None => None,
        };
        let cursor = match &node.element.visual_element().style.cursor {
            Some(StyleValue::Value(Cursor::Texture { address, .. })) => Some(address.clone()),
            Some(StyleValue::Value(Cursor::Default)) | Some(StyleValue::Keyword { .. }) | None => {
                None
            }
        };
        let part_assets = part_assets(&node.element);
        let state = UiElementState {
            object_id,
            element: node.element,
            parent_id,
            document_root_id,
            children: child_ids,
            is_document_root,
        };
        self.elements.insert(object_id, state);
        if let Some(source) = source {
            self.retain_source(source);
        }
        if let Some(source) = icon {
            self.retain_icon(source);
        }
        if let Some(source) = background {
            self.retain_background(source);
        }
        if let Some(source) = material {
            self.retain_material(source);
        }
        if let Some(source) = cursor {
            self.retain_cursor(source);
        }
        self.retain_part_assets(part_assets);
        for child in node.children {
            self.insert_subtree(Some(object_id), child, false, document_root_id)?;
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
        hierarchy::require_container(parent.element.kind())?;
        hierarchy::require_placement(self.elements[&object_id].kind(), parent.element.kind())?;
        let old_parent = self.elements[&object_id]
            .parent_id
            .expect("non-root has no parent");
        if old_parent != parent_id
            && parent.kind() == UiElementKind::ToggleButtonGroup
            && parent.children.len() >= 64
        {
            return Err(UiWorldError::InvalidHierarchy);
        }
        if self.elements[&object_id].document_root_id != parent.document_root_id {
            return Err(UiWorldError::InvalidHierarchy);
        }
        let destination_len = parent.children.len()
            - usize::from(self.elements[&object_id].parent_id == Some(parent_id));
        let index = child_index.map_or(destination_len, |value| value as usize);
        if index > destination_len || self.is_descendant(parent_id, object_id) {
            return Err(UiWorldError::InvalidHierarchy);
        }
        if self.depth(parent_id) + self.subtree_depth(object_id) + 1 > MAXIMUM_HIERARCHY_DEPTH {
            return Err(UiWorldError::InvalidHierarchy);
        }
        let previous_index = self.elements[&old_parent]
            .children
            .iter()
            .position(|value| *value == object_id)
            .expect("parent did not contain child");
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
        self.clamp_tab_selection(old_parent);
        self.clamp_tab_selection(parent_id);
        if old_parent == parent_id {
            let child_count = self.elements[&parent_id].children.len();
            choice_groups::reorder(
                &mut self
                    .elements
                    .get_mut(&parent_id)
                    .expect("parent disappeared")
                    .element,
                previous_index,
                index,
                child_count,
            );
        } else {
            let old_child_count = self.elements[&old_parent].children.len();
            choice_groups::remove(
                &mut self
                    .elements
                    .get_mut(&old_parent)
                    .expect("old parent disappeared")
                    .element,
                previous_index,
                old_child_count,
            );
            let new_child_count = self.elements[&parent_id].children.len();
            choice_groups::insert(
                &mut self
                    .elements
                    .get_mut(&parent_id)
                    .expect("new parent disappeared")
                    .element,
                index,
                new_child_count,
            );
        }
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

    fn depth(&self, object_id: ObjectId) -> usize {
        let mut depth = 0;
        let mut cursor = self.elements[&object_id].parent_id;
        while let Some(value) = cursor {
            depth += 1;
            cursor = self.elements[&value].parent_id;
        }
        depth
    }

    fn subtree_depth(&self, object_id: ObjectId) -> usize {
        self.elements[&object_id]
            .children
            .iter()
            .map(|child| self.subtree_depth(*child) + 1)
            .max()
            .unwrap_or(0)
    }

    fn clamp_tab_selection(&mut self, object_id: ObjectId) {
        let child_count = self.elements[&object_id].children.len();
        let UiElement::TabView(value) = &mut self
            .elements
            .get_mut(&object_id)
            .expect("tab parent disappeared")
            .element
        else {
            return;
        };
        if child_count == 0 {
            value.selected_tab_index = None;
        } else if let Some(index) = value.selected_tab_index {
            value.selected_tab_index = Some(index.min((child_count - 1) as u32));
        }
    }

    fn remove_subtree(&mut self, object_id: ObjectId) {
        let children = self.elements[&object_id].children.clone();
        for child in children {
            self.remove_subtree(child);
        }
        if let Some(source) = self.elements[&object_id].image_source().cloned() {
            self.release_source(&source);
        }
        if let Some(source) = self.elements[&object_id].icon_source().cloned() {
            self.release_icon(&source);
        }
        if let Some(source) = self.elements[&object_id].background_source().cloned() {
            self.release_background(&source);
        }
        if let Some(source) = self.elements[&object_id].material_source().cloned() {
            self.release_material(&source);
        }
        if let Some(source) = self.elements[&object_id].cursor_source().cloned() {
            self.release_cursor(&source);
        }
        self.release_part_assets(part_assets(self.elements[&object_id].element()));
        self.elements.remove(&object_id);
    }

    fn retain_source(&mut self, source: ImageSource) {
        *self.asset_usage.entry(source).or_default() += 1;
    }

    fn release_source(&mut self, source: &ImageSource) {
        let count = self
            .asset_usage
            .get_mut(source)
            .expect("live image source had no usage count");
        *count -= 1;
        if *count == 0 {
            self.asset_usage.remove(source);
        }
    }

    fn retain_icon(&mut self, source: IconSource) {
        *self.icon_usage.entry(source).or_default() += 1;
    }

    fn release_icon(&mut self, source: &IconSource) {
        let count = self
            .icon_usage
            .get_mut(source)
            .expect("live button icon had no usage count");
        *count -= 1;
        if *count == 0 {
            self.icon_usage.remove(source);
        }
    }

    fn retain_material(&mut self, source: MaterialAddress) {
        *self.material_usage.entry(source).or_default() += 1;
    }

    fn retain_background(&mut self, source: BackgroundSource) {
        *self.background_usage.entry(source).or_default() += 1;
    }

    fn retain_cursor(&mut self, source: TextureAddress) {
        *self.cursor_usage.entry(source).or_default() += 1;
    }

    fn release_cursor(&mut self, source: &TextureAddress) {
        let count = self
            .cursor_usage
            .get_mut(source)
            .expect("live UI cursor had no usage count");
        *count -= 1;
        if *count == 0 {
            self.cursor_usage.remove(source);
        }
    }

    fn release_background(&mut self, source: &BackgroundSource) {
        let count = self
            .background_usage
            .get_mut(source)
            .expect("live UI background had no usage count");
        *count -= 1;
        if *count == 0 {
            self.background_usage.remove(source);
        }
    }

    fn release_material(&mut self, source: &MaterialAddress) {
        let count = self
            .material_usage
            .get_mut(source)
            .expect("live UI material had no usage count");
        *count -= 1;
        if *count == 0 {
            self.material_usage.remove(source);
        }
    }

    fn retain_part_assets(&mut self, assets: PartAssets) {
        for source in assets.backgrounds {
            self.retain_background(source);
        }
        for source in assets.cursors {
            self.retain_cursor(source);
        }
        for source in assets.materials {
            self.retain_material(source);
        }
    }

    fn release_part_assets(&mut self, assets: PartAssets) {
        for source in assets.backgrounds {
            self.release_background(&source);
        }
        for source in assets.cursors {
            self.release_cursor(&source);
        }
        for source in assets.materials {
            self.release_material(&source);
        }
    }
}

#[derive(Default)]
struct PartAssets {
    backgrounds: Vec<BackgroundSource>,
    cursors: Vec<TextureAddress>,
    materials: Vec<MaterialAddress>,
}

fn part_assets(element: &UiElement) -> PartAssets {
    let mut result = PartAssets::default();
    for style in authored_private_part_styles(element) {
        if let Some(StyleValue::Value(value)) = &style.background_image {
            result.backgrounds.push(value.clone());
        }
        if let Some(StyleValue::Value(Cursor::Texture { address, .. })) = &style.cursor {
            result.cursors.push(address.clone());
        }
        if let Some(StyleValue::Value(value)) = &style.unity_material {
            result.materials.push(value.clone());
        }
    }
    result
}

fn map_validation_error(value: battlement_ui::UiValidationError) -> UiWorldError {
    match value {
        battlement_ui::UiValidationError::DuplicateObject => UiWorldError::DuplicateObject,
        battlement_ui::UiValidationError::InvalidHierarchy => UiWorldError::InvalidHierarchy,
        battlement_ui::UiValidationError::InvalidProperty
        | battlement_ui::UiValidationError::InvalidReference => UiWorldError::InvalidProperty,
    }
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
