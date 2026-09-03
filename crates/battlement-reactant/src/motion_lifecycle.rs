//! Rust callback storage for native Motion lifecycle boundaries.

use std::{
  any::{Any, TypeId},
  fmt,
  rc::Rc,
};

use battlement::{
  MotionCallbackSubscriptions, MotionDescriptor, MotionEventKind, MotionGeneration,
  MotionGestureEvent, MotionGestureEventKind, MotionGestureSubscriptions, MotionLifecycleEvent,
  MotionPresentationSample, MotionSlotId, ObjectId,
};

use crate::motion::{MotionProps, MotionTarget};

type LifecycleCallback = dyn Fn(&mut dyn Any, &MotionLifecycleEvent);
type UpdateCallback = dyn Fn(&mut dyn Any, &MotionPresentationSample);
type GestureCallback = dyn Fn(&mut dyn Any, &MotionGestureEvent);

#[derive(Clone, Copy)]
pub(crate) enum MotionCallbackKind {
  Start,
  Repeat,
  Complete,
  Stop,
  Cancel,
}

#[derive(Clone, Default)]
pub(crate) struct MotionCallbacks {
  start: Option<MotionHandler>,
  update: Option<MotionUpdateHandler>,
  repeat: Option<MotionHandler>,
  complete: Option<MotionHandler>,
  stop: Option<MotionHandler>,
  cancel: Option<MotionHandler>,
  gestures: [Option<GestureHandler>; 24],
  gesture_observers: [Option<Rc<dyn Fn()>>; 24],
}

#[derive(Clone)]
pub(crate) struct MotionCallbackRegistration {
  descriptor_id: ObjectId,
  slot: MotionSlotId,
  generation: MotionGeneration,
  callbacks: MotionCallbacks,
}

#[derive(Clone)]
struct MotionHandler {
  model: TypeId,
  callback: Rc<LifecycleCallback>,
}

#[derive(Clone)]
struct MotionUpdateHandler {
  model: TypeId,
  callback: Rc<UpdateCallback>,
}

#[derive(Clone)]
struct GestureHandler {
  model: TypeId,
  callback: Rc<GestureCallback>,
}

impl MotionTarget {
  /// Runs when this slot leaves its delay.
  #[must_use]
  pub fn on_start<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.brief(MotionCallbackKind::Start, callback);
    self
  }

  /// Runs with the native boundary when this slot leaves its delay.
  #[must_use]
  pub fn on_start_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.event(MotionCallbackKind::Start, callback);
    self
  }

  /// Runs for this slot's coalesced rendered-frame samples.
  #[must_use]
  pub fn on_update<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.update_brief(callback);
    self
  }

  /// Runs with each coalesced rendered-frame sample from this slot.
  #[must_use]
  pub fn on_update_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionPresentationSample) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.update(callback);
    self
  }

  /// Runs when this slot crosses a repeat boundary.
  #[must_use]
  pub fn on_repeat<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.brief(MotionCallbackKind::Repeat, callback);
    self
  }

  /// Runs with the native boundary when this slot repeats.
  #[must_use]
  pub fn on_repeat_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.event(MotionCallbackKind::Repeat, callback);
    self
  }

  /// Runs after this finite slot completes successfully.
  #[must_use]
  pub fn on_complete<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.brief(MotionCallbackKind::Complete, callback);
    self
  }

  /// Runs with the native boundary after successful completion.
  #[must_use]
  pub fn on_complete_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.event(MotionCallbackKind::Complete, callback);
    self
  }

  /// Runs if imperative playback stops this slot.
  #[must_use]
  pub fn on_stop<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.brief(MotionCallbackKind::Stop, callback);
    self
  }

  /// Runs with the native boundary if imperative playback stops this slot.
  #[must_use]
  pub fn on_stop_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.event(MotionCallbackKind::Stop, callback);
    self
  }

  /// Runs when this slot is cancelled or superseded.
  #[must_use]
  pub fn on_cancel<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.brief(MotionCallbackKind::Cancel, callback);
    self
  }

  /// Runs with the native boundary when this slot is cancelled.
  #[must_use]
  pub fn on_cancel_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.event(MotionCallbackKind::Cancel, callback);
    self
  }
}

