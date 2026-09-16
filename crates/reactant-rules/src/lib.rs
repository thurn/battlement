//! Typed synchronous game rules and immutable worker publications.

mod connection;
mod execution;
mod game;
mod publication;
mod response;
#[cfg(test)]
mod response_tests;
mod run;
mod worker;

#[cfg(feature = "platform-proof")]
#[doc(hidden)]
pub mod platform_proof;

pub use connection::DisplayConnection;
pub use execution::ExecutionMode;
pub use game::{ChoiceOwner, ChoicePolicy, Game, PromptData};
pub use publication::{Checkpoint, PublicationObservation};
pub use response::{PresentedPrompt, ResponseHandle};
pub use run::{RulesRun, RunObservation};
