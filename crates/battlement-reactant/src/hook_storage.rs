use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  ptr,
  rc::{Rc, Weak},
};

use crate::{
  context,
  effect::EffectOperation,
  geometry::GeometryTarget,
  geometry_effect::GeometryEffectOperation,
  geometry_runtime::GeometryRuntime,
  presence::{Presence, PresenceCell, PresenceRenderState},
};

#[derive(Clone)]
pub(crate) struct HookOwner {
  pub(crate) mounted: Cell<bool>,
}

pub(crate) struct HookComponent {
  pub(crate) owner: Rc<HookOwner>,
  pub(crate) slots: Vec<Box<dyn HookSlot>>,
  pub(crate) expected_count: Option<usize>,
}

pub(crate) struct StateQueue<T> {
  pub(crate) owner: Weak<HookOwner>,
  pub(crate) updates: RefCell<Vec<StateUpdate<T>>>,
}

pub(crate) struct ReducerQueue<A> {
  pub(crate) owner: Weak<HookOwner>,
  pub(crate) actions: RefCell<Vec<A>>,
}

pub(crate) struct StateSlot<T> {
  pub(crate) committed: T,
  pub(crate) rendered: T,
  pub(crate) applied: usize,
  pub(crate) queue: Rc<StateQueue<T>>,
}

pub(crate) struct ReducerSlot<S, A> {
  pub(crate) committed: S,
  pub(crate) rendered: S,
  pub(crate) applied: usize,
  pub(crate) queue: Rc<ReducerQueue<A>>,
}

pub(crate) struct RefSlot<T> {
  pub(crate) value: Rc<RefCell<T>>,
}

pub(crate) struct PresenceSlot {
  pub(crate) state: Rc<PresenceCell>,
  pub(crate) manual: bool,
}

pub(crate) struct ContextSlot<T> {
  pub(crate) identity: context::ContextIdentity,
  pub(crate) value: T,
  pub(crate) read: Rc<dyn Fn() -> T>,
}

pub(crate) struct MemoSlot<D, T> {
  pub(crate) committed_dependencies: D,
  pub(crate) committed_value: T,
  pub(crate) rendered_dependencies: D,
  pub(crate) rendered_value: T,
}

pub(crate) enum StateUpdate<T> {
  Replace(T),
  Update(Rc<dyn Fn(T) -> T>),
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum HookKind {
  AnimationControl,
  AnimationScope,
  Context,
  DragControl,
  ElementRef,
  Effect,
  Geometry,
  GeometryEffect,
  Memo,
  MotionValue,
  MotionValueEvent,
  Presence,
  Reducer,
  Ref,
  Resource,
  State,
  Store,
}

pub(crate) trait HookSlot {
  fn as_any_mut(&mut self) -> &mut dyn Any;
  fn clone_box(&self) -> Box<dyn HookSlot>;
  fn commit(&mut self);
  fn discard_pending(&mut self);
  fn has_pending(&self) -> bool;
  fn has_pending_change(&self) -> bool;
  fn context_changed(&self) -> bool;
  fn kind(&self) -> HookKind;
  fn value_type(&self) -> TypeId;

  fn pending_len(&self) -> usize {
    0
  }

  fn truncate_pending(&self, length: usize) {
    assert_eq!(length, 0, "nonqueued hook received a pending checkpoint");
  }

  fn take_effect_operation(&mut self) -> Option<EffectOperation> {
    None
  }

  fn take_unmount_operation(&mut self) -> Option<EffectOperation> {
    None
  }

  fn geometry_effect_model(&self) -> Option<TypeId> {
    None
  }

  fn take_geometry_effect_operation(
    &mut self,
    _runtime: &GeometryRuntime,
  ) -> Option<GeometryEffectOperation> {
    None
  }

  fn take_geometry_unmount_operation(&mut self) -> Option<GeometryEffectOperation> {
    None
  }

  fn stabilize_store(&mut self) -> bool {
    false
  }

  fn freeze_store_wake(&mut self) {}

  fn unmount_store(&mut self) {}

  fn geometry_targets(&self, _targets: &mut Vec<GeometryTarget>) {}

  fn presence_hold(&self) -> Option<Rc<PresenceCell>> {
    None
  }
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

  pub(crate) fn context_changed(&self) -> bool {
    self.slots.iter().any(|slot| slot.context_changed())
  }

  pub(crate) fn discard_pending(&mut self) {
    for slot in &mut self.slots {
      slot.discard_pending();
    }
  }

  pub(crate) fn pending_lengths(&self, lengths: &mut Vec<usize>) {
    lengths.extend(self.slots.iter().map(|slot| slot.pending_len()));
  }

  pub(crate) fn truncate_pending(&self, lengths: &[usize], cursor: &mut usize) {
    for slot in &self.slots {
      slot.truncate_pending(lengths[*cursor]);
      *cursor += 1;
    }
  }

  pub(crate) fn commit(&mut self) {
    for slot in &mut self.slots {
      slot.commit();
    }
    self.owner.mounted.set(true);
  }

  pub(crate) fn take_effect_operations(&mut self, operations: &mut Vec<EffectOperation>) {
    operations.extend(
      self
        .slots
        .iter_mut()
        .filter_map(|slot| slot.take_effect_operation()),
    );
  }

  pub(crate) fn geometry_effect_model_matches(&self, model: TypeId) -> bool {
    self.slots.iter().all(|slot| {
      slot
        .geometry_effect_model()
        .is_none_or(|value| value == model)
    })
  }

  pub(crate) fn take_geometry_effect_operations(
    &mut self,
    runtime: &GeometryRuntime,
    operations: &mut Vec<GeometryEffectOperation>,
  ) {
    operations.extend(
      self
        .slots
        .iter_mut()
        .filter_map(|slot| slot.take_geometry_effect_operation(runtime)),
    );
  }