impl MotionProps {
  /// Adds a host-direct start callback.
  #[must_use]
  pub fn on_start<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.brief(MotionCallbackKind::Start, callback);
    self
  }

  /// Adds a host-direct start callback with its native boundary.
  #[must_use]
  pub fn on_start_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.event(MotionCallbackKind::Start, callback);
    self
  }

  /// Adds a host-direct rendered-frame update callback.
  #[must_use]
  pub fn on_update<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.update_brief(callback);
    self
  }

  /// Adds a host-direct callback with each coalesced presentation sample.
  #[must_use]
  pub fn on_update_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionPresentationSample) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.update(callback);
    self
  }

  /// Adds a host-direct repeat callback.
  #[must_use]
  pub fn on_repeat<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.brief(MotionCallbackKind::Repeat, callback);
    self
  }

  /// Adds a host-direct repeat callback with its native boundary.
  #[must_use]
  pub fn on_repeat_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.event(MotionCallbackKind::Repeat, callback);
    self
  }

  /// Adds a host-direct completion callback.
  #[must_use]
  pub fn on_complete<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.brief(MotionCallbackKind::Complete, callback);
    self
  }

  /// Adds a host-direct completion callback with its native boundary.
  #[must_use]
  pub fn on_complete_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.event(MotionCallbackKind::Complete, callback);
    self
  }

  /// Adds a host-direct stop callback.
  #[must_use]
  pub fn on_stop<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.brief(MotionCallbackKind::Stop, callback);
    self
  }

  /// Adds a host-direct stop callback with its native boundary.
  #[must_use]
  pub fn on_stop_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.event(MotionCallbackKind::Stop, callback);
    self
  }

  /// Adds a host-direct cancellation callback.
  #[must_use]
  pub fn on_cancel<G: 'static>(mut self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.callbacks = self.callbacks.brief(MotionCallbackKind::Cancel, callback);
    self
  }

  /// Adds a host-direct cancellation callback with its native boundary.
  #[must_use]
  pub fn on_cancel_event<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.event(MotionCallbackKind::Cancel, callback);
    self
  }
}

impl MotionCallbacks {
  pub(crate) const fn new() -> Self {
    Self {
      start: None,
      update: None,
      repeat: None,
      complete: None,
      stop: None,
      cancel: None,
      gestures: [const { None }; 24],
      gesture_observers: [const { None }; 24],
    }
  }

