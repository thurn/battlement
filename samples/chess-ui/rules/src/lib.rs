//! A worked example of a Battlement application and a small Reactant design system.
//!
//! Start at [`engine::create_engine`], which mounts [`gallery::Gallery`] through
//! the application builder. [`pages::gallery`] registers configured component
//! values as review pages. Each selection gets a fresh Reactant key, resetting state and
//! focusing the page heading so experiments are repeatable.
//!
//! The `review_*` components own the gallery theme and accessible behavior.
//! The arcade controls own their game appearance; callers supply typed content,
//! current values, and callbacks through builders. Parent state remains the
//! authority for a controlled checkbox or slider value.
//!
//! [`portrait_viewport::PortraitViewport`] and [`review_stage::ReviewStage`]
//! use Reactant's measured `ScaleToFit` canvas. `LabelBinding` connects composed
//! visible labels to controls, while common focus and reveal hooks own effects.
//! Explicit element refs are reserved for the navigation scroll container and
//! the checkbox input that must receive focus when its label is activated.
//!
//! ```
//! use battlement_reactant::{app::App, host::TextElement};
//! use battlement_rules::{pages, toggle_control::ToggleControl};
//!
//! let app = App::new("chess-ui/content").ui(pages::gallery());
//! let toggle = ToggleControl::new(TextElement::new("VSync"), false, |checked| {
//!     // Store the accepted value in parent state and render it again.
//! }).first(true);
//! ```

pub mod action_button;
mod action_harness;
mod action_skin;
mod assets;
pub mod caret;
pub mod check_mark;
pub mod clipped_inset;
pub mod concept_frame;
pub mod engine;
mod frame_harness;
mod frame_styles;
pub mod gallery;
pub mod pages;
mod portrait_harness;
pub mod portrait_viewport;
pub mod return_button;
pub mod review_button;
pub mod review_navigation;
pub mod review_page;
pub mod review_panel;
pub mod review_stage;
pub mod review_surface;
pub mod review_text;
mod review_theme;
pub mod screen_frame;
pub mod select_control;
mod select_harness;
pub mod setting_row;
mod setting_row_harness;
pub mod toggle_control;
mod toggle_harness;
pub mod volume_control;
mod volume_harness;
mod volume_skin;
