use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{CommandId, ConflictPolicy};

use super::CommandBody;

/// A fully typed Masonry core command.
///
/// `command_id` also identifies any asynchronous operation started by the
/// command. Commands are blocking by default; a nonblocking command lets its
/// batch advance while the operation continues.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    /// Session-unique identity of the command and any operation it starts.
    pub command_id: CommandId,
    /// Whether later groups wait for this command to finish.
    #[serde(
        default = "crate::serialization::default_true",
        skip_serializing_if = "crate::serialization::is_true"
    )]
    pub blocking: bool,
    /// Exact core command type, conflict behavior, and payload.
    #[serde(flatten)]
    pub body: CommandBody,
}

impl Command {
    /// Creates a blocking command.
    #[must_use]
    pub fn new(command_id: CommandId, body: CommandBody) -> Self {
        Self {
            command_id,
            blocking: true,
            body,
        }
    }

    /// Marks this command as nonblocking and returns it.
    #[must_use]
    pub fn nonblocking(mut self) -> Self {
        self.blocking = false;
        self
    }
}

/// A core-command body that does not participate in property conflict handling.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct CommandPayload<P> {
    /// Command-specific payload.
    pub payload: P,
}

impl<P> From<P> for CommandPayload<P> {
    fn from(payload: P) -> Self {
        Self { payload }
    }
}

/// A property-writing core-command body.
///
/// Omitted conflict behavior means [`ConflictPolicy::Cancel`].
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PropertyCommand<P> {
    /// How to handle an operation already controlling the same canonical property.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub on_conflict: ConflictPolicy,
    /// Command-specific payload.
    pub payload: P,
}

impl<P> PropertyCommand<P> {
    /// Creates a property write that cancels conflicting work.
    #[must_use]
    pub fn canceling(payload: P) -> Self {
        Self {
            on_conflict: ConflictPolicy::Cancel,
            payload,
        }
    }

    /// Creates a property write that waits for conflicting work.
    #[must_use]
    pub fn waiting(payload: P) -> Self {
        Self {
            on_conflict: ConflictPolicy::Wait,
            payload,
        }
    }
}

/// A custom game command using Masonry's shared command format.
///
/// The namespaced type and payload contract belong to the game's Rust types
/// rather than the Masonry core crate.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CustomCommand<P = Value> {
    /// Session-unique command and operation identity.
    pub command_id: CommandId,
    /// Game-owned namespaced command discriminator.
    #[serde(rename = "type")]
    #[schemars(length(max = 65_536))]
    pub command_type: String,
    /// Whether later groups wait for the custom handler's operation.
    #[serde(
        default = "crate::serialization::default_true",
        skip_serializing_if = "crate::serialization::is_true"
    )]
    pub blocking: bool,
    /// Game-specific payload, raw JSON by default or a game-owned Rust type.
    pub payload: P,
}

impl<P> CustomCommand<P> {
    /// Creates a blocking custom command.
    #[must_use]
    pub fn new(command_id: CommandId, command_type: impl Into<String>, payload: P) -> Self {
        Self {
            command_id,
            command_type: command_type.into(),
            blocking: true,
            payload,
        }
    }

    /// Marks this custom command as nonblocking and returns it.
    #[must_use]
    pub fn nonblocking(mut self) -> Self {
        self.blocking = false;
        self
    }
}

/// A command list entry that may contain either core or game-specific work.
///
/// Use this as the command parameter of [`crate::Response`] when a rules engine
/// needs to mix core commands with registered custom commands. The custom
/// payload defaults to raw JSON but can be a game-owned Rust type.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(untagged)]
pub enum AnyCommand<P = Value> {
    /// A command implemented by Masonry itself.
    Core(Command),
    /// A command handled by registered game code.
    Custom(CustomCommand<P>),
}

impl<P> From<Command> for AnyCommand<P> {
    fn from(command: Command) -> Self {
        Self::Core(command)
    }
}

impl<P> From<CustomCommand<P>> for AnyCommand<P> {
    fn from(command: CustomCommand<P>) -> Self {
        Self::Custom(command)
    }
}
