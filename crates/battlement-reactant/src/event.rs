//! Typed views of Reactant event dispatches.

use std::{cell::Cell, rc::Rc};

use battlement::{ObjectId, UiEventBody};

use crate::runtime::Root;

/// The logical phase observed by one Reactant handler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventPhase {
  /// Logical capture traversal.
  Capture,
  /// The originating target.
  Target,
  /// Logical bubble traversal.
  Bubble,
}

/// Identifies a logical host at the time an event was dispatched.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ElementTarget {
  root: Root,
  object_id: ObjectId,
}

/// A typed view of one shared event dispatch.
pub struct ReactantEvent<E> {
  inner: Rc<EventInner>,
  payload: EventPayload<E>,
  current_target: ElementTarget,
  phase: EventPhase,
}

impl ElementTarget {
  /// Returns the event-time native host identity.
  #[must_use]
  pub const fn object_id(self) -> ObjectId {
    self.object_id
  }

  /// Returns the logical source root.
  #[must_use]
  pub const fn root(self) -> Root {
    self.root
  }

  pub(crate) const fn new(root: Root, object_id: ObjectId) -> Self {
    Self { root, object_id }
  }
}

impl<E> ReactantEvent<E> {
  /// Returns the event-family-specific payload.
  #[must_use]
  pub fn payload(&self) -> &E {
    match &self.payload {
      EventPayload::Shared { body, extract } => extract(body),
      EventPayload::Owned(payload) => payload,
    }
  }

  /// Returns the original logical target.
  #[must_use]
  pub fn target(&self) -> ElementTarget {
    self.inner.target
  }

  /// Returns the host whose callback is currently running.
  #[must_use]
  pub fn current_target(&self) -> ElementTarget {
    self.current_target
  }

  /// Returns the logical route phase.
  #[must_use]
  pub fn phase(&self) -> EventPhase {
    self.phase
  }

  /// Stops later logical callbacks for this dispatch.
  pub fn stop_propagation(&self) {
    self.inner.propagation_stopped.set(true);
  }

  pub(crate) fn new(
    inner: Rc<EventInner>,
    body: Rc<UiEventBody>,
    extract: fn(&UiEventBody) -> &E,
    current_target: ElementTarget,
    phase: EventPhase,
  ) -> Self {
    Self {
      inner,
      payload: EventPayload::Shared { body, extract },
      current_target,
      phase,
    }
  }

  pub(crate) fn new_owned(
    inner: Rc<EventInner>,
    payload: E,
    current_target: ElementTarget,
    phase: EventPhase,
  ) -> Self {
    Self {
      inner,
      payload: EventPayload::Owned(Rc::new(payload)),
      current_target,
      phase,
    }
  }
}

impl<E> Clone for ReactantEvent<E> {
  fn clone(&self) -> Self {
    Self {
      inner: Rc::clone(&self.inner),
      payload: self.payload.clone(),
      current_target: self.current_target,
      phase: self.phase,
    }
  }
}

pub(crate) struct EventInner {
  target: ElementTarget,
  propagation_stopped: Rc<Cell<bool>>,
}

impl EventInner {
  pub(crate) fn new(target: ElementTarget, propagation_stopped: Rc<Cell<bool>>) -> Self {
    Self {
      target,
      propagation_stopped,
    }
  }

  pub(crate) fn propagation_stopped(&self) -> bool {
    self.propagation_stopped.get()
  }
}

enum EventPayload<E> {
  Shared {
    body: Rc<UiEventBody>,
    extract: fn(&UiEventBody) -> &E,
  },
  Owned(Rc<E>),
}

impl<E> Clone for EventPayload<E> {
  fn clone(&self) -> Self {
    match self {
      Self::Shared { body, extract } => Self::Shared {
        body: Rc::clone(body),
        extract: *extract,
      },
      Self::Owned(payload) => Self::Owned(Rc::clone(payload)),
    }
  }
}
