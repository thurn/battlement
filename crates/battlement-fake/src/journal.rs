//! Command history retained by the fake client.

use battlement::{BatchId, Command, CommandId, SessionId};

/// An opaque position in a fake client's command journal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandCheckpoint {
    pub(crate) length: usize,
}

impl CommandCheckpoint {
    pub(crate) fn new(length: usize) -> Self {
        Self { length }
    }
}

/// One command that completed in the fake client.
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutedCommand {
    /// Session in which the command ran.
    pub session_id: SessionId,
    /// Batch that contained the command.
    pub batch_id: BatchId,
    /// Zero-based group position within the batch.
    pub group_index: usize,
    /// Zero-based command position within the group.
    pub command_index: usize,
    /// The original command value.
    pub command: Command,
}

impl ExecutedCommand {
    /// Returns the command identity recorded in this journal entry.
    #[must_use]
    pub fn command_id(&self) -> CommandId {
        self.command.command_id
    }
}
