//! Entry transactions and frozen runtime inputs.

use std::{
  cell::RefCell,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
  thread,
};

use battlement::{self, CommandBody, UiDocument};

use crate::{
  effect::EffectOperation,
  element_ref::{AttachmentSet, ElementRefRuntime},
  external_portal::{ExternalPortalRegistry, SessionExternal},
  geometry_runtime::{GeometryPlan, GeometryRuntime},
  portal, reconcile,
  render::RenderTree,
  resource_cache::{FrozenCompletions, PanicPayload, ResourceOverlay},
  resource_runtime::ResourceRuntime,
  root_view::RootRegistration,
  runtime_document,
};

pub(crate) struct EntryCheckpoint {
  actions: usize,
  hooks: Vec<Vec<usize>>,
}

pub(crate) struct FrozenResources {
  completions: Option<FrozenCompletions>,
  overlay: Rc<ResourceOverlay>,
  resources: Rc<ResourceRuntime>,
}

pub(crate) struct PlannedSession {
  pub(crate) documents: Vec<UiDocument>,
  pub(crate) committed: Vec<RenderTree>,
  pub(crate) external: SessionExternal,
  pub(crate) resource_completions: FrozenCompletions,
  pub(crate) attachments: AttachmentSet,
  pub(crate) geometry: GeometryPlan,
  pub(crate) frozen_actions: usize,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum RuntimeState {
  Registering,
  Active,
  Closed,
  Poisoned,
}

pub(crate) fn run_or_poison(state: &mut RuntimeState, operation: impl FnOnce()) {
  if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(operation)) {
    *state = RuntimeState::Poisoned;
    panic::resume_unwind(payload);
  }
}

pub(crate) fn drop_runtime<G>(
  state: &mut RuntimeState,
  roots: &mut [RootRegistration<G>],
  pending_effects: &mut Vec<EffectOperation>,
  element_refs: &Rc<RefCell<ElementRefRuntime>>,
) {
  if matches!(*state, RuntimeState::Closed | RuntimeState::Registering) {
    return;
  }
  let healthy = *state == RuntimeState::Active;
  *state = RuntimeState::Poisoned;
  cleanup_passive(roots, pending_effects, element_refs);
  if healthy && !thread::panicking() {
    panic!("an active Reactant runtime was dropped without shutdown");
  }
}

impl EntryCheckpoint {
  pub(crate) fn capture<'a>(
    trees: impl IntoIterator<Item = &'a RenderTree>,
    element_refs: &Rc<RefCell<ElementRefRuntime>>,
  ) -> Self {
    Self {
      actions: element_refs.borrow().queued_actions(),
      hooks: trees
        .into_iter()
        .map(|tree| {
          let mut lengths = Vec::new();
          tree.pending_hook_lengths(&mut lengths);
          lengths
        })
        .collect(),
    }
  }

  pub(crate) fn discard_actions(&self, element_refs: &Rc<RefCell<ElementRefRuntime>>) {
    element_refs.borrow_mut().truncate_actions(self.actions);
  }

  pub(crate) fn rollback<'a>(
    &self,
    trees: impl IntoIterator<Item = &'a RenderTree>,
    element_refs: &Rc<RefCell<ElementRefRuntime>>,
  ) {
    for (tree, lengths) in trees.into_iter().zip(&self.hooks) {
      let mut cursor = 0;
      tree.truncate_pending_hooks(lengths, &mut cursor);
      assert_eq!(cursor, lengths.len());
    }
    self.discard_actions(element_refs);
  }
}

impl FrozenResources {
  pub(crate) fn freeze(resources: Rc<ResourceRuntime>) -> Result<Self, PanicPayload> {
    let mut cache = resources.cache.borrow_mut();
    let mut completions = cache.freeze();
    if let Some(payload) = cache.current_panic(&mut completions) {
      cache.restore(completions);
      return Err(payload);
    }
    let overlay = Rc::new(cache.overlay(&completions));
    drop(cache);
    Ok(Self {
      completions: Some(completions),
      overlay,
      resources,
    })
  }

