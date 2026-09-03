//! Application lifecycle observations and external requests.

use serde::{Deserialize, Serialize};

use crate::{Command, CommandBody, Connect};

/// The latest focus and pause observations supplied by Unity.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationState {
  /// Whether the player owns application focus.
  pub focused: bool,
  /// Whether Unity has suspended the application.
  pub paused: bool,
}

/// Host-reported preference for reducing nonessential motion.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReducedMotionPreference {
  /// The current Unity target cannot report a preference.
  #[default]
  Unavailable,
  /// The host requests reduced motion.
  Reduce,
  /// The host reports no reduced-motion preference.
  NoPreference,
}

/// Requests the platform's external handler for an absolute URL.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUrlRequest {
  /// Absolute URL supplied by the application.
  pub url: String,
}

impl Default for ApplicationState {
  fn default() -> Self {
    Self {
      focused: true,
      paused: false,
    }
  }
}

impl ApplicationState {
  /// Whether the application is focused and not suspended.
  #[must_use]
  pub const fn is_active(self) -> bool {
    self.focused && !self.paused
  }
}

impl Command {
  /// Requests an external URL; completion acknowledges dispatch, not page loading.
  #[must_use]
  pub fn open_external_url(url: impl Into<String>) -> Self {
    let url = url.into();
    assert!(!url.trim().is_empty(), "external URL must not be empty");
    Self::new_v4(CommandBody::ApplicationOpenUrl(ExternalUrlRequest { url }))
  }
}

impl Connect {
  /// Supplies the host's initial application observations.
  #[must_use]
  pub fn application_state(mut self, state: ApplicationState) -> Self {
    self.application_state = state;
    self
  }
}
