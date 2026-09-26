mod app;
#[allow(dead_code)]
pub mod assets;
pub mod card_assets;
mod card_controls;
mod card_gesture;
mod card_input;
#[cfg(test)]
mod card_input_tests;
mod card_table;
#[cfg(test)]
mod card_table_tests;
pub mod controller;
pub mod domain;
mod layout_fixture;
pub mod projection;
pub mod reducer;
mod scene;

pub use projection::HumanView;

pub use app::{application, application_from_state};
pub use controller::{HeartsController, use_hearts, use_hearts_with_policy};
pub use reducer::HeartsReducer;

reactant::export_application!(app::exported_application);
