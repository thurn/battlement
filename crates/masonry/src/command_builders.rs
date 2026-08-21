//! Fluent configuration methods for commands with constructor defaults.

use crate::{Command, CommandBody, ConflictPolicy, CustomCommand, PropertyCommand};

impl Command {
    /// Sets whether the command blocks its group and returns the updated command.
    #[must_use]
    pub fn blocking(mut self, value: bool) -> Self {
        self.blocking = value;
        self
    }

    /// Replaces the command body and returns the updated command.
    #[must_use]
    pub fn body(mut self, value: CommandBody) -> Self {
        self.body = value;
        self
    }
}

impl<P> PropertyCommand<P> {
    /// Sets the conflict policy and returns the updated command.
    #[must_use]
    pub fn on_conflict(mut self, value: ConflictPolicy) -> Self {
        self.on_conflict = value;
        self
    }
}

impl<P> CustomCommand<P> {
    /// Replaces the custom command type and returns the updated command.
    #[must_use]
    pub fn command_type(mut self, value: impl Into<String>) -> Self {
        self.command_type = value.into();
        self
    }

    /// Sets whether the command blocks its group and returns the updated command.
    #[must_use]
    pub fn blocking(mut self, value: bool) -> Self {
        self.blocking = value;
        self
    }
}
