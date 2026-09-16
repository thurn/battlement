//! Typed synchronous game rules and immutable worker publications.

mod connection;
mod execution;
mod game;
mod publication;
mod run;
mod worker;

#[cfg(feature = "platform-proof")]
#[doc(hidden)]
pub mod platform_proof;

pub use connection::DisplayConnection;
pub use execution::ExecutionMode;
pub use game::{ChoiceOwner, ChoicePolicy, Game, PromptData};
pub use publication::{Checkpoint, PublicationObservation};
pub use run::{RulesRun, RunObservation};
