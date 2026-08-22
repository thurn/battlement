use serde::{Deserialize, Serialize};

use crate::{
    GameObject, MaterialAddress, ObjectId, PreparedAsset, Quaternion, SceneAddress, SceneId, Tween,
    Vector3,
};

/// Atomically replaces the complete prepared asset set.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ReplaceAssetSetPayload {
    /// Complete replacement set; addresses must be unique.
    pub assets: Vec<PreparedAsset>,
}

/// Loads one prepared scene additively.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SceneLoadPayload {
    /// New session-unique scene instance identity.
    pub scene_id: SceneId,
    /// Prepared Addressables scene address.
    pub address: SceneAddress,
    /// Whether to make the loaded scene primary after it is ready.
    pub make_primary: bool,
}

/// A payload that names one loaded content scene.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct SceneIdPayload {
    /// Target scene identity.
    pub scene_id: SceneId,
}

/// Creates one complete game-object record.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ObjectCreatePayload {
    /// Complete object to create.
    pub object: GameObject,
}

/// A payload that names one game object.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ObjectIdPayload {
    /// Target game object.
    pub object_id: ObjectId,
}

/// Sets a game object's Unity activation state.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ObjectSetActivePayload {
    /// Target game object.
    pub object_id: ObjectId,
    /// New `activeSelf` value passed to `GameObject.SetActive`.
    ///
    /// A true value does not guarantee `activeInHierarchy` when a parent is
    /// inactive. This does not change component `enabled` flags or Unity's
    /// active Scene.
    pub active: bool,
}

/// Reparents a game object within its current placement.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ObjectReparentPayload {
    /// Game object to reparent.
    pub object_id: ObjectId,
    /// New game-object parent, or `null` for the placement container.
    pub parent_id: Option<ObjectId>,
    /// Whether Unity preserves the object's current world transform.
    pub world_position_stays: bool,
}

/// Sets a game object's position immediately.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct PositionPayload {
    /// Target game object.
    pub object_id: ObjectId,
    /// Requested local or world position, according to the command type.
    pub position: Vector3,
}

/// Tweens a game object's position.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TweenPositionPayload {
    /// Target game object.
    pub object_id: ObjectId,
    /// Requested final local or world position.
    pub position: Vector3,
    /// Tween timing and repetition.
    pub tween: Tween,
}

/// Sets a game object's rotation immediately.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct RotationPayload {
    /// Target game object.
    pub object_id: ObjectId,
    /// Requested local or world rotation, according to the command type.
    pub rotation: Quaternion,
}

/// Tweens a game object's rotation along the normalized shortest arc.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TweenRotationPayload {
    /// Target game object.
    pub object_id: ObjectId,
    /// Requested final local or world rotation.
    pub rotation: Quaternion,
    /// Tween timing and repetition.
    pub tween: Tween,
}

/// Sets a game object's local scale immediately.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ScalePayload {
    /// Target game object.
    pub object_id: ObjectId,
    /// Requested local scale.
    pub scale: Vector3,
}

/// Tweens a game object's local scale.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TweenScalePayload {
    /// Target game object.
    pub object_id: ObjectId,
    /// Requested final local scale.
    pub scale: Vector3,
    /// Tween timing and repetition.
    pub tween: Tween,
}

/// Assigns one prepared material to a supported renderer.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SetMaterialPayload {
    /// Target primitive or prefab game object.
    pub object_id: ObjectId,
    /// Prepared material address.
    pub address: MaterialAddress,
    /// Zero-based renderer slot, or every renderer slot when [`None`].
    pub slot: Option<u32>,
}

/// Enables or disables a supported component or billboard behavior.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ObjectEnabledPayload {
    /// Target game object.
    pub object_id: ObjectId,
    /// New enabled state.
    pub enabled: bool,
}
