//! Positional component state hooks.

use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  ptr,
  rc::{Rc, Weak},
};

use crate::context;

const RENDER_RETRY_LIMIT: usize = 25;

thread_local! {
  static CURRENT: RefCell<Option<Rc<RefCell<HookAttempt>>>> = const { RefCell::new(None) };
}

/// Queues state replacements and updater functions for one mounted hook.
pub struct StateSetter<T> {
  queue: Weak<StateQueue<T>>,
}

/// Queues actions for one mounted reducer hook.
pub struct ReducerDispatch<A> {
  queue: Weak<ReducerQueue<A>>,
}

impl<T> Clone for StateSetter<T> {
  fn clone(&self) -> Self {
    Self {
      queue: self.queue.clone(),
    }
  }
}

impl<T> PartialEq for StateSetter<T> {
  fn eq(&self, other: &Self) -> bool {
    Weak::ptr_eq(&self.queue, &other.queue)
  }
}

impl<T> Eq for StateSetter<T> {}

impl<T: Clone + 'static> StateSetter<T> {
  /// Queues a replacement value for the next render.
  pub fn set(&self, value: T) {
    if let Some(queue) = self.queue.upgrade() {
      queue.enqueue(StateUpdate::Replace(value));
    }
  }

  /// Queues an updater against the value produced by earlier queue entries.
  pub fn update(&self, update: impl Fn(T) -> T + 'static) {
    if let Some(queue) = self.queue.upgrade() {
      queue.enqueue(StateUpdate::Update(Rc::new(update)));
    }
  }
}

impl<A> Clone for ReducerDispatch<A> {
  fn clone(&self) -> Self {
    Self {
      queue: self.queue.clone(),
    }
  }
}

impl<A> PartialEq for ReducerDispatch<A> {
  fn eq(&self, other: &Self) -> bool {
    Weak::ptr_eq(&self.queue, &other.queue)
  }
}

impl<A> Eq for ReducerDispatch<A> {}

impl<A: Clone + 'static> ReducerDispatch<A> {
  /// Queues an action for the next render.
  pub fn send(&self, action: A) {
    if let Some(queue) = self.queue.upgrade() {
      queue.enqueue(action);
    }
  }
}

/// Returns component-local state and its stable setter.
pub fn use_state<T>(initial: T) -> (T, StateSetter<T>)
where
  T: Clone + PartialEq + 'static,
{
  self::use_state_with(|| initial)
}

/// Lazily initializes component-local state and returns its stable setter.
pub fn use_state_with<T>(initial: impl FnOnce() -> T) -> (T, StateSetter<T>)
where
  T: Clone + PartialEq + 'static,
{
  assert!(
    context::hooks_allowed(),
    "Reactant hooks require a component render context"
  );
  let current = CURRENT
    .with(|slot| slot.borrow().clone())
    .expect("Reactant hooks require a component render context");
  let mut attempt = current.borrow_mut();
  let index = attempt.cursor;
  attempt.cursor += 1;
  if index == attempt.component.slots.len() {
    assert!(
      attempt.component.expected_count.is_none(),
      "Reactant hook count changed"
    );
    let value = context::with_hooks_forbidden(initial);
    let queue = Rc::new(StateQueue {
      owner: Rc::downgrade(&attempt.component.owner),
      updates: RefCell::new(Vec::new()),
    });
    attempt.component.slots.push(Box::new(StateSlot {
      committed: value.clone(),
      rendered: value,
      applied: 0,
      queue,
    }));
  }
  let slot = &mut attempt.component.slots[index];
  assert!(slot.kind() == HookKind::State, "Reactant hook kind changed");
  assert!(
    slot.value_type() == TypeId::of::<T>(),
    "Reactant hook type changed"
  );
  let state = slot
    .as_any_mut()
    .downcast_mut::<StateSlot<T>>()
    .expect("validated state hook type");
  state.prepare();
  (
    state.rendered.clone(),
    StateSetter {
      queue: Rc::downgrade(&state.queue),
    },
  )
}

/// Returns reducer-managed state and its stable action dispatcher.
pub fn use_reducer<S, A, F>(reducer: F, initial: S) -> (S, ReducerDispatch<A>)
where
  S: Clone + PartialEq + 'static,
  A: Clone + 'static,
  F: Fn(&S, A) -> S + 'static,
{
  self::use_reducer_with(reducer, || initial)
}

