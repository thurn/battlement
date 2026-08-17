//! Canonical Rust types for the Masonry wire protocol.
//!
//! Masonry is a Unity rendering and input client for turn-based games. This
//! crate models the JSON exchanged between that client and an authoritative
//! rules engine.
//!
//! The main entry points are [`Connect`], [`Response`], [`ResponseMessage`],
//! [`ClientMessage`], [`Snapshot`], and [`Batch`]. Rules engines normally build
//! commands with [`Command`] and [`CommandBody`]. Game-specific integrations
//! can use [`CustomAction`] and [`CustomCommand`] without giving up strongly
//! typed IDs or the shared command and action formats.
//!
//! These types derive [`serde::Serialize`], [`serde::Deserialize`], and
//! [`schemars::JsonSchema`]. Schema generation itself intentionally lives
//! outside this crate's public API.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod assets;
pub mod commands;
pub mod ids;
pub mod messages;
pub mod objects;
mod serialization;
pub mod values;

pub use assets::*;
pub use commands::*;
pub use ids::*;
pub use messages::*;
pub use objects::*;
pub use values::*;

#[cfg(test)]
mod schema_tests {
    use schemars::generate::SchemaSettings;
    use serde_json::Value;

    use crate::{ClientMessage, Command, Connect, Response};

    #[test]
    fn public_roots_generate_draft_7_schemas_in_memory() {
        for schema in [
            draft_7_schema::<Response>(),
            draft_7_schema::<Command>(),
            draft_7_schema::<ClientMessage>(),
        ] {
            assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
        }
    }

    #[test]
    fn generated_command_schema_contains_core_discriminators() {
        let schema = draft_7_schema::<Command>();
        let text = serde_json::to_string(&schema).unwrap();

        assert!(text.contains("masonry.transform.tweenWorldPosition"));
        assert!(text.contains("masonry.input.setGlobalKeys"));
        assert!(text.contains("additionalProperties"));
    }

    #[test]
    fn generated_record_schemas_are_strict_without_changing_serde_tolerance() {
        let schema = draft_7_schema::<Connect>();

        assert_eq!(schema["additionalProperties"], false);
    }

    fn draft_7_schema<T: schemars::JsonSchema>() -> Value {
        serde_json::to_value(
            SchemaSettings::draft07()
                .into_generator()
                .into_root_schema_for::<T>(),
        )
        .unwrap()
    }
}
