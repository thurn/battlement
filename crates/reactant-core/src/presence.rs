//! Terminal visual retention after logical component destruction.

use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  rc::{Rc, Weak},
};

use battlement::{MotionEventKind, MotionGeneration, MotionLifecycleEvent, MotionSlotId, ObjectId};

use crate::{
  hook_storage::HookOwner,
  key::ErasedKey,
  render::{Render, RenderSink},
  render_value::Sealed,
  retained_visual::RetainedResources,
  variant_map::{ErasedVariantData, VariantData},
};

type PresenceCallback = dyn Fn(&mut dyn Any);
pub(crate) type PresenceHold = (Rc<PresenceCell>, Rc<HookOwner>);

thread_local! {
  static CURRENT: RefCell<PresenceRenderState> = const {
    RefCell::new(PresenceRenderState { present: true, generation: 0 })
  };
}

/// Determines how entering children interact with retained exits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PresenceMode {
  /// Entering and exiting children coexist.
  #[default]
  Sync,
  /// Defers one entering child until every current exit completes.
  Wait,
  /// Reserves projection-backed removal for the layout system.
  PopLayout,
}

/// Retains keyed native visuals after their logical children unmount.
pub struct AnimatePresence<R = ()> {
  child: R,
  initial: bool,
  mode: PresenceMode,
  custom: Option<ErasedVariantData>,
  on_exit_complete: Option<PresenceHandler>,
}

/// A logical-lifetime handle that can release an externally held exit.
pub struct Presence {
  pub(crate) state: Rc<PresenceCell>,
}

/// An explicit shared use of a terminal visual and its native resource descriptions.
pub struct RetainedVisual {
  state: Rc<PresenceCell>,
}

#[derive(Clone, Copy)]
pub(crate) struct PresenceRenderState {
  pub(crate) present: bool,
  pub(crate) generation: u64,
}

pub(crate) struct PresenceCell {
  present: Cell<bool>,
  generation: Cell<u64>,
  released: Cell<Option<u64>>,
  dirty: Cell<bool>,
  observers: Cell<usize>,
  retained: Cell<usize>,
  explicit: Cell<bool>,
  resources: RefCell<Weak<RetainedResources>>,
}

#[derive(Clone)]
pub(crate) struct PresenceHandler {
  model: TypeId,
  callback: Rc<PresenceCallback>,
}

#[derive(Clone)]
pub(crate) struct PresenceConfig {
  pub(crate) initial: bool,
  pub(crate) mode: PresenceMode,
  pub(crate) custom: Option<ErasedVariantData>,
  pub(crate) on_exit_complete: Option<PresenceHandler>,
}

#[derive(Clone)]
pub(crate) struct PresenceBoundaryState {
  pub(crate) generation: u64,
  pub(crate) exits: Vec<PresenceExit>,
  pub(crate) notified: bool,
  pub(crate) handler: Option<PresenceHandler>,
}

#[derive(Clone)]
pub(crate) struct PresenceExit {
  pub(crate) key: ErasedKey,
  pub(crate) generation: u64,
  pub(crate) automatic: Vec<AutomaticExit>,
  pub(crate) holds: Vec<PresenceHold>,
  pub(crate) resources: Rc<RetainedResources>,
}

#[derive(Clone)]
pub(crate) struct AutomaticExit {
  pub(crate) descriptor_id: ObjectId,
  slot: MotionSlotId,
  generation: MotionGeneration,
  terminal: bool,
}

pub(crate) struct PresenceMarker;

impl AnimatePresence<()> {
  /// Creates an empty synchronous presence boundary.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      child: (),
      initial: true,
      mode: PresenceMode::Sync,
      custom: None,
      on_exit_complete: None,
    }
  }
}

impl<R> AnimatePresence<R> {
  /// Replaces the boundary's logical child output.
  #[must_use]
  pub fn child<C>(self, child: C) -> AnimatePresence<C> {
    AnimatePresence {
      child,
      initial: self.initial,
      mode: self.mode,
      custom: self.custom,
      on_exit_complete: self.on_exit_complete,
    }
  }

