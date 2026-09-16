//! Thread-safe external-store snapshots and subscription lifetimes.

use std::{
  any::{Any, TypeId},
  cell::RefCell,
  rc::Rc,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

use crate::{
  context,
  hook_storage::{HookKind, HookSlot},
};

/// Supplies comparable snapshots and change notifications from outside Reactant.
pub trait ExternalStore: Clone + PartialEq + Send + Sync + 'static {
  /// The immutable value read during rendering.
  type Snapshot: Clone + PartialEq + Send + Sync + 'static;

  /// Reads the store's current snapshot.
  fn snapshot(&self) -> Self::Snapshot;

  /// Registers a change callback and returns its lifetime guard.
  fn subscribe(&self, notify: StoreNotify) -> Subscription;
}

/// Queues one coalesced store wake for its subscription generation.
#[derive(Clone)]
pub struct StoreNotify {
  wake: Arc<AtomicBool>,
}

/// Unsubscribes an external-store listener when dropped.
pub struct Subscription {
  cleanup: Option<Box<dyn FnOnce() + Send>>,
}

pub(crate) struct StoreSlot<S>
where
  S: ExternalStore,
{
  committed_source: S,
  committed_snapshot: S::Snapshot,
  rendered_source: S,
  rendered_snapshot: S::Snapshot,
  active: Rc<RefCell<Option<StoreGeneration>>>,
  provisional_source: Option<S>,
  provisional: Option<StoreGeneration>,
  committed: bool,
  frozen_wake: bool,
}

impl StoreNotify {
  /// Queues a wake without reading the store on the calling thread.
  pub fn notify(&self) {
    self.wake.store(true, Ordering::Release);
  }
}

impl Subscription {
  /// Creates a guard that runs `cleanup` at most once when dropped.
  pub fn new(cleanup: impl FnOnce() + Send + 'static) -> Self {
    Self {
      cleanup: Some(Box::new(cleanup)),
    }
  }
}

impl Drop for Subscription {
  fn drop(&mut self) {
    if let Some(cleanup) = self.cleanup.take() {
      cleanup();
    }
  }
}

impl<S> StoreSlot<S>
where
  S: ExternalStore,
{
  pub(crate) fn new(source: S) -> Self {
    let snapshot = context::with_hooks_forbidden(|| source.snapshot());
    Self {
      committed_source: source.clone(),
      committed_snapshot: snapshot.clone(),
      rendered_source: source,
      rendered_snapshot: snapshot,
      active: Rc::new(RefCell::new(None)),
      provisional_source: None,
      provisional: None,
      committed: false,
      frozen_wake: false,
    }
  }

  pub(crate) fn prepare(&mut self, source: S) {
    if self.provisional_source.as_ref() != Some(&source) {
      self.provisional = None;
      self.provisional_source = None;
    }
    self.rendered_snapshot = context::with_hooks_forbidden(|| source.snapshot());
    self.rendered_source = source;
  }

  pub(crate) fn snapshot(&self) -> S::Snapshot {
    self.rendered_snapshot.clone()
  }

  fn source_changed(&self) -> bool {
    !self.committed || self.committed_source != self.rendered_source
  }

  fn stabilize(&mut self) -> bool {
    if !self.source_changed() {
      return false;
    }
    if self.provisional.is_none() {
      let wake = Arc::new(AtomicBool::new(false));
      let notify = StoreNotify {
        wake: Arc::clone(&wake),
      };
      let subscription = context::with_hooks_forbidden(|| self.rendered_source.subscribe(notify));
      self.provisional_source = Some(self.rendered_source.clone());
      self.provisional = Some(StoreGeneration {
        wake,
        _subscription: subscription,
      });
    }
    self
      .provisional
      .as_ref()
      .expect("provisional Reactant store subscription exists")
      .wake
      .swap(false, Ordering::AcqRel);
    context::with_hooks_forbidden(|| self.rendered_source.snapshot()) != self.rendered_snapshot
  }

  fn freeze_wake(&mut self) {
    let notified = self
      .active
      .borrow()
      .as_ref()
      .is_some_and(|generation| generation.wake.swap(false, Ordering::AcqRel));
    self.frozen_wake |= notified;
  }

  fn unsubscribe(&mut self) {
    self.frozen_wake = false;
    self.provisional_source = None;
    self.provisional = None;
    let active = self.active.borrow_mut().take();
    drop(active);
  }
}

impl<S> HookSlot for StoreSlot<S>
where
  S: ExternalStore,
{
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    assert!(
      self.committed && self.provisional.is_none(),
      "Reactant cannot clone an uncommitted external store"
    );
    Box::new(Self {
      committed_source: self.committed_source.clone(),
      committed_snapshot: self.committed_snapshot.clone(),
      rendered_source: self.committed_source.clone(),
      rendered_snapshot: self.committed_snapshot.clone(),
      active: Rc::clone(&self.active),
      provisional_source: None,
      provisional: None,
      committed: true,
      frozen_wake: self.frozen_wake,
    })
  }

  fn commit(&mut self) {
    if self.source_changed() {
      let next = self
        .provisional
        .take()
        .expect("changed Reactant store has a provisional subscription");
      let previous = self.active.borrow_mut().replace(next);
      drop(previous);
    }
    self.committed_source.clone_from(&self.rendered_source);
    self.committed_snapshot.clone_from(&self.rendered_snapshot);
    self.provisional_source = None;
    self.committed = true;
    self.frozen_wake = false;
  }

  fn discard_pending(&mut self) {
    self.rendered_source.clone_from(&self.committed_source);
    self.rendered_snapshot.clone_from(&self.committed_snapshot);
    self.provisional_source = None;
    self.provisional = None;
    self.frozen_wake = false;
  }

  fn has_pending(&self) -> bool {
    self.frozen_wake
  }

  fn has_pending_change(&self) -> bool {
    self.frozen_wake
      && context::with_hooks_forbidden(|| self.committed_source.snapshot())
        != self.committed_snapshot
  }

  fn context_changed(&self) -> bool {
    false
  }

  fn kind(&self) -> HookKind {
    HookKind::Store
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<(S, S::Snapshot)>()
  }

  fn stabilize_store(&mut self) -> bool {
    self.stabilize()
  }

  fn freeze_store_wake(&mut self) {
    self.freeze_wake();
  }

  fn unmount_store(&mut self) {
    self.unsubscribe();
  }
}

struct StoreGeneration {
  wake: Arc<AtomicBool>,
  _subscription: Subscription,
}
