//! Application observations and session-bound command handles.

use std::{
  cell::RefCell,
  rc::{Rc, Weak},
};

use battlement::application::{ApplicationState, ReducedMotionPreference};
use battlement::{ActionId, Command, DisplayId, ScreenSize};
use trox::Localizer;

use crate::{
  action_context,
  geometry::{self, MeasurementStatus, ViewportRef},
  hooks, localization,
};

/// Submits native work without owning a protocol session or response queue.
#[derive(Clone)]
pub struct AppHandle {
  queue: Weak<RefCell<AppQueue>>,
  generation: u64,
  origin: Option<ActionId>,
}

/// Returns the current application's session-bound operations handle.
pub fn use_app() -> AppHandle {
  hooks::use_required_context::<AppHandle>()
}

/// Reads logical display dimensions, using the connection size until measured.
pub fn use_viewport_size() -> ScreenSize {
  let initial = hooks::use_required_context::<ScreenSize>();
  let measurement = geometry::use_geometry(ViewportRef::display(DisplayId(0))).measurements;
  if measurement.status == MeasurementStatus::Waiting {
    return initial;
  }
  measurement.latest.map_or(initial, |geometry| {
    let scale = if geometry.scale.is_finite() && geometry.scale > 0.0 {
      geometry.scale
    } else {
      1.0
    };
    ScreenSize::new(
      (geometry.viewport.width / scale).round() as u32,
      (geometry.viewport.height / scale).round() as u32,
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

  /// Replaces the application's localizer after the current commit.
  pub fn set_localizer(&self, localizer: Localizer) {
    let Some(queue) = self.queue.upgrade() else {
      return;
    };
    let mut queue = queue.borrow_mut();
    if self.generation != queue.generation {
      return;
    }
    let localizer = Rc::new(localizer);
    localization::replace_announcement_localizer(Rc::clone(&localizer));
    queue.localizer = Some(localizer);
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
  pub(crate) localizer: Option<Rc<Localizer>>,
}

pub(crate) struct Observations {
  pub(crate) application: ApplicationState,
  pub(crate) reduced_motion: ReducedMotionPreference,
  pub(crate) screen: ScreenSize,
  pub(crate) remount: u64,
}

pub(crate) struct QueuedCommand {
  pub(crate) command: Command,
  pub(crate) action: Option<ActionId>,
}