  /// Enables or suppresses initial animation for the first committed output.
  #[must_use]
  pub const fn initial(mut self, value: bool) -> Self {
    self.initial = value;
    self
  }

  /// Selects entering and exiting child coordination.
  #[must_use]
  pub fn mode(mut self, value: PresenceMode) -> Self {
    self.mode = value;
    self
  }

  /// Supplies custom data snapshotted when a child begins exiting.
  #[must_use]
  pub fn custom<T: VariantData>(mut self, value: T) -> Self {
    self.custom = Some(ErasedVariantData::new(value));
    self
  }

  /// Runs after every child in one exit wave becomes removable.
  #[must_use]
  pub fn on_exit_complete<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.on_exit_complete = Some(PresenceHandler::new(callback));
    self
  }

  fn config(&self) -> PresenceConfig {
    PresenceConfig {
      initial: self.initial,
      mode: self.mode,
      custom: self.custom.clone(),
      on_exit_complete: self.on_exit_complete.clone(),
    }
  }
}

impl<R: Render> Render for AnimatePresence<R> {}

#[allow(private_interfaces)]
impl<R: Render> Sealed for AnimatePresence<R> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<PresenceMarker>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_presence::<PresenceMarker>(self.config(), |children| {
      self.child.render_into(children);
    });
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    let config = self.config();
    sink.push_presence::<PresenceMarker>(config, |children| {
      self.child.render_owned(children);
    });
  }
}

impl Default for AnimatePresence<()> {
  fn default() -> Self {
    Self::new()
  }
}

impl Presence {
  pub(crate) fn new(state: Rc<PresenceCell>, _generation: u64) -> Self {
    state.observers.set(state.observers.get() + 1);
    Self { state }
  }

  /// Reports whether the component belongs to the current output.
  #[must_use]
  pub fn is_present(&self) -> bool {
    self.state.present.get()
  }

  /// Retains the prepared native visual after this component is destroyed.
  /// The last cloned handle releases the hold; it does not keep hooks alive.
  pub fn retain_visual(&self) -> RetainedVisual {
    assert!(
      !crate::context::rendering(),
      "retain visuals after committing their component"
    );
    assert!(
      self.is_present(),
      "retain a visual before logical destruction"
    );
    self.state.explicit.set(true);
    self.state.retained.set(self.state.retained.get() + 1);
    RetainedVisual {
      state: Rc::clone(&self.state),
    }
  }

  /// Releases this logical lifetime's legacy manual exit hold.
  pub fn safe_to_remove(&self) {
    if self.state.present.get() {
      return;
    }
    let generation = self.state.generation.get();
    if self.state.released.replace(Some(generation)) != Some(generation) {
      self.state.dirty.set(true);
    }
  }
}

impl PresenceCell {
  pub(crate) fn new(state: PresenceRenderState) -> Rc<Self> {
    Rc::new(Self {
      present: Cell::new(state.present),
      generation: Cell::new(state.generation),
      released: Cell::new(state.present.then_some(state.generation)),
      dirty: Cell::new(false),
      observers: Cell::new(0),
      retained: Cell::new(0),
      explicit: Cell::new(false),
      resources: RefCell::new(Weak::new()),
    })
  }

  pub(crate) fn prepare(&self, state: PresenceRenderState) {
    self.present.set(state.present);
    self.generation.set(state.generation);
    if state.present {
      self.released.set(Some(state.generation));
    } else if self.released.get() != Some(state.generation) {
      self.released.set(None);
    }
  }

  pub(crate) fn ready(&self, generation: u64) -> bool {
    let generation = if self.present.get() {
      generation
    } else {
      self.generation.get()
    };
    let manual_released = self.released.get() == Some(generation) || self.observers.get() == 0;
    let released = self.explicit.get() || manual_released;
    released && self.retained.get() == 0
  }

