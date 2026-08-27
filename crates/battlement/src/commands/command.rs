use serde::{Deserialize, Serialize};

use crate::{
  CommandId, ConflictPolicy, ObjectId, UiElement, UiNode, VisualElementAction, VisualElementCreate,
  VisualElementDestroy, VisualElementPerformAction, VisualElementUpdate,
};

use super::CommandBody;

/// A fully typed Battlement core command.
///
/// `command_id` also identifies any asynchronous operation started by the
/// command. Commands are blocking by default; a nonblocking command lets its
/// batch advance while the operation continues.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Command {
  /// Identifier for the command and any operation it starts.
  pub command_id: CommandId,
  /// Whether later groups wait for this command to finish.
  #[serde(
    default = "crate::default_true",
    skip_serializing_if = "crate::is_true"
  )]
  pub blocking: bool,
  /// Exact core command type, conflict behavior, and payload.
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

  /// Creates a blocking command with a generated identity.
  #[must_use]
  pub fn new_v4(body: CommandBody) -> Self {
    Self::new(CommandId::new_v4(), body)
  }

  /// Marks this command as nonblocking and returns it.
  #[must_use]
  pub fn nonblocking(mut self) -> Self {
    self.blocking = false;
    self
  }

  /// Appends a UI subtree to a logical parent.
  #[must_use]
  pub fn create_visual_element(parent_id: ObjectId, node: UiNode) -> Self {
    Self::new_v4(CommandBody::VisualElementCreate(Box::new(
      VisualElementCreate::new(parent_id, node),
    )))
  }

  /// Applies sparse visual properties to one UI element.
  #[must_use]
  pub fn update_visual_element(object_id: ObjectId, element: impl Into<UiElement>) -> Self {
    Self::new_v4(CommandBody::VisualElementUpdate(Box::new(
      VisualElementUpdate::Properties {
        object_id,
        element: std::boxed::Box::new(element.into()),
      },
    )))
  }

  /// Moves one UI element beneath a different parent and appends it.
  #[must_use]
  pub fn update_visual_element_parent(object_id: ObjectId, parent_id: ObjectId) -> Self {
    Self::new_v4(CommandBody::VisualElementUpdate(Box::new(
      VisualElementUpdate::Parent {
        object_id,
        parent_id,
      },
    )))
  }

  /// Changes one UI element's index within its current parent.
  #[must_use]
  pub fn update_visual_element_index(object_id: ObjectId, child_index: u32) -> Self {
    Self::new_v4(CommandBody::VisualElementUpdate(Box::new(
      VisualElementUpdate::Index {
        object_id,
        child_index,
      },
    )))
  }

  /// Recursively destroys one UI element.
  #[must_use]
  pub fn destroy_visual_element(object_id: ObjectId) -> Self {
    Self::new_v4(CommandBody::VisualElementDestroy(VisualElementDestroy {
      object_id,
    }))
  }

  /// Performs one transient UI operation.
  #[must_use]
  pub fn perform_visual_element_action(object_id: ObjectId, action: VisualElementAction) -> Self {
    Self::new_v4(CommandBody::VisualElementPerformAction(
      VisualElementPerformAction { object_id, action },
    ))
  }
}

/// A property-writing core-command body.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PropertyCommand<P> {
  /// How to handle an operation already controlling the same canonical property.
  #[serde(default, skip_serializing_if = "crate::is_default")]
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

/// A custom game command using Battlement's shared command format.
///
/// The namespaced type and payload contract belong to the game's Rust types
/// rather than the Battlement core crate.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CustomCommand<P> {
  /// Session-unique command and operation identity.
  pub command_id: CommandId,
  /// Game-owned namespaced command type.
  pub command_type: String,
  /// Whether later groups wait for the custom handler's operation.
  #[serde(
    default = "crate::default_true",
    skip_serializing_if = "crate::is_true"
  )]
  pub blocking: bool,
  /// Game-specific payload.
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
/// payload is a game-owned Rust type.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum AnyCommand<P> {
  /// A command implemented by Battlement itself.
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
