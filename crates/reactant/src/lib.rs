//! Ergonomic Reactant application facade.
//!
//! This crate combines the shared component runtime, Battlement UI authoring,
//! and typed rules-session machinery while keeping those lower layers acyclic.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[path = "application.rs"]
mod application_shell;
mod game_app;
mod game_hooks;
mod game_output;
mod game_session;
mod input;
mod persistence;
mod persistence_file;
mod persistence_operation;
mod persistence_store;
pub mod prelude;
mod presentation_inspector;
mod timers;
pub mod world;
mod world_adapter;
mod world_hit_region;
mod world_layout;
mod world_layout_algorithms;
mod world_layout_builders;
mod world_object;
mod world_properties;
mod world_text;
mod world_view;
mod world_visuals;

pub use application_shell::{Application, ApplicationEngine, use_portal_target};
pub use battlement_native::Engine;
pub use game_app::use_game;
pub use game_hooks::{
  GamePresentation, GameRoot, SnapshotAnimation, use_animate, use_game_observation,
  use_game_presentation, use_game_prompt, use_game_publication, use_game_selector,
  use_game_selector_with, use_game_state, use_game_status,
};
pub use game_output::{GameConsumer, GameOutput};
pub use game_session::{DispatchResult, GameHandle, GameObservation, GameStatus};
pub use input::{GlobalInput, use_global_input};
pub use persistence::{
  PersistentState, use_host_module, use_persistent_state, use_persistent_state_with,
};
pub use persistence_file::FilePersistenceBackend;
pub use persistence_operation::{
  PersistenceBackend, PersistenceCompletion, PersistenceId, PersistenceOperation,
  PersistenceRequest,
};
pub use persistence_store::{PersistenceSnapshot, PersistenceStore};
pub use presentation_inspector::{InspectorObject, PresentationInspector};
pub use reactant_core::{
  __register_generated_asset, animation_controls, announcement, app_context, application,
  asset_generator, audio, callback, component, components, context, control_behavior,
  cooperative_executor, display_store, element_behavior, element_ref, error_boundary, event,
  executor, external_store, focus, geometry, gesture, hooks, host, host_node, identity, key,
  label_binding, layout, local_point, motion, motion_config, motion_value, native_host,
  navigation_handlers, overlay, paint, pointer_handlers, portal, presence, presentation, props,
  render, resource, resource_control, scale_to_fit, semantics, suspense, visibility,
};
pub use timers::{use_interval, use_timeout};

#[doc(hidden)]
pub use battlement_native as __native;

/// Typed rules execution and worker lifecycle support.
pub use reactant_rules as rules;

/// Battlement UI controls and host adapters.
pub use reactant_ui as ui;

/// Low-level compatibility surface for Reactant's own transport tests.
#[doc(hidden)]
pub mod testing {
  pub use crate::game_app::GameApp;
  pub use reactant_core::app::App;
}
