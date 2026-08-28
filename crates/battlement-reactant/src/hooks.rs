//! Positional component state hooks.

use std::{
  any::TypeId,
  cell::RefCell,
  ops::Deref,
  rc::{Rc, Weak},
};

use crate::context;
use crate::context::{Context, ContextIdentity, RequiredContext};
use crate::effect::{EffectCleanup, EffectSetup, EffectSlot};
use crate::hook_storage::{
  ContextSlot, HookComponent, HookKind, HookOwner, MemoSlot, ReducerQueue, ReducerSlot, RefSlot,
  StateQueue, StateSlot, StateUpdate,
};

const RENDER_RETRY_LIMIT: usize = 25;

thread_local! {
  static CURRENT: RefCell<Option<Rc<RefCell<HookAttempt>>>> = const { RefCell::new(None) };
}

/// Marks a cloneable equality-comparable dependency list.
pub trait Dependencies: Clone + PartialEq + 'static {}

impl<T> Dependencies for T where T: Clone + PartialEq + 'static {}

/// Converts an effect setup result into an optional cleanup.
pub trait IntoEffectCleanup: effect_cleanup::Sealed + 'static {
  /// Returns the cleanup that should run before replacement or unmount.
  fn into_cleanup(self) -> Option<Box<dyn FnOnce()>>;
}

/// Queues state replacements and updater functions for one mounted hook.
pub struct StateSetter<T> {
  queue: Weak<StateQueue<T>>,
}

/// Queues actions for one mounted reducer hook.
pub struct ReducerDispatch<A> {
  queue: Weak<ReducerQueue<A>>,
}

/// Holds stable mutable data without scheduling renders.
pub struct Ref<T> {
  value: Rc<RefCell<T>>,
}

/// Holds a callback with stable identity while its dependencies are equal.
pub struct Callback<F> {
  callback: Rc<F>,
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

impl<T> Clone for Ref<T> {
  fn clone(&self) -> Self {
    Self {
      value: Rc::clone(&self.value),
    }
  }
}

impl<T> PartialEq for Ref<T> {
  fn eq(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.value, &other.value)
  }
}

impl<T> Eq for Ref<T> {}

impl<T: 'static> Ref<T> {
  /// Clones the current value.
  pub fn get(&self) -> T
  where
    T: Clone,
  {
    self.with(Clone::clone)
  }

  /// Replaces and returns the current value.
  pub fn replace(&self, value: T) -> T {
    assert!(
      !context::rendering(),
      "Reactant refs cannot be accessed while rendering"
    );
    self.value.replace(value)
  }

  /// Reads the current value without exposing a borrow guard.
  pub fn with<R>(&self, read: impl FnOnce(&T) -> R) -> R {
    assert!(
      !context::rendering(),
      "Reactant refs cannot be accessed while rendering"
    );
    read(&self.value.borrow())
  }

  /// Mutates the current value without exposing a borrow guard.
  pub fn with_mut<R>(&self, write: impl FnOnce(&mut T) -> R) -> R {
    assert!(
      !context::rendering(),
      "Reactant refs cannot be accessed while rendering"
    );
    write(&mut self.value.borrow_mut())
  }
}

impl<F> Clone for Callback<F> {
  fn clone(&self) -> Self {
    Self {
      callback: Rc::clone(&self.callback),
    }
  }
}

impl<F> PartialEq for Callback<F> {
  fn eq(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.callback, &other.callback)
  }
}

impl<F> Eq for Callback<F> {}

impl<F> Deref for Callback<F> {
  type Target = F;

  fn deref(&self) -> &Self::Target {
    &self.callback
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

/// Memoizes a calculated value until its dependencies differ.
pub fn use_memo<D, T>(calculate: impl FnOnce() -> T, dependencies: D) -> T
where
  D: Dependencies,
  T: Clone + 'static,
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
    let value = context::with_hooks_forbidden(calculate);
    attempt.component.slots.push(Box::new(MemoSlot {
      committed_dependencies: dependencies.clone(),
      committed_value: value.clone(),
      rendered_dependencies: dependencies,
      rendered_value: value.clone(),
    }));
    return value;
  }
  let slot = &mut attempt.component.slots[index];
  assert!(slot.kind() == HookKind::Memo, "Reactant hook kind changed");
  assert!(
    slot.value_type() == TypeId::of::<(D, T)>(),
    "Reactant hook type changed"
  );
  let memo = slot
    .as_any_mut()
    .downcast_mut::<MemoSlot<D, T>>()
    .expect("validated memo hook type");
  memo.rendered_value = if memo.committed_dependencies == dependencies {
    memo.committed_value.clone()
  } else {
    context::with_hooks_forbidden(calculate)
  };
  memo.rendered_dependencies = dependencies;
  memo.rendered_value.clone()
}

/// Memoizes a callback until its dependencies differ.
pub fn use_callback<D, F>(callback: F, dependencies: D) -> Callback<F>
where
  D: Dependencies,
  F: 'static,
{
  self::use_memo(
    || Callback {
      callback: Rc::new(callback),
    },
    dependencies,
  )
}

