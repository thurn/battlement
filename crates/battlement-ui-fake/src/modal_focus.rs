use std::collections::HashMap;

use battlement_types::ObjectId;
use battlement_ui::{OverlayPlacement, Prop, UiVisualElementProperties};

use crate::{UiWorld, actions};

#[derive(Default, Debug, Clone)]
pub(crate) struct ModalFocus {
  active: Option<ObjectId>,
  application_return: Option<ObjectId>,
  restore: Option<ObjectId>,
  last_focused: HashMap<ObjectId, ObjectId>,
}

impl UiWorld {
  /// Resolves modal focus after committed UI changes, without advancing time.
  pub fn reconcile_modal_focus(&mut self) -> Option<ObjectId> {
    let mut focused = self.focused;
    let mut roots = self
      .elements
      .values()
      .filter(|e| e.is_document_root)
      .map(|e| e.object_id)
      .collect::<Vec<_>>();
    roots.sort();
    self.modal_focus.retain(|root, _| roots.contains(root));
    for root in roots {
      let mut state = self.modal_focus.remove(&root).unwrap_or_default();
      let next = self.active_modal(root);
      if next != state.active {
        if state.active.is_none() && next.is_some() {
          state.application_return = focused.filter(|id| {
            self
              .elements
              .get(id)
              .is_some_and(|e| e.document_root_id == root)
          });
        }
        focused = match next {
          Some(id) => Some(self.modal_initial_focus(id, &state)),
          None => state
            .restore
            .filter(|id| self.is_focus_eligible(*id))
            .or_else(|| {
              state
                .application_return
                .filter(|id| self.is_focus_eligible(*id))
            }),
        };
        state.active = next;
        if next.is_none() {
          state.application_return = None;
        }
      } else if let Some(id) = next
        && !focused.is_some_and(|target| self.focus_in_modal(target, id))
      {
        focused = Some(self.modal_initial_focus(id, &state));
      }
      if let Some(id) = next {
        state.restore = match self.elements[&id]
          .element
          .visual_element()
          .overlay_placement
        {
          Prop::Set(OverlayPlacement::Modal { restore_focus, .. }) => restore_focus,
          _ => unreachable!("active modal must have modal placement"),
        };
        if let Some(target) = focused {
          state.last_focused.insert(id, target);
        }
      }
      state
        .last_focused
        .retain(|id, _| self.elements.contains_key(id));
      if next.is_some() {
        self.modal_focus.insert(root, state);
      }
    }
    focused
  }

  pub(crate) fn focus_traversal(&self, root: ObjectId) -> Vec<ObjectId> {
    let mut result = vec![root];
    for child in &self.elements[&root].children {
      result.extend(self.focus_traversal(*child));
    }
    result
  }

  /// Whether a mounted control accepts explicit focus, including negative tab indices.
  pub fn is_focus_eligible(&self, id: ObjectId) -> bool {
    self.input_eligible(id) && actions::focusable(&self.elements[&id])
  }

  fn focus_in_modal(&self, target: ObjectId, scope: ObjectId) -> bool {
    self.is_focus_eligible(target)
      && self.elements[&target].document_root_id == self.elements[&scope].document_root_id
      && self.in_modal_scope(target, scope)
  }

  fn modal_initial_focus(&self, id: ObjectId, state: &ModalFocus) -> ObjectId {
    if let Some(target) = state
      .last_focused
      .get(&id)
      .copied()
      .filter(|target| self.focus_in_modal(*target, id))
    {
      return target;
    }
    if let Prop::Set(OverlayPlacement::Modal {
      initial_focus: Some(target),
      ..
    }) = self.elements[&id]
      .element
      .visual_element()
      .overlay_placement
      && self.focus_in_modal(target, id)
    {
      return target;
    }
    let root = self.elements[&id].document_root_id;
    let mut targets = self
      .focus_traversal(root)
      .into_iter()
      .filter(|target| *target != id && self.focus_in_modal(*target, id))
      .filter(|target| self.elements[target].tab_index().unwrap_or(0) >= 0)
      .collect::<Vec<_>>();
    targets.sort_by_key(
      |target| match self.elements[target].tab_index().unwrap_or(0) {
        0 => i32::MAX,
        value => value,
      },
    );
    targets.first().copied().unwrap_or(id)
  }
}
