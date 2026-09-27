//! Application observations and session-bound command handles.

use battlement::host_settings::HostSettings;
use std::{
  cell::RefCell,
  path::PathBuf,
  rc::{Rc, Weak},
};

use battlement::application::{ApplicationState, ReducedMotionPreference};
use battlement::{ActionId, Command, DisplayId, ObjectId, Rect, ScreenSize};
use trox::Localizer;

use crate::{
  action_context,
  geometry::{self, MeasurementStatus, ViewportRef},
  hooks, localization,
  work_scope::WorkScope,
};

/// Submits native work without owning a protocol session or response queue.
#[derive(Clone)]
pub struct AppHandle {
  queue: Weak<RefCell<AppQueue>>,
  generation: u64,
  origin: Option<ActionId>,
  scope: Option<u64>,
}

/// Display dimensions and unobscured bounds in logical, top-left coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
  /// Full display size in logical pixels.
  pub size: ScreenSize,
  /// Unobscured rectangle relative to the display origin.
  pub safe_area: Rect,
}

/// Returns the current application's session-bound operations handle.
pub fn use_app() -> AppHandle {
  let mut handle = hooks::use_required_context::<AppHandle>();
  handle.scope = hooks::use_optional_context::<WorkScope>()
    .map(|scope| scope.0)
    .filter(|scope| *scope != 0);
  handle
}

/// Reads logical display dimensions, using the connection size until measured.
pub fn use_viewport_size() -> ScreenSize {
  self::use_viewport().size
}

/// Observes resize, display scale and safe-area changes as one coherent value.
pub fn use_viewport() -> Viewport {
  let initial = hooks::use_required_context::<ScreenSize>();
  let measurement = geometry::use_geometry(ViewportRef::display(DisplayId(0))).measurements;
  let fallback = Viewport {
    size: initial,
    safe_area: Rect {
      x: 0.0,
      y: 0.0,
      width: initial.width.into(),
      height: initial.height.into(),
    },
  };
  if measurement.status == MeasurementStatus::Waiting {
    return fallback;
  }
  measurement.latest.map_or(fallback, |geometry| {
    let scale = if geometry.scale.is_finite() && geometry.scale > 0.0 {
      geometry.scale
    } else {
      1.0
    };
    let size = ScreenSize::new(
      (geometry.viewport.width / scale).round() as u32,
      (geometry.viewport.height / scale).round() as u32,
    );
    let x = (geometry.safe_area.x / scale).clamp(0.0, size.width.into());
    let y = (geometry.safe_area.y / scale).clamp(0.0, size.height.into());
    let right =
      ((geometry.safe_area.x + geometry.safe_area.width) / scale).clamp(x, size.width.into());
    let bottom =
      ((geometry.safe_area.y + geometry.safe_area.height) / scale).clamp(y, size.height.into());
    Viewport {
      size,
      safe_area: Rect {
        x,
        y,
        width: right - x,
        height: bottom - y,
      },
    }
  })
}

/// Connection capabilities supplied to component-first application services.
#[doc(hidden)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostEnvironment {
  /// Selected native host modules.
  pub modules: Vec<String>,
  /// Current host-supported settings and applied values.
  pub settings: HostSettings,
  /// Per-application persistent storage directory.
  pub persistent_data_path: Option<PathBuf>,
}

impl AppHandle {
  /// Queues a native command after the current UI commit.
  pub fn send(&self, command: Command) {
    self.with_queue(|queue| {
      queue.commands.push(QueuedCommand {
        command,
        scope: self.scope,
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

  /// Acquires or releases one app-owned pause for a captured game presentation scope.
  #[doc(hidden)]
  pub fn set_game_presentation_paused(&self, scope: u64, owner_id: ObjectId, paused: bool) {
    assert!(scope != 0, "game presentation scope must be nonzero");
    self.with_queue(|queue| {
      queue.presentation_controls.push(QueuedPresentationControl {
        scope,
        owner_id,
        paused,
        action: action_context::current().or(self.origin),
      });
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
      scope: None,
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
    (self.scope, self.generation, self.origin) == (other.scope, other.generation, other.origin)
      && Weak::ptr_eq(&self.queue, &other.queue)
  }
}

#[derive(Default)]
pub(crate) struct AppQueue {
  pub(crate) generation: u64,
  pub(crate) commands: Vec<QueuedCommand>,
  pub(crate) presentation_controls: Vec<QueuedPresentationControl>,
  pub(crate) snapshot_action: Option<ActionId>,
  pub(crate) snapshot: bool,
  pub(crate) localizer: Option<Rc<Localizer>>,
}

pub(crate) struct Observations {
  pub(crate) application: ApplicationState,
  pub(crate) reduced_motion: ReducedMotionPreference,
  pub(crate) screen: ScreenSize,
  pub(crate) remount: u64,
  pub(crate) host: HostEnvironment,
}

pub(crate) struct QueuedCommand {
  pub(crate) scope: Option<u64>,
  pub(crate) command: Command,
  pub(crate) action: Option<ActionId>,
}

pub(crate) struct QueuedPresentationControl {
  pub(crate) scope: u64,
  pub(crate) owner_id: ObjectId,
  pub(crate) paused: bool,
  pub(crate) action: Option<ActionId>,
}
