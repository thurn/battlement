//! Core command formats and payloads.

mod animation;
mod body;
mod camera_lighting;
mod command;
mod control;
mod effects;
mod image_text;
mod world;

pub use animation::*;
pub use body::*;
pub use camera_lighting::*;
pub use command::*;
pub use control::*;
pub use effects::*;
pub use image_text::*;
pub use world::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Tween, Vector3};
    use serde_json::json;

    #[test]
    fn command_serializes_with_a_flat_namespaced_discriminator() {
        let command_id = "7bbcb27e-f75b-4c63-bf86-ad1b0f6ee2cd".parse().unwrap();
        let object_id = "cc847d6e-1468-42c6-9bec-9af5b5aa5c03".parse().unwrap();
        let command = Command::new(
            command_id,
            CommandBody::TransformTweenWorldPosition(PropertyCommand::canceling(
                TweenPositionPayload {
                    object_id,
                    position: Vector3::new(4.0, 0.0, 2.0),
                    tween: Tween {
                        duration_ms: 300,
                        ..Tween::default()
                    },
                },
            )),
        );

        assert_eq!(
            serde_json::to_value(command).unwrap(),
            json!({
                "commandId": "7bbcb27e-f75b-4c63-bf86-ad1b0f6ee2cd",
                "type": "masonry.transform.tweenWorldPosition",
                "payload": {
                    "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
                    "position": { "x": 4.0, "y": 0.0, "z": 2.0 },
                    "durationMs": 300
                }
            })
        );
    }

    #[test]
    fn wait_conflict_policy_is_explicit_on_the_wire() {
        let command_id = "565e76aa-b480-43c2-900b-1cb9d90e4602".parse().unwrap();
        let object_id = "cc847d6e-1468-42c6-9bec-9af5b5aa5c03".parse().unwrap();
        let command = Command::new(
            command_id,
            CommandBody::TransformSetLocalScale(PropertyCommand::waiting(ScalePayload {
                object_id,
                scale: Vector3::ONE,
            })),
        );
        let value = serde_json::to_value(command).unwrap();

        assert_eq!(value["onConflict"], "wait");
    }
}