  pub(crate) fn begin_exit(&self, generation: u64, resources: Rc<RetainedResources>) {
    if self.present.get() {
      self.prepare(PresenceRenderState {
        present: false,
        generation,
      });
    }
    self.resources.replace(Rc::downgrade(&resources));
  }

  pub(crate) fn release_resources(&self) {
    self.resources.replace(Weak::new());
  }

  pub(crate) fn dirty(&self) -> bool {
    self.dirty.get()
  }

  pub(crate) fn clear_dirty(&self) {
    self.dirty.set(false);
  }
}

impl PresenceHandler {
  fn new<G: 'static>(callback: impl Fn(&mut G) + 'static) -> Self {
    Self {
      model: TypeId::of::<G>(),
      callback: Rc::new(move |game| {
        callback(
          game
            .downcast_mut::<G>()
            .expect("presence callback model type was not validated"),
        );
      }),
    }
  }

  pub(crate) fn model(&self) -> TypeId {
    self.model
  }

  pub(crate) fn invoke(&self, game: &mut dyn Any) {
    (self.callback)(game);
  }
}

impl PresenceBoundaryState {
  pub(crate) fn new(generation: u64, handler: Option<PresenceHandler>) -> Self {
    Self {
      generation,
      exits: Vec::new(),
      notified: false,
      handler,
    }
  }

  pub(crate) fn apply(&mut self, event: &MotionLifecycleEvent) -> bool {
    let mut changed = false;
    for exit in &mut self.exits {
      for automatic in &mut exit.automatic {
        let matches = automatic.descriptor_id == event.descriptor_id
          && automatic.slot == event.slot
          && automatic.generation == event.generation;
        let terminal = matches!(
          event.kind,
          MotionEventKind::Completed | MotionEventKind::Stopped | MotionEventKind::Cancelled
        );
        if matches && terminal && !automatic.terminal {
          automatic.terminal = true;
          changed = true;
        }
      }
    }
    changed
  }

  pub(crate) fn ready(&self) -> bool {
    !self.exits.is_empty() && self.exits.iter().all(PresenceExit::ready)
  }
}

impl PresenceExit {
  pub(crate) fn ready(&self) -> bool {
    self.automatic.iter().all(|value| value.terminal)
      && self
        .holds
        .iter()
        .all(|(value, _)| value.ready(self.generation))
  }
}

impl AutomaticExit {
  pub(crate) const fn new(
    descriptor_id: ObjectId,
    slot: MotionSlotId,
    generation: MotionGeneration,
  ) -> Self {
    Self {
      descriptor_id,
      slot,
      generation,
      terminal: false,
    }
  }
}

pub(crate) fn current() -> PresenceRenderState {
  CURRENT.with(|value| *value.borrow())
}

pub(crate) fn with_state<R>(state: PresenceRenderState, render: impl FnOnce() -> R) -> R {
  CURRENT.with(|current| {
    let previous = current.replace(state);
    let result = render();
    current.replace(previous);
    result
  })
}

impl RetainedVisual {
  /// Native handles owned by this retained exit, independent of logical refs.
  pub fn native_objects(&self) -> Vec<ObjectId> {
    self
      .state
      .resources
      .borrow()
      .upgrade()
      .map_or_else(Vec::new, |resources| {
        resources.hosts.iter().map(|host| host.object_id).collect()
      })
  }
}

impl Clone for RetainedVisual {
  fn clone(&self) -> Self {
    self.state.retained.set(self.state.retained.get() + 1);
    Self {
      state: Rc::clone(&self.state),
    }
  }
}

impl Drop for RetainedVisual {
  fn drop(&mut self) {
    self.state.retained.set(self.state.retained.get() - 1);
    self.state.dirty.set(true);
  }
}

impl Clone for Presence {
  fn clone(&self) -> Self {
    Self::new(Rc::clone(&self.state), self.state.generation.get())
  }
}

impl Drop for Presence {
  fn drop(&mut self) {
    let remaining = self.state.observers.get() - 1;
    self.state.observers.set(remaining);
    if remaining == 0 && !self.state.present.get() {
      self.state.dirty.set(true);
    }
  }
}