  pub(crate) fn from_frozen(
    resources: Rc<ResourceRuntime>,
    completions: FrozenCompletions,
  ) -> Self {
    let overlay = Rc::new(resources.cache.borrow().overlay(&completions));
    Self {
      completions: Some(completions),
      overlay,
      resources,
    }
  }

  pub(crate) fn apply(&mut self) -> Result<(), PanicPayload> {
    self.resources.cache.borrow_mut().apply(
      self
        .completions
        .take()
        .expect("Reactant resource transaction applies once"),
    )
  }

  pub(crate) fn overlay(&self) -> Rc<ResourceOverlay> {
    Rc::clone(&self.overlay)
  }

  pub(crate) fn changed(&self) -> bool {
    !self.overlay.is_empty()
  }

  pub(crate) fn take(&mut self) -> FrozenCompletions {
    self
      .completions
      .take()
      .expect("Reactant resource transaction is taken once")
  }
}

impl Drop for FrozenResources {
  fn drop(&mut self) {
    let Some(completions) = self.completions.take() else {
      return;
    };
    self.resources.cache.borrow_mut().restore(completions);
  }
}

pub(crate) fn cleanup_passive<G>(
  roots: &mut [RootRegistration<G>],
  pending: &mut Vec<EffectOperation>,
  element_refs: &Rc<RefCell<ElementRefRuntime>>,
) {
  let mut effects = std::mem::take(pending);
  for root in roots {
    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
      root.committed.unmount_all_effects(&mut effects);
    }));
  }
  for effect in effects {
    let _ = panic::catch_unwind(AssertUnwindSafe(|| effect.run_cleanup()));
  }
  element_refs.borrow_mut().detach_all();
}

pub(crate) fn plan_shutdown<G>(
  runtime_id: u64,
  roots: &[RootRegistration<G>],
  external_portals: &ExternalPortalRegistry,
  geometry_runtime: &GeometryRuntime,
) -> (Vec<Vec<CommandBody>>, GeometryPlan) {
  let bindings = external_portals.active_bindings();
  let previous_trees = roots
    .iter()
    .map(|root| root.committed.clone())
    .collect::<Vec<_>>();
  let empty_trees = roots
    .iter()
    .map(|_| RenderTree::default())
    .collect::<Vec<_>>();
  let previous = portal::layout(runtime_id, &previous_trees, &bindings);
  let desired = portal::layout(runtime_id, &empty_trees, &bindings);
  let documents = roots
    .iter()
    .zip(&desired.roots)
    .map(|(root, physical)| runtime_document::render(&root.document, physical))
    .collect::<Vec<_>>();
  battlement::validate_documents(&documents)
    .expect("Reactant planned an invalid shutdown hierarchy");
  let groups = roots
    .iter()
    .zip(previous.roots.iter().zip(&desired.roots))
    .map(|(root, (previous, desired))| {
      let groups =
        reconcile::command_groups(root.document.root_id, &previous.hosts, &desired.hosts);
      runtime_document::with_coverage_barrier(root.document.root_id, previous, desired, groups)
    })
    .fold(Vec::new(), self::merge_groups);
  let groups = self::merge_groups(
    groups,
    external_portals.active_groups(&previous, &desired, &documents),
  );
  let attachments = AttachmentSet::collect(
    runtime_id,
    roots
      .iter()
      .zip(&empty_trees)
      .map(|(root, tree)| (root.document.document_id, tree)),
  );
  let geometry = geometry_runtime
    .plan(&[], &attachments, false)
    .expect("Reactant planned an invalid shutdown geometry registry");
  (geometry.command_groups(groups), geometry)
}

fn merge_groups(
  mut merged: Vec<Vec<CommandBody>>,
  groups: Vec<Vec<CommandBody>>,
) -> Vec<Vec<CommandBody>> {
  for (index, group) in groups.into_iter().enumerate() {
    if index == merged.len() {
      merged.push(group);
    } else {
      merged[index].extend(group);
    }
  }
  merged
}
