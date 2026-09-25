//! Root preference ownership and stable chess setting identities.

pub(crate) mod audio;
pub mod bindings;
mod context;
mod model;
pub mod reporting;
mod save_status;

pub use context::{SettingsContext, SettingsRoot, use_settings};
pub use model::{ChessSettings, FILE_NAME, Language, SettingsChange, TextSize};
pub use save_status::SettingsSaveStatus;
