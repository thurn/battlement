//! On-demand observations of committed presentation declarations.

use battlement::ObjectId;
use uuid::Uuid;

use crate::{app::App, render::RenderTree, runtime::Reactant};

/// The latest Rust declaration, which may be ahead of native command playback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationObservation {
  /// The selected application-wide presentation identity.
  pub id: Uuid,
  /// Its logical document root, independent of portal placement.
  pub logical_root: ObjectId,
  /// Identified logical ancestors, from outermost to innermost.
  pub ancestors: Vec<Uuid>,
  /// Native handles contributed by this declaration, including portal descendants.
  pub native_objects: Vec<ObjectId>,
}

impl<G: 'static> App<G> {
  /// Observes one declared identity without polling or advancing presentation.
  pub fn presentation(&self, id: Uuid) -> Option<PresentationObservation> {
    self.runtime.presentation(id)
  }
}

impl<G: 'static> Reactant<G> {
  /// Observes one declared identity without polling or advancing presentation.
  pub fn presentation(&self, id: Uuid) -> Option<PresentationObservation> {
    self
      .roots
      .iter()
      .find_map(|root| self::find(&root.committed, id, root.document.root_id, &mut Vec::new()))
  }
}

fn find(
  tree: &RenderTree,
  id: Uuid,
  root: ObjectId,
  ancestors: &mut Vec<Uuid>,
) -> Option<PresentationObservation> {
  for position in &tree.positions {
    if position.terminal_visual {
      continue;
    }
    if position.presentation_id == Some(id) {
      let mut native_objects = Vec::new();
      if let Some(host) = &position.host {
        native_objects.push(host.object_id);
      }
      self::hosts(&position.children, &mut native_objects);
      return Some(PresentationObservation {
        id,
        logical_root: root,
        ancestors: ancestors.clone(),
        native_objects,
      });
    }
    if let Some(ancestor) = position.presentation_id {
      ancestors.push(ancestor);
    }
    let found = self::find(&position.children, id, root, ancestors).or_else(|| {
      position
        .suspense
        .as_ref()
        .and_then(|suspense| self::find(&suspense.primary, id, root, ancestors))
    });
    if position.presentation_id.is_some() {
      ancestors.pop();
    }
    if found.is_some() {
      return found;
    }
  }
  None
}

fn hosts(tree: &RenderTree, objects: &mut Vec<ObjectId>) {
  for position in &tree.positions {
    if position.terminal_visual {
      continue;
    }
    if let Some(host) = &position.host {
      objects.push(host.object_id);
    }
    self::hosts(&position.children, objects);
  }
}
