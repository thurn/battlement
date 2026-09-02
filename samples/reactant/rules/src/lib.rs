//! Reactant sample components and application.

mod animation_validation;
mod app_setup;
mod assets;
mod collection_settings;
mod composed_effects;
mod composition;
mod context_memo;
mod controls;
mod design_system;
mod effects_stores;
mod events_portals;
mod gestures_drag;
mod layout_gallery;
mod layout_gallery_styles;
mod layout_performance;
mod layout_reorder;
mod model;
mod motion_performance;
mod navigation;
mod physical_motion;
mod presence_lifecycle;
mod preview_resource;
mod refs_geometry;
mod resources_boundaries;
mod sample_constants;
mod sample_navigation;
mod sample_shell;
mod screens;
mod state_identity;
mod styles_decorations;
#[cfg(test)]
mod tests;
mod values_time_controls;
mod variants_orchestration;

pub use app_setup::{ReactantEngine, create_engine, generated_asset_addresses};
pub(crate) use controls::{control_state, interactive_button};
pub use model::Game;
pub(crate) use model::{Control, Interaction};
pub(crate) use sample_constants::MISSING_GEOMETRY_TARGET_ID;
pub use sample_constants::{
  CONTENT_SCENE, DITTO_VISUAL_STATE_REGISTRY, GEOMETRY_TARGET_ID, MOTION_AUDIO_CLIP,
  MOTION_MATERIAL, MOTION_TEXTURE, ROOT_ID, Screen,
};
