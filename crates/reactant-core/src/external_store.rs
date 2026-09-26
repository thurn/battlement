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
    let snapshot = crate::store_snapshot::read(&source);
    Self {
      committed_source: source.clone(),
      committed_snapshot: snapshot.clone(),
      rendered_source: source,
      rendered_snapshot: snapshot,
      active: Rc::new(RefCell::new(None)),
      committed: false,
      frozen_wake: false,
    }
  }

  pub(crate) fn prepare(&mut self, source: S) {
    self.rendered_snapshot = crate::store_snapshot::read(&source);
    self.rendered_source = source;
  }

  pub(crate) fn snapshot(&self) -> S::Snapshot {
    self.rendered_snapshot.clone()
  }

  fn source_changed(&self) -> bool {
    !self.committed || self.committed_source != self.rendered_source
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
    Box::new(self.clone())
  }

  fn commit(&mut self) {
    if self.source_changed() {
      let wake = Arc::new(AtomicBool::new(false));
      let subscription = context::with_hooks_forbidden(|| {
        self.rendered_source.subscribe(StoreNotify {
          wake: Arc::clone(&wake),
        })
      });
      if context::with_hooks_forbidden(|| self.rendered_source.snapshot()) != self.rendered_snapshot
      {
        wake.store(true, Ordering::Release);
      }
      let next = StoreGeneration {
        wake,
        _subscription: subscription,
      };
      let previous = self.active.borrow_mut().replace(next);
      drop(previous);
    }
    self.committed_source.clone_from(&self.rendered_source);
    self.committed_snapshot.clone_from(&self.rendered_snapshot);
    self.committed = true;
    self.frozen_wake = false;
  }

  fn discard_pending(&mut self) {
    self.rendered_source.clone_from(&self.committed_source);
    self.rendered_snapshot.clone_from(&self.committed_snapshot);
    self.frozen_wake = false;
  }

  fn has_pending(&self) -> bool {
    self.frozen_wake
      || self
        .active
        .borrow()
        .as_ref()
        .is_some_and(|generation| generation.wake.load(Ordering::Acquire))
  }

  fn has_pending_change(&self) -> bool {
    self.frozen_wake
      && crate::store_snapshot::read(&self.committed_source) != self.committed_snapshot
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

impl<S: ExternalStore> Clone for StoreSlot<S> {
  fn clone(&self) -> Self {
    assert!(
      self.committed,
      "Reactant cannot clone an uncommitted external store"
    );
    Self {
      committed_source: self.committed_source.clone(),
      committed_snapshot: self.committed_snapshot.clone(),
      rendered_source: self.committed_source.clone(),
      rendered_snapshot: self.committed_snapshot.clone(),
      active: Rc::clone(&self.active),
      committed: true,
      frozen_wake: self.frozen_wake,
    }
  }
}

/// Reads selected store data, suppressing reevaluation when the selected value is equal.
pub fn use_external_store_selector<S, V>(
  source: S,
  select: impl Fn(&S::Snapshot) -> V + 'static,
) -> V
where
  S: ExternalStore,
  V: Clone + PartialEq + 'static,
{
  self::use_external_store_selector_with(source, select, PartialEq::eq)
}

/// Reads selected store data with an explicit comparison for consumer updates.
pub fn use_external_store_selector_with<S, V>(
  source: S,
  select: impl Fn(&S::Snapshot) -> V + 'static,
  equal: impl Fn(&V, &V) -> bool + 'static,
) -> V
where
  S: ExternalStore,
  V: Clone + 'static,
{
  let select: Selector<S, V> = Rc::new(select);
  let equal: Equal<V> = Rc::new(equal);
  crate::hooks::use_slot(
    HookKind::Store,
    TypeId::of::<(S, V, SelectorMarker)>(),
    |_| SelectedStoreSlot::new(source.clone(), select.clone(), equal.clone()),
    |slot| {
      slot.store.prepare(source.clone());
      slot.rendered_value = context::with_hooks_forbidden(|| select(&slot.store.rendered_snapshot));
      slot.rendered_select = select.clone();
      slot.rendered_equal = equal.clone();
      slot.rendered_value.clone()
    },
  )
}

type Selector<S, V> = Rc<dyn Fn(&<S as ExternalStore>::Snapshot) -> V>;
type Equal<V> = Rc<dyn Fn(&V, &V) -> bool>;

struct SelectorMarker;
struct SelectedStoreSlot<S: ExternalStore, V> {
  store: StoreSlot<S>,
  committed_value: V,
  rendered_value: V,
  committed_select: Selector<S, V>,
  rendered_select: Selector<S, V>,
  committed_equal: Equal<V>,
  rendered_equal: Equal<V>,
}

impl<S: ExternalStore, V: Clone + 'static> SelectedStoreSlot<S, V> {
  fn new(source: S, select: Selector<S, V>, equal: Equal<V>) -> Self {
    let store = StoreSlot::new(source);
    let value = context::with_hooks_forbidden(|| select(&store.rendered_snapshot));
    Self {
      store,
      committed_value: value.clone(),
      rendered_value: value,
      committed_select: select.clone(),
      rendered_select: select,
      committed_equal: equal.clone(),
      rendered_equal: equal,
    }
  }
}

impl<S: ExternalStore, V: Clone + 'static> HookSlot for SelectedStoreSlot<S, V> {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      store: self.store.clone(),
      committed_value: self.committed_value.clone(),
      rendered_value: self.committed_value.clone(),
      committed_select: self.committed_select.clone(),
      rendered_select: self.committed_select.clone(),
      committed_equal: self.committed_equal.clone(),
      rendered_equal: self.committed_equal.clone(),
    })
  }
  fn commit(&mut self) {
    self.store.commit();
    self.committed_value.clone_from(&self.rendered_value);
    self.committed_select = self.rendered_select.clone();
    self.committed_equal = self.rendered_equal.clone();
  }
  fn discard_pending(&mut self) {
    self.store.discard_pending();
    self.rendered_value.clone_from(&self.committed_value);
    self.rendered_select = self.committed_select.clone();
    self.rendered_equal = self.committed_equal.clone();
  }
  fn has_pending(&self) -> bool {
    self.store.has_pending()
  }
  fn has_pending_change(&self) -> bool {
    self.store.frozen_wake
      && context::with_hooks_forbidden(|| {
        let snapshot = crate::store_snapshot::read(&self.store.committed_source);
        let value = (self.committed_select)(&snapshot);
        !(self.committed_equal)(&self.committed_value, &value)
      })
  }
  fn context_changed(&self) -> bool {
    false
  }
  fn kind(&self) -> HookKind {
    HookKind::Store
  }
  fn value_type(&self) -> TypeId {
    TypeId::of::<(S, V, SelectorMarker)>()
  }
  fn freeze_store_wake(&mut self) {
    self.store.freeze_store_wake();
  }
  fn unmount_store(&mut self) {
    self.store.unmount_store();
  }
}
