//! Application observations and session-bound command handles.

use std::{
  cell::RefCell,
  rc::{Rc, Weak},
};

use battlement::application::ApplicationState;
use battlement::{ActionId, Command, DisplayId, ScreenSize};

use crate::{
  action_context,
  context::RequiredContext,
  geometry::{self, MeasurementStatus, ViewportRef},
  hooks,
};

pub(crate) static APP: RequiredContext<AppHandle> = RequiredContext::new();
pub(crate) static VIEWPORT: RequiredContext<ScreenSize> = RequiredContext::new();

/// Submits native work without owning a protocol session or response queue.
#[derive(Clone)]
pub struct AppHandle {
  queue: Weak<RefCell<AppQueue>>,
  generation: u64,
  origin: Option<ActionId>,
}

/// Returns the current application's session-bound operations handle.
pub fn use_app() -> AppHandle {
  hooks::use_required_context(&APP)
}

/// Reads physical display dimensions, using the connection size until measured.
pub fn use_viewport_size() -> ScreenSize {
  let initial = hooks::use_required_context(&VIEWPORT);
  let measurement = geometry::use_geometry(ViewportRef::display(DisplayId(0))).measurements;
  if measurement.status == MeasurementStatus::Waiting {
    return initial;
  }
  measurement.latest.map_or(initial, |geometry| {
    ScreenSize::new(
      geometry.viewport.width as u32,
      geometry.viewport.height as u32,
    )
  })
}

impl AppHandle {
  /// Queues a native command after the current UI commit.
  pub fn send(&self, command: Command) {
    self.with_queue(|queue| {
      queue.commands.push(QueuedCommand {
        command,
        action: action_context::current().or(self.origin),
      })
    });
  }

  /// Rebuilds the client presentation while retaining application and component state.
  pub fn refresh_snapshot(&self) {
    self.with_queue(|queue| {
      queue.snapshot = true;
      queue.snapshot_action = action_context::current().or(self.origin);
    });
  }

  pub(crate) fn new(queue: &Rc<RefCell<AppQueue>>) -> Self {
    Self {
      queue: Rc::downgrade(queue),
      generation: queue.borrow().generation,
      origin: action_context::current(),
    }
  }

  fn with_queue(&self, update: impl FnOnce(&mut AppQueue)) {
    if let Some(queue) = self.queue.upgrade() {
      let mut queue = queue.borrow_mut();
      if self.generation == queue.generation {
        update(&mut queue);
      }
    }
  }
}

impl PartialEq for AppHandle {
  fn eq(&self, other: &Self) -> bool {
    self.generation == other.generation
      && self.origin == other.origin
      && Weak::ptr_eq(&self.queue, &other.queue)
  }
}

#[derive(Default)]
pub(crate) struct AppQueue {
  pub(crate) generation: u64,
  pub(crate) commands: Vec<QueuedCommand>,
  pub(crate) snapshot_action: Option<ActionId>,
  pub(crate) snapshot: bool,
}

pub(crate) struct Observations {
  pub(crate) application: ApplicationState,
  pub(crate) screen: ScreenSize,
  pub(crate) remount: u64,
}

pub(crate) struct QueuedCommand {
  pub(crate) command: Command,
  pub(crate) action: Option<ActionId>,
}
