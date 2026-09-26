mod app;
#[allow(dead_code)]
pub mod assets;
pub mod card_assets;
pub mod controller;
pub mod domain;
pub mod projection;
pub mod reducer;

pub use projection::HumanView;

pub use app::{application, application_from_state};
pub use controller::{HeartsController, use_hearts, use_hearts_with_policy};
pub use reducer::HeartsReducer;

reactant::export_application!(app::exported_application);
