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
//! use Reactant's measured `ScaleToFit` canvas. Associated control labels connect
//! composed visible labels to explicitly selected control hosts, while common
//! focus and reveal hooks own effects. Explicit element refs remain reserved for
//! the navigation scroll container.
//!
//! ```no_run
//! use battlement_reactant::{control_behavior, app::App};
//! use battlement_rules::{pages, toggle_control::ToggleControl};
//! use trox::tx;
//!
//! let mut app = App::new("chess-ui/content");
//! let overlay = app.create_portal_target();
//! let app = app.ui(pages::gallery(overlay));
//! let toggle = ToggleControl::new()
//!   .label(control_behavior::name_source_text(tx(
//!     "VSync",
//!     "Visible label for the VSync setting.",
//!   )))
//!   .checked(false)
//!   .on_change(|checked| {
//!     // Store the accepted value in parent state and render it again.
//!   })
//!   .first(true);
//! ```

pub mod action_button;

mod action_harness;

mod action_skin;

pub mod arcade_modal;

mod arcade_modal_harness;

mod assets;

pub mod caret;

pub mod check_mark;

pub mod concept_frame;

pub mod engine;

mod frame_harness;

mod frame_styles;

pub mod font_scale;

mod font_scale_harness;

pub mod gallery;

pub mod input_settings;

pub mod input_binding_icons;

mod input_skin_harness;

pub mod pages;

mod portrait_harness;

mod privacy_harness;

pub mod privacy_policy;

pub mod portrait_viewport;

pub mod return_button;

mod rendering_audit;

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

mod select_navigation;

mod select_option;

mod select_popover_harness;

pub mod setting_row;

mod setting_row_harness;

pub mod toggle_control;

mod toggle_harness;

mod toggle_accessibility_harness;

pub mod use_interaction;

pub mod volume_control;

mod volume_harness;

mod volume_input;

mod volume_input_harness;

mod volume_skin;

pub mod settings_tabs;

pub mod settings_panel;

mod tabs_harness;

mod tabs_navigation;

mod tabs_skin;

mod header_artwork;

mod header_harness;

mod interaction_harness;

pub mod screen_header;
