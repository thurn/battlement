//! Command history retained by the fake client.

use masonry::{BatchId, Command, CommandId, SessionId};

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