/// Lazily initializes reducer-managed state and returns its stable dispatcher.
pub fn use_reducer_with<S, A, F>(reducer: F, initial: impl FnOnce() -> S) -> (S, ReducerDispatch<A>)
where
  S: Clone + PartialEq + 'static,
  A: Clone + 'static,
  F: Fn(&S, A) -> S + 'static,
{
  assert!(
    context::hooks_allowed(),
    "Reactant hooks require a component render context"
  );
  let current = CURRENT
    .with(|slot| slot.borrow().clone())
    .expect("Reactant hooks require a component render context");
  let mut attempt = current.borrow_mut();
  let index = attempt.cursor;
  attempt.cursor += 1;
  if index == attempt.component.slots.len() {
    assert!(
      attempt.component.expected_count.is_none(),
      "Reactant hook count changed"
    );
    let value = context::with_hooks_forbidden(initial);
    let queue = Rc::new(ReducerQueue {
      owner: Rc::downgrade(&attempt.component.owner),
      actions: RefCell::new(Vec::<A>::new()),
    });
    attempt.component.slots.push(Box::new(ReducerSlot {
      committed: value.clone(),
      rendered: value,
      applied: 0,
      queue,
    }));
  }
  let slot = &mut attempt.component.slots[index];
  assert!(
    slot.kind() == HookKind::Reducer,
    "Reactant hook kind changed"
  );
  assert!(
    slot.value_type() == TypeId::of::<(S, A)>(),
    "Reactant hook type changed"
  );
  let state = slot
    .as_any_mut()
    .downcast_mut::<ReducerSlot<S, A>>()
    .expect("validated reducer hook type");
  state.prepare(&reducer);
  (
    state.rendered.clone(),
    ReducerDispatch {
      queue: Rc::downgrade(&state.queue),
    },
  )
}

#[derive(Clone)]
pub(crate) struct HookOwner {
  mounted: Cell<bool>,
}

pub(crate) struct HookComponent {
  owner: Rc<HookOwner>,
  slots: Vec<Box<dyn HookSlot>>,
  expected_count: Option<usize>,
}

impl Clone for HookComponent {
  fn clone(&self) -> Self {
    Self {
      owner: Rc::clone(&self.owner),
      slots: self.slots.iter().map(|slot| slot.clone_box()).collect(),
      expected_count: self.expected_count,
    }
  }
}

impl HookComponent {
  pub(crate) fn new() -> Self {
    Self {
      owner: Rc::new(HookOwner {
        mounted: Cell::new(false),
      }),
      slots: Vec::new(),
      expected_count: None,
    }
  }

  pub(crate) fn owner(&self) -> Rc<HookOwner> {
    Rc::clone(&self.owner)
  }

  pub(crate) fn has_pending(&self) -> bool {
    self.slots.iter().any(|slot| slot.has_pending())
  }

  pub(crate) fn has_pending_change(&self) -> bool {
    self.slots.iter().any(|slot| slot.has_pending_change())
  }

  pub(crate) fn discard_pending(&mut self) {
    for slot in &mut self.slots {
      slot.discard_pending();
    }
  }

  pub(crate) fn commit(&mut self) {
    for slot in &mut self.slots {
      slot.commit();
    }
    self.owner.mounted.set(true);
  }
}

impl HookOwner {
  pub(crate) fn same(&self, other: &Rc<Self>) -> bool {
    ptr::eq(self, other.as_ref())
  }

  pub(crate) fn unmount(&self) {
    self.mounted.set(false);
  }
}

pub(crate) fn render_component(
  component: HookComponent,
  operation: impl FnOnce(),
) -> (HookComponent, bool) {
  let attempt = Rc::new(RefCell::new(HookAttempt {
    component,
    cursor: 0,
    render_phase_update: false,
  }));
  let previous = CURRENT.with(|slot| slot.replace(Some(Rc::clone(&attempt))));
  let _restore = Restore(previous);
  context::with_component(operation);
  let mut attempt = attempt.borrow_mut();
  if let Some(expected) = attempt.component.expected_count {
    assert_eq!(attempt.cursor, expected, "Reactant hook count changed");
  } else {
    attempt.component.expected_count = Some(attempt.cursor);
  }
  (
    attempt.component.clone(),
    attempt.render_phase_update && attempt.component.has_pending_change(),
  )
}

pub(crate) const fn retry_limit() -> usize {
  RENDER_RETRY_LIMIT
}

struct Restore(Option<Rc<RefCell<HookAttempt>>>);

impl Drop for Restore {
  fn drop(&mut self) {
    CURRENT.with(|slot| slot.replace(self.0.take()));
  }
}

struct HookAttempt {
  component: HookComponent,
  cursor: usize,
  render_phase_update: bool,
}

struct StateQueue<T> {
  owner: Weak<HookOwner>,
  updates: RefCell<Vec<StateUpdate<T>>>,
}

impl<T: Clone + 'static> StateQueue<T> {
  fn enqueue(&self, update: StateUpdate<T>) {
    let Some(owner) = self.owner.upgrade() else {
      return;
    };
    if self::schedule_update(&owner) {
      self.updates.borrow_mut().push(update);
    }
  }
}

struct ReducerQueue<A> {
  owner: Weak<HookOwner>,
  actions: RefCell<Vec<A>>,
}

