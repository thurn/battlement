//! Typed synchronous game rules and immutable worker publications.

mod connection;
mod execution;
mod game;
mod publication;
mod response;
#[cfg(test)]
mod response_tests;
mod run;
mod session_context;
mod worker;
mod worker_observer;

#[cfg(feature = "platform-proof")]
#[doc(hidden)]
pub mod platform_proof;

pub use connection::DisplayConnection;
pub use execution::ExecutionMode;
pub use game::{ChoiceOwner, ChoicePolicy, Game, PromptData};
pub use publication::{Checkpoint, CheckpointParts, PublicationObservation};
pub use response::{PresentedPrompt, ResponseHandle};
pub use run::{RulesRun, RunObservation};
pub use session_context::{CompletedAction, RulesContext, RulesWorker};