  pub(crate) fn stabilize_stores(&mut self) -> bool {
    let mut retry = false;
    for slot in &mut self.slots {
      retry |= slot.stabilize_store();
    }
    retry
  }

  pub(crate) fn freeze_store_wakes(&mut self) {
    for slot in &mut self.slots {
      slot.freeze_store_wake();
    }
  }

  pub(crate) fn geometry_targets(&self, targets: &mut Vec<GeometryTarget>) {
    for slot in &self.slots {
      slot.geometry_targets(targets);
    }
  }

  pub(crate) fn presence_holds(&self) -> Vec<Rc<PresenceCell>> {
    self
      .slots
      .iter()
      .filter_map(|slot| slot.presence_hold())
      .collect()
  }

  pub(crate) fn unmount(&mut self, operations: &mut Vec<EffectOperation>) {
    self.owner.unmount();
    for slot in &mut self.slots {
      slot.unmount_store();
    }
    operations.extend(
      self
        .slots
        .iter_mut()
        .filter_map(|slot| slot.take_unmount_operation()),
    );
  }

  pub(crate) fn unmount_geometry_effects(&mut self, operations: &mut Vec<GeometryEffectOperation>) {
    operations.extend(
      self
        .slots
        .iter_mut()
        .filter_map(|slot| slot.take_geometry_unmount_operation()),
    );
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

impl<T: Clone + PartialEq + 'static> StateSlot<T> {
  pub(crate) fn prepare(&mut self) {
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

  fn context_changed(&self) -> bool {
    false
  }

  fn kind(&self) -> HookKind {
    HookKind::State
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<T>()
  }

  fn pending_len(&self) -> usize {
    self.queue.updates.borrow().len()
  }

  fn truncate_pending(&self, length: usize) {
    self.queue.updates.borrow_mut().truncate(length);
  }
}

impl<S, A> ReducerSlot<S, A>
where
  S: Clone + PartialEq + 'static,
  A: Clone + 'static,
{
  pub(crate) fn prepare(&mut self, reducer: &impl Fn(&S, A) -> S) {
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

  fn context_changed(&self) -> bool {
    false
  }

  fn kind(&self) -> HookKind {
    HookKind::Reducer
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<(S, A)>()
  }

  fn pending_len(&self) -> usize {
    self.queue.actions.borrow().len()
  }

  fn truncate_pending(&self, length: usize) {
    self.queue.actions.borrow_mut().truncate(length);
  }
}

impl<T: 'static> HookSlot for RefSlot<T> {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      value: Rc::clone(&self.value),
    })
  }

  fn commit(&mut self) {}

  fn discard_pending(&mut self) {}

  fn has_pending(&self) -> bool {
    false
  }

  fn has_pending_change(&self) -> bool {
    false
  }

  fn context_changed(&self) -> bool {
    false
  }

  fn kind(&self) -> HookKind {
    HookKind::Ref
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<T>()
  }
}

impl PresenceSlot {
  pub(crate) fn prepare(&mut self, state: PresenceRenderState, manual: bool) {
    assert_eq!(
      self.manual, manual,
      "Reactant presence hook kind changed between manual and observed"
    );
    self.state.prepare(state);
  }
}

impl HookSlot for PresenceSlot {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      state: Rc::clone(&self.state),
      manual: self.manual,
    })
  }

  fn commit(&mut self) {
    self.state.clear_dirty();
  }

  fn discard_pending(&mut self) {
    self.state.clear_dirty();
  }

  fn has_pending(&self) -> bool {
    self.state.dirty()
  }

  fn has_pending_change(&self) -> bool {
    self.state.dirty()
  }

  fn context_changed(&self) -> bool {
    false
  }

  fn kind(&self) -> HookKind {
    HookKind::Presence
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<Presence>()
  }

  fn presence_hold(&self) -> Option<Rc<PresenceCell>> {
    self.manual.then(|| Rc::clone(&self.state))
  }
}

impl<T: Clone + PartialEq + 'static> HookSlot for ContextSlot<T> {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      identity: self.identity,
      value: self.value.clone(),
      read: Rc::clone(&self.read),
    })
  }

  fn commit(&mut self) {}

  fn discard_pending(&mut self) {}

  fn has_pending(&self) -> bool {
    false
  }

  fn has_pending_change(&self) -> bool {
    false
  }

  fn context_changed(&self) -> bool {
    context::with_hooks_forbidden(|| (self.read)() != self.value)
  }

  fn kind(&self) -> HookKind {
    HookKind::Context
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<T>()
  }
}

impl<D, T> HookSlot for MemoSlot<D, T>
where
  D: Clone + PartialEq + 'static,
  T: Clone + 'static,
{
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      committed_dependencies: self.committed_dependencies.clone(),
      committed_value: self.committed_value.clone(),
      rendered_dependencies: self.rendered_dependencies.clone(),
      rendered_value: self.rendered_value.clone(),
    })
  }

  fn commit(&mut self) {
    self
      .committed_dependencies
      .clone_from(&self.rendered_dependencies);
    self.committed_value.clone_from(&self.rendered_value);
  }

  fn discard_pending(&mut self) {
    self
      .rendered_dependencies
      .clone_from(&self.committed_dependencies);
    self.rendered_value.clone_from(&self.committed_value);
  }

  fn has_pending(&self) -> bool {
    false
  }

  fn has_pending_change(&self) -> bool {
    false
  }

  fn context_changed(&self) -> bool {
    false
  }

  fn kind(&self) -> HookKind {
    HookKind::Memo
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<(D, T)>()
  }
}
