//! Ergonomic Reactant application facade.
//!
//! This crate combines the shared component runtime, Battlement UI authoring,
//! and typed rules-session machinery while keeping those lower layers acyclic.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod game_app;
mod game_hooks;
mod game_output;
mod game_session;
pub mod prelude;
pub mod world;
mod world_adapter;

pub use game_app::GameApp;
pub use game_hooks::{
  GameRoot, use_game_prompt, use_game_selector, use_game_selector_with, use_game_state,
  use_game_status,
};
pub use game_output::{GameConsumer, GameOutput};
pub use game_session::{DispatchResult, GameHandle, GameStatus};
pub use reactant_core::*;

/// Typed rules execution and worker lifecycle support.
pub use reactant_rules as rules;

/// Battlement UI controls and host adapters.
pub use reactant_ui as ui;