impl<A: Clone + 'static> ReducerQueue<A> {
  fn enqueue(&self, action: A) {
    let Some(owner) = self.owner.upgrade() else {
      return;
    };
    if self::schedule_update(&owner) {
      self.actions.borrow_mut().push(action);
    }
  }
}

enum StateUpdate<T> {
  Replace(T),
  Update(Rc<dyn Fn(T) -> T>),
}

trait HookSlot {
  fn as_any_mut(&mut self) -> &mut dyn Any;
  fn clone_box(&self) -> Box<dyn HookSlot>;
  fn commit(&mut self);
  fn discard_pending(&mut self);
  fn has_pending(&self) -> bool;
  fn has_pending_change(&self) -> bool;
  fn kind(&self) -> HookKind;
  fn value_type(&self) -> TypeId;
}

struct StateSlot<T> {
  committed: T,
  rendered: T,
  applied: usize,
  queue: Rc<StateQueue<T>>,
}

struct ReducerSlot<S, A> {
  committed: S,
  rendered: S,
  applied: usize,
  queue: Rc<ReducerQueue<A>>,
}

impl<T: Clone + PartialEq + 'static> StateSlot<T> {
  fn prepare(&mut self) {
    let updates = self.queue.updates.borrow();
    self.rendered = updates
      .iter()
      .fold(self.committed.clone(), |value, update| {
        context::with_hooks_forbidden(|| match update {
          StateUpdate::Replace(replacement) => replacement.clone(),
          StateUpdate::Update(update) => update(value),
        })
      });
    self.applied = updates.len();
  }
}

impl<T: Clone + PartialEq + 'static> HookSlot for StateSlot<T> {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      committed: self.committed.clone(),
      rendered: self.rendered.clone(),
      applied: self.applied,
      queue: Rc::clone(&self.queue),
    })
  }

  fn commit(&mut self) {
    self.committed.clone_from(&self.rendered);
    self.queue.updates.borrow_mut().drain(..self.applied);
    self.applied = 0;
  }

  fn discard_pending(&mut self) {
    self.rendered.clone_from(&self.committed);
    self.queue.updates.borrow_mut().clear();
    self.applied = 0;
  }

  fn has_pending(&self) -> bool {
    !self.queue.updates.borrow().is_empty()
  }

  fn has_pending_change(&self) -> bool {
    let updates = self.queue.updates.borrow();
    context::with_hooks_forbidden(|| {
      updates
        .iter()
        .fold(self.committed.clone(), |value, update| match update {
          StateUpdate::Replace(replacement) => replacement.clone(),
          StateUpdate::Update(update) => update(value),
        })
        != self.committed
    })
  }

  fn kind(&self) -> HookKind {
    HookKind::State
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<T>()
  }
}

impl<S, A> ReducerSlot<S, A>
where
  S: Clone + PartialEq + 'static,
  A: Clone + 'static,
{
  fn prepare(&mut self, reducer: &impl Fn(&S, A) -> S) {
    let actions = self.queue.actions.borrow();
    self.rendered = actions
      .iter()
      .fold(self.committed.clone(), |state, action| {
        context::with_hooks_forbidden(|| reducer(&state, action.clone()))
      });
    self.applied = actions.len();
  }
}

impl<S, A> HookSlot for ReducerSlot<S, A>
where
  S: Clone + PartialEq + 'static,
  A: Clone + 'static,
{
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      committed: self.committed.clone(),
      rendered: self.rendered.clone(),
      applied: self.applied,
      queue: Rc::clone(&self.queue),
    })
  }

  fn commit(&mut self) {
    self.committed.clone_from(&self.rendered);
    self.queue.actions.borrow_mut().drain(..self.applied);
    self.applied = 0;
  }

  fn discard_pending(&mut self) {
    self.rendered.clone_from(&self.committed);
    self.queue.actions.borrow_mut().clear();
    self.applied = 0;
  }

  fn has_pending(&self) -> bool {
    !self.queue.actions.borrow().is_empty()
  }

  fn has_pending_change(&self) -> bool {
    let pending = self.queue.actions.borrow().len();
    pending != self.applied || self.rendered != self.committed
  }

  fn kind(&self) -> HookKind {
    HookKind::Reducer
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<(S, A)>()
  }
}

fn schedule_update(owner: &Rc<HookOwner>) -> bool {
  let current = CURRENT.with(|slot| slot.borrow().clone());
  if let Some(current) = current {
    assert!(context::hooks_allowed(), "hook updates are forbidden here");
    let mut attempt = current.borrow_mut();
    assert!(
      attempt.component.owner.same(owner),
      "a component cannot update another component while rendering"
    );
    attempt.render_phase_update = true;
  } else {
    assert!(
      !context::rendering(),
      "a component cannot update hooks while rendering outside its hook context"
    );
    if !owner.mounted.get() {
      return false;
    }
  }
  true
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum HookKind {
  Reducer,
  State,
}
