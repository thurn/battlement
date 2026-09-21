//! Native Reactant rules engine for the standalone chess sample.

mod ai;
mod app;
#[allow(dead_code)]
mod assets;
mod audio;
mod chess_board;
mod chess_prompt;
mod chess_ui_state;
mod cursor;
mod motion;
mod persistence;
mod position;
mod promotion_dialog;
mod reactant_effects;
mod reactant_game;
mod reactant_input;
mod reactant_view;
mod visual_state;

pub mod contract;

pub use app::{EngineDependencies, create_engine};
pub use reactant::{Engine, PersistenceBackend};

reactant::export_application!(app::application);
