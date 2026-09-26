//! Native Reactant rules engine for the standalone chess sample.

mod ai;
mod app;
#[allow(dead_code)]
pub mod assets;
pub mod audio;
mod board_shake;
mod chess_board;
mod chess_labels;
mod chess_opening;
mod chess_prompt;
mod chess_ui_state;
mod computer_turn;
mod cursor;
mod localization;
mod menu;
mod motion;
mod opponent;
pub mod persistence;
mod position;
mod promotion_dialog;
mod reactant_effects;
mod reactant_game;
mod reactant_input;
mod reactant_view;
mod saved_progress;
pub mod settings;
mod visual_state;

pub use chess_prompt::ChessPrompt;
pub use opponent::Opponent;

pub use app::{EngineDependencies, create_application, create_engine};
pub use reactant::{Engine, PersistenceBackend};
pub use reactant_game::ChessGame;

reactant::export_application!(app::application);