  pub(crate) fn brief<G: 'static>(
    mut self,
    kind: MotionCallbackKind,
    callback: impl Fn(&mut G) + 'static,
  ) -> Self {
    self.set(
      kind,
      MotionHandler::new(move |game: &mut G, _event| callback(game)),
    );
    self
  }

  pub(crate) fn event<G: 'static>(
    mut self,
    kind: MotionCallbackKind,
    callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static,
  ) -> Self {
    self.set(kind, MotionHandler::new(callback));
    self
  }

  pub(crate) fn update<G: 'static>(
    mut self,
    callback: impl Fn(&mut G, &MotionPresentationSample) + 'static,
  ) -> Self {
    self.update = Some(MotionUpdateHandler::new(callback));
    self
  }

  pub(crate) fn update_brief<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> Self {
    self.update(move |game, _sample| callback(game))
  }

  pub(crate) fn merge(mut self, value: Self) -> Self {
    if value.start.is_some() {
      self.start = value.start;
    }
    if value.repeat.is_some() {
      self.repeat = value.repeat;
    }
    if value.update.is_some() {
      self.update = value.update;
    }
    if value.complete.is_some() {
      self.complete = value.complete;
    }
    if value.stop.is_some() {
      self.stop = value.stop;
    }
    if value.cancel.is_some() {
      self.cancel = value.cancel;
    }
    for (index, handler) in value.gestures.into_iter().enumerate() {
      if handler.is_some() {
        self.gestures[index] = handler;
      }
    }
    for (index, observer) in value.gesture_observers.into_iter().enumerate() {
      if observer.is_some() {
        self.gesture_observers[index] = observer;
      }
    }
    self
  }

  pub(crate) fn gesture_event<G: 'static>(
    mut self,
    kind: MotionGestureEventKind,
    callback: impl Fn(&mut G, &MotionGestureEvent) + 'static,
  ) -> Self {
    self.gestures[gesture_index(kind)] = Some(GestureHandler::new(callback));
    self
  }

  pub(crate) fn gesture_brief(
    mut self,
    kind: MotionGestureEventKind,
    callback: impl Fn() + 'static,
  ) -> Self {
    self.gesture_observers[gesture_index(kind)] = Some(Rc::new(callback));
    self
  }

  pub(crate) fn subscriptions(&self) -> MotionCallbackSubscriptions {
    MotionCallbackSubscriptions {
      start: self.start.is_some(),
      update: self.update.is_some(),
      repeat: self.repeat.is_some(),
      complete: self.complete.is_some(),
      stop: self.stop.is_some(),
      cancel: self.cancel.is_some(),
    }
  }

  pub(crate) fn gesture_subscriptions(&self) -> MotionGestureSubscriptions {
    MotionGestureSubscriptions {
      hover: self.has_any_gesture(&[
        MotionGestureEventKind::HoverStart,
        MotionGestureEventKind::HoverEnd,
      ]),
      tap: self.has_any_gesture(&[
        MotionGestureEventKind::TapStart,
        MotionGestureEventKind::Tap,
        MotionGestureEventKind::TapCancel,
      ]),
      focus: self.has_any_gesture(&[
        MotionGestureEventKind::FocusStart,
        MotionGestureEventKind::FocusEnd,
      ]),
      focus_visible: self.has_any_gesture(&[
        MotionGestureEventKind::FocusVisibleStart,
        MotionGestureEventKind::FocusVisibleEnd,
      ]),
      pan: self.has_any_gesture(&[
        MotionGestureEventKind::PanSessionStart,
        MotionGestureEventKind::PanStart,
        MotionGestureEventKind::PanEnd,
        MotionGestureEventKind::PanCancel,
      ]),
      pan_update: self.has_any_gesture(&[MotionGestureEventKind::Pan]),
      drag: self.has_any_gesture(&[
        MotionGestureEventKind::DragStart,
        MotionGestureEventKind::DragDirectionLock,
        MotionGestureEventKind::DragEnd,
        MotionGestureEventKind::DragCancel,
      ]),
      drag_update: self.has_any_gesture(&[MotionGestureEventKind::Drag]),
      momentum_complete: self.has_any_gesture(&[MotionGestureEventKind::DragMomentumComplete]),
      constraints_measured: self
        .has_any_gesture(&[MotionGestureEventKind::DragConstraintsMeasured]),
      scroll: self.has_any_gesture(&[MotionGestureEventKind::Scroll]),
      in_view: self.has_any_gesture(&[
        MotionGestureEventKind::InViewEnter,
        MotionGestureEventKind::InViewLeave,
      ]),
    }
  }

  pub(crate) fn invoke(&self, game: &mut dyn Any, event: &MotionLifecycleEvent) -> bool {
    let handler = match event.kind {
      MotionEventKind::Started => &self.start,
      MotionEventKind::Repeated { .. } => &self.repeat,
      MotionEventKind::Completed => &self.complete,
      MotionEventKind::Stopped => &self.stop,
      MotionEventKind::Cancelled => &self.cancel,
      MotionEventKind::Activated => return false,
    };
    if let Some(handler) = handler {
      handler.invoke(game, event);
      true
    } else {
      false
    }
  }

  pub(crate) fn invoke_sample(
    &self,
    game: &mut dyn Any,
    sample: &MotionPresentationSample,
  ) -> bool {
    if let Some(handler) = &self.update {
      handler.invoke(game, sample);
      true
    } else {
      false
    }
  }

  pub(crate) fn invoke_gesture(&self, game: &mut dyn Any, event: &MotionGestureEvent) -> bool {
    let index = gesture_index(event.kind);
    if let Some(observer) = &self.gesture_observers[index] {
      observer();
    }
    if let Some(handler) = &self.gestures[index] {
      handler.invoke(game, event);
    }
    self.gesture_observers[index].is_some() || self.gestures[index].is_some()
  }

  pub(crate) fn validate_model(&self, model: TypeId) {
    for handler in [
      &self.start,
      &self.repeat,
      &self.complete,
      &self.stop,
      &self.cancel,
    ]
    .into_iter()
    .flatten()
    {
      assert_eq!(
        handler.model, model,
        "Motion callback model type does not match its runtime"
      );
    }
    if let Some(handler) = &self.update {
      assert_eq!(
        handler.model, model,
        "Motion update callback model type does not match its runtime"
      );
    }
    for handler in self.gestures.iter().flatten() {
      assert_eq!(
        handler.model, model,
        "Motion gesture callback model type does not match its runtime"
      );
    }
  }

  fn set(&mut self, kind: MotionCallbackKind, handler: MotionHandler) {
    *match kind {
      MotionCallbackKind::Start => &mut self.start,
      MotionCallbackKind::Repeat => &mut self.repeat,
      MotionCallbackKind::Complete => &mut self.complete,
      MotionCallbackKind::Stop => &mut self.stop,
      MotionCallbackKind::Cancel => &mut self.cancel,
    } = Some(handler);
  }

  fn has_any_gesture(&self, kinds: &[MotionGestureEventKind]) -> bool {
    kinds.iter().any(|kind| {
      let index = gesture_index(*kind);
      self.gestures[index].is_some() || self.gesture_observers[index].is_some()
    })
  }
}

