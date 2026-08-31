//! Logical presence retention and manual removal holds.

use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  rc::Rc,
};

use battlement::{MotionEventKind, MotionGeneration, MotionLifecycleEvent, MotionSlotId, ObjectId};

use crate::{
  key::ErasedKey,
  render::{Render, RenderSink},
  render_value::Sealed,
  variant_map::{ErasedVariantData, VariantData},
};

type PresenceCallback = dyn Fn(&mut dyn Any);

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

/// Retains keyed logical children while their exit work completes.
pub struct AnimatePresence<R = ()> {
  child: R,
  initial: bool,
  mode: PresenceMode,
  custom: Option<ErasedVariantData>,
  on_exit_complete: Option<PresenceHandler>,
}

/// Stable component-local access to the nearest presence boundary.
#[derive(Clone)]
pub struct Presence {
  pub(crate) state: Rc<PresenceCell>,
  generation: u64,
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
  pub(crate) holds: Vec<Rc<PresenceCell>>,
}

#[derive(Clone)]
pub(crate) struct AutomaticExit {
  descriptor_id: ObjectId,
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
    assert!(
      value != PresenceMode::PopLayout,
      "PresenceMode::PopLayout requires layout projection"
    );
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
  pub(crate) fn new(state: Rc<PresenceCell>, generation: u64) -> Self {
    Self { state, generation }
  }

  /// Reports whether the component belongs to the current output.
  #[must_use]
  pub fn is_present(&self) -> bool {
    self.state.present.get()
  }

  /// Releases this component's manual hold for the observed exit generation.
  pub fn safe_to_remove(&self) {
    if self.state.present.get() || self.state.generation.get() != self.generation {
      return;
    }
    if self.state.released.replace(Some(self.generation)) != Some(self.generation) {
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
    self.released.get() == Some(generation)
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
      && self.holds.iter().all(|value| value.ready(self.generation))
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
