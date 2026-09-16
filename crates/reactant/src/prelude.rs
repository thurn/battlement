//! Ordinary component authoring with app-owned game sessions.

pub use crate::{
  game_app::GameApp,
  game_hooks::{GameRoot, use_game_prompt, use_game_state, use_game_status},
  game_session::{DispatchResult, GameHandle, GameStatus},
};
pub use reactant_core::prelude::*;