const fn gesture_index(kind: MotionGestureEventKind) -> usize {
  match kind {
    MotionGestureEventKind::HoverStart => 0,
    MotionGestureEventKind::HoverEnd => 1,
    MotionGestureEventKind::TapStart => 2,
    MotionGestureEventKind::Tap => 3,
    MotionGestureEventKind::TapCancel => 4,
    MotionGestureEventKind::FocusStart => 5,
    MotionGestureEventKind::FocusEnd => 6,
    MotionGestureEventKind::FocusVisibleStart => 22,
    MotionGestureEventKind::FocusVisibleEnd => 23,
    MotionGestureEventKind::PanSessionStart => 7,
    MotionGestureEventKind::PanStart => 8,
    MotionGestureEventKind::Pan => 9,
    MotionGestureEventKind::PanEnd => 10,
    MotionGestureEventKind::PanCancel => 11,
    MotionGestureEventKind::DragStart => 12,
    MotionGestureEventKind::DragDirectionLock => 13,
    MotionGestureEventKind::Drag => 14,
    MotionGestureEventKind::DragEnd => 15,
    MotionGestureEventKind::DragCancel => 16,
    MotionGestureEventKind::DragMomentumComplete => 17,
    MotionGestureEventKind::DragConstraintsMeasured => 18,
    MotionGestureEventKind::Scroll => 19,
    MotionGestureEventKind::InViewEnter => 20,
    MotionGestureEventKind::InViewLeave => 21,
  }
}

impl MotionCallbackRegistration {
  pub(crate) fn for_descriptor(
    descriptor: &MotionDescriptor,
    callbacks: &MotionCallbacks,
  ) -> Vec<Self> {
    descriptor
      .slots
      .iter()
      .filter(|slot| slot.callbacks != MotionCallbackSubscriptions::default())
      .map(|slot| Self {
        descriptor_id: descriptor.descriptor_id,
        slot: slot.slot,
        generation: slot.generation,
        callbacks: callbacks.clone(),
      })
      .collect()
  }

  pub(crate) fn matches(&self, event: &MotionLifecycleEvent) -> bool {
    self.descriptor_id == event.descriptor_id
      && self.slot == event.slot
      && self.generation == event.generation
  }

  pub(crate) fn invoke(&self, game: &mut dyn Any, event: &MotionLifecycleEvent) -> bool {
    self.callbacks.invoke(game, event)
  }

  pub(crate) fn validate_model(&self, model: TypeId) {
    self.callbacks.validate_model(model);
  }
}

pub(crate) fn carry_registrations(
  previous: &[MotionCallbackRegistration],
  descriptor: Option<&MotionDescriptor>,
  callbacks: &MotionCallbacks,
) -> Vec<MotionCallbackRegistration> {
  let mut registrations = previous.to_vec();
  if let Some(descriptor) = descriptor {
    for registration in MotionCallbackRegistration::for_descriptor(descriptor, callbacks) {
      if !registrations.iter().any(|value| {
        value.descriptor_id == registration.descriptor_id
          && value.slot == registration.slot
          && value.generation == registration.generation
      }) {
        registrations.push(registration);
      }
    }
  }
  if registrations.len() > 32 {
    registrations.drain(..registrations.len() - 32);
  }
  registrations
}

impl MotionHandler {
  fn new<G: 'static>(callback: impl Fn(&mut G, &MotionLifecycleEvent) + 'static) -> Self {
    Self {
      model: TypeId::of::<G>(),
      callback: Rc::new(move |game, event| {
        callback(
          game
            .downcast_mut::<G>()
            .expect("Motion callback model type was not validated"),
          event,
        );
      }),
    }
  }

  fn invoke(&self, game: &mut dyn Any, event: &MotionLifecycleEvent) {
    (self.callback)(game, event);
  }
}

impl MotionUpdateHandler {
  fn new<G: 'static>(callback: impl Fn(&mut G, &MotionPresentationSample) + 'static) -> Self {
    Self {
      model: TypeId::of::<G>(),
      callback: Rc::new(move |game, sample| {
        callback(
          game
            .downcast_mut::<G>()
            .expect("Motion update callback model type was not validated"),
          sample,
        );
      }),
    }
  }

  fn invoke(&self, game: &mut dyn Any, sample: &MotionPresentationSample) {
    (self.callback)(game, sample);
  }
}

impl GestureHandler {
  fn new<G: 'static>(callback: impl Fn(&mut G, &MotionGestureEvent) + 'static) -> Self {
    Self {
      model: TypeId::of::<G>(),
      callback: Rc::new(move |game, event| {
        callback(
          game
            .downcast_mut::<G>()
            .expect("Motion gesture callback model type was not validated"),
          event,
        );
      }),
    }
  }

  fn invoke(&self, game: &mut dyn Any, event: &MotionGestureEvent) {
    (self.callback)(game, event);
  }
}

impl fmt::Debug for MotionCallbacks {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("MotionCallbacks")
      .field("subscriptions", &self.subscriptions())
      .field("gesture_subscriptions", &self.gesture_subscriptions())
      .finish()
  }
}

impl PartialEq for MotionCallbacks {
  fn eq(&self, other: &Self) -> bool {
    self.subscriptions() == other.subscriptions()
      && self.gesture_subscriptions() == other.gesture_subscriptions()
  }
}