/// Queues an effect after a commit when its dependencies change.
pub fn use_effect<D, S, C>(setup: S, dependencies: D)
where
  D: Dependencies,
  S: FnOnce() -> C + 'static,
  C: IntoEffectCleanup,
{
  self::use_effect_hook(setup, dependencies, false);
}

/// Queues an effect after every commit of its component.
pub fn use_effect_always<S, C>(setup: S)
where
  S: FnOnce() -> C + 'static,
  C: IntoEffectCleanup,
{
  self::use_effect_hook(setup, (), true);
}

/// Returns one stable mutable ref for this mounted hook slot.
pub fn use_ref<T: 'static>(initial: T) -> Ref<T> {
  self::use_ref_with(|| initial)
}

/// Lazily initializes one stable mutable ref for this mounted hook slot.
pub fn use_ref_with<T: 'static>(initial: impl FnOnce() -> T) -> Ref<T> {
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
    attempt.component.slots.push(Box::new(RefSlot {
      value: Rc::new(RefCell::new(context::with_hooks_forbidden(initial))),
    }));
  }
  let slot = &mut attempt.component.slots[index];
  assert!(slot.kind() == HookKind::Ref, "Reactant hook kind changed");
  assert!(
    slot.value_type() == TypeId::of::<T>(),
    "Reactant hook type changed"
  );
  Ref {
    value: Rc::clone(
      &slot
        .as_any_mut()
        .downcast_mut::<RefSlot<T>>()
        .expect("validated ref hook type")
        .value,
    ),
  }
}

/// Returns the nearest provider value or this runtime's stored default.
pub fn use_context<T>(source: &'static Context<T>) -> T
where
  T: Clone + PartialEq + 'static,
{
  self::use_context_value(source.identity(), move || source.read())
}

/// Returns the nearest provider value or panics when none exists.
pub fn use_required_context<T>(source: &'static RequiredContext<T>) -> T
where
  T: Clone + PartialEq + 'static,
{
  self::use_context_value(source.identity(), move || source.read())
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
  let restore = Restore(previous);
  context::with_component(operation);
  drop(restore);
  let mut attempt = Rc::try_unwrap(attempt)
    .unwrap_or_else(|_| panic!("Reactant hook render context escaped its component"))
    .into_inner();
  if let Some(expected) = attempt.component.expected_count {
    assert_eq!(attempt.cursor, expected, "Reactant hook count changed");
  } else {
    attempt.component.expected_count = Some(attempt.cursor);
  }
  let retry = attempt.render_phase_update && attempt.component.has_pending_change();
  (attempt.component, retry)
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

fn use_context_value<T>(identity: ContextIdentity, read: impl Fn() -> T + 'static) -> T
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
  let read: Rc<dyn Fn() -> T> = Rc::new(read);
  let value = read();
  if index == attempt.component.slots.len() {
    assert!(
      attempt.component.expected_count.is_none(),
      "Reactant hook count changed"
    );
    attempt.component.slots.push(Box::new(ContextSlot {
      identity,
      value: value.clone(),
      read: Rc::clone(&read),
    }));
  }
  let slot = &mut attempt.component.slots[index];
  assert!(
    slot.kind() == HookKind::Context,
    "Reactant hook kind changed"
  );
  assert!(
    slot.value_type() == TypeId::of::<T>(),
    "Reactant hook type changed"
  );
  let context = slot
    .as_any_mut()
    .downcast_mut::<ContextSlot<T>>()
    .expect("validated context hook type");
  assert!(
    context.identity == identity,
    "Reactant context identity changed"
  );
  context.value = value;
  context.read = read;
  context.value.clone()
}

fn use_effect_hook<D, S, C>(setup: S, dependencies: D, always: bool)
where
  D: Dependencies,
  S: FnOnce() -> C + 'static,
  C: IntoEffectCleanup,
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
  let setup: EffectSetup = Box::new(move || setup().into_cleanup());
  let value_type = TypeId::of::<(D, C)>();
  if index == attempt.component.slots.len() {
    assert!(
      attempt.component.expected_count.is_none(),
      "Reactant hook count changed"
    );
    attempt.component.slots.push(Box::new(EffectSlot::new(
      dependencies,
      setup,
      value_type,
      always,
    )));
    return;
  }
  let slot = &mut attempt.component.slots[index];
  assert!(
    slot.kind() == HookKind::Effect,
    "Reactant hook kind changed"
  );
  assert!(
    slot.value_type() == value_type,
    "Reactant hook type changed"
  );
  let effect = slot
    .as_any_mut()
    .downcast_mut::<EffectSlot<D>>()
    .expect("validated effect hook type");
  assert!(
    effect.always == always,
    "Reactant effect dependency mode changed"
  );
  effect.prepare(dependencies, setup);
}

impl IntoEffectCleanup for () {
  fn into_cleanup(self) -> Option<EffectCleanup> {
    None
  }
}

impl<F> IntoEffectCleanup for F
where
  F: FnOnce() + 'static,
{
  fn into_cleanup(self) -> Option<EffectCleanup> {
    Some(Box::new(self))
  }
}

mod effect_cleanup {
  pub trait Sealed {}

  impl Sealed for () {}

  impl<F> Sealed for F where F: FnOnce() + 'static {}
}
