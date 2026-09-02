//! Reactive application lifecycle context.

use battlement::application::ApplicationState;

use crate::{
  context::{RequiredContext, RequiredContextProvider},
  hooks,
};

static APPLICATION_STATE: RequiredContext<ApplicationState> = RequiredContext::new();

/// Provides the engine's latest application observation to a logical subtree.
/// Nested providers can supply controlled observations for an isolated preview.
#[must_use]
pub fn provider(state: ApplicationState) -> RequiredContextProvider<ApplicationState> {
  APPLICATION_STATE.provider(state)
}

/// Reads application focus and suspension, rerendering when the provider changes.
#[must_use]
pub fn use_application_state() -> ApplicationState {
  hooks::use_required_context(&APPLICATION_STATE)
}
