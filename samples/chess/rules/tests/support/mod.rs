//! Chess tests read like user scenarios. Support owns input gestures and host queries.
//!
//! The ordinary path constructs a Board directly, runs unchanged synchronous rules
//! inline, decodes real engine output, and advances only scheduled virtual events.
//! Nothing waits for a thread or repeatedly asks whether an assertion is true.
//! Assertions inspect visible prefab assets and accumulated transforms, never the
//! rules snapshot, component tree, internal IDs, or invisible status labels.
#![allow(dead_code)]
pub mod board;
pub mod fixtures;
pub mod game;
pub mod storage;
