//! Group and opaque-prefab lowering through the shared host interface.

use battlement::{
  Command, CommandBody, GameObject, GameObjectKind, LocalTransform, ObjectId,
  ObjectReparentPayload, ObjectSetActivePayload, ParentScene, PointerEvent, PointerEventsPayload,
  PositionPayload, PropertyCommand, RotationPayload, ScalePayload,
};
use reactant_core::host_node::{HostAdapter, HostNode};

#[derive(Clone)]
pub(crate) struct WorldDescription {
  pub(crate) kind: GameObjectKind,
  pub(crate) scene: ParentScene,
  pub(crate) root: bool,
  pub(crate) transform: LocalTransform,
  pub(crate) active: bool,
  pub(crate) clickable: bool,
}

pub(crate) struct WorldAdapter;

impl HostAdapter for WorldAdapter {
  type Description = WorldDescription;

  fn requires_remount(previous: &WorldDescription, desired: &WorldDescription) -> bool {
    let attachment_changed = previous.root && previous.scene != desired.scene;
    previous.kind != desired.kind || previous.root != desired.root || attachment_changed
  }

  fn create_command(node: &HostNode, parent: ObjectId, _: u32) -> Command {
    Command::new_v4(CommandBody::object_create(
      Self::object(node.object_id, node.description::<Self>(), Some(parent)).expect("world object"),
    ))
  }

  fn property_command(
    _: ObjectId,
    _: &WorldDescription,
    _: &WorldDescription,
    _: bool,
  ) -> Option<Command> {
    None
  }

  fn property_commands(
    object_id: ObjectId,
    previous: &WorldDescription,
    desired: &WorldDescription,
    _: bool,
  ) -> Vec<Command> {
    let mut bodies = Vec::new();
    if previous.transform.position != desired.transform.position {
      bodies.push(CommandBody::TransformSetLocalPosition(
        PropertyCommand::canceling(PositionPayload {
          object_id,
          position: desired.transform.position,
        }),
      ));
    }
    if previous.transform.rotation != desired.transform.rotation {
      bodies.push(CommandBody::TransformSetLocalRotation(
        PropertyCommand::canceling(RotationPayload {
          object_id,
          rotation: desired.transform.rotation,
        }),
      ));
    }
    if previous.transform.scale != desired.transform.scale {
      bodies.push(CommandBody::TransformSetLocalScale(
        PropertyCommand::canceling(ScalePayload {
          object_id,
          scale: desired.transform.scale,
        }),
      ));
    }
    if previous.active != desired.active {
      bodies.push(CommandBody::ObjectSetActive(ObjectSetActivePayload {
        object_id,
        active: desired.active,
      }));
    }
    if previous.clickable != desired.clickable {
      bodies.push(CommandBody::InputSetPointerEvents(PointerEventsPayload {
        object_id,
        events: Self::events(desired),
      }));
    }
    bodies.into_iter().map(Command::new_v4).collect()
  }

  fn move_command(object_id: ObjectId, parent_id: ObjectId, _: u32) -> Command {
    Command::new_v4(CommandBody::ObjectReparent(ObjectReparentPayload {
      object_id,
      parent_id: Some(parent_id),
      world_position_stays: false,
    }))
  }

  fn index_command(_: ObjectId, _: u32) -> Option<Command> {
    None
  }
  fn destroy_command(object_id: ObjectId) -> Command {
    Command::new_v4(CommandBody::object_destroy(object_id))
  }
  fn constrains_children(_: &WorldDescription) -> bool {
    false
  }
  fn creates_children() -> bool {
    false
  }
  fn scene_root(description: &WorldDescription) -> bool {
    description.root
  }
  fn inert(description: &mut WorldDescription) {
    description.clickable = false;
  }
  fn hide(description: &mut WorldDescription) {
    description.active = false;
  }

  fn object(
    object_id: ObjectId,
    description: &WorldDescription,
    parent: Option<ObjectId>,
  ) -> Option<GameObject> {
    let mut object = GameObject::new(object_id, description.kind.clone());
    object.parent_scene = description.scene;
    object.parent_id = if description.root { None } else { parent };
    object.local_transform = description.transform;
    object.active = description.active;
    object.pointer_events = Self::events(description);
    Some(object)
  }
}

impl WorldAdapter {
  fn events(description: &WorldDescription) -> Vec<PointerEvent> {
    if description.clickable {
      vec![PointerEvent::Click]
    } else {
      Vec::new()
    }
  }
}
