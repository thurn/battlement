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

#[cfg(test)]
mod tests;

pub use app::application;

reactant::export_application!(application);
