mod support;

use std::sync::Arc;

use masonry::{
    Batch, CameraState, Command, CommandBody, GameObject, GameObjectKind, ParallelCommandGroup,
    PreparedAsset, Response, ResponseMessage, Scene, Snapshot, Vector3,
};
use masonry_fake::{assets::FakeAssetCatalog, client::FakeClient};
use support::ScriptedEngine;
use uuid::Uuid;

fn session(value: u128) -> masonry::SessionId {
    masonry::SessionId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn object(value: u128) -> masonry::ObjectId {
    masonry::ObjectId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn batch(value: u128) -> masonry::BatchId {
    masonry::BatchId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn command(value: u128) -> masonry::CommandId {
    masonry::CommandId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn catalog() -> Arc<FakeAssetCatalog> {
    let mut value = FakeAssetCatalog::new();
    value.add_scene("temporal/scene");
    value.add_texture("temporal/texture");
    value.add_audio_clip("temporal/audio");
    Arc::new(value)
}

fn initial(session_id: masonry::SessionId) -> Response {
    let mut snapshot = Snapshot::new(
        session_id,
        vec![
            PreparedAsset::Scene("temporal/scene".into()),
            PreparedAsset::Texture("temporal/texture".into()),
            PreparedAsset::AudioClip("temporal/audio".into()),
        ],
        vec![Scene::new(
            masonry::SceneId::from_uuid(Uuid::from_u128(10)).unwrap(),
            "temporal/scene",
        )],
        vec![
            GameObject::new(
                object(1),
                GameObjectKind::Camera {
                    camera: CameraState::default(),
                },
            ),
            GameObject::new(object(2), GameObjectKind::Cube { materials: vec![] }),
            GameObject::new(
                object(3),
                GameObjectKind::Image {
                    image: masonry::ImageState::new("temporal/texture", 1.0, 1.0),
                },
            ),
            GameObject::new(
                object(4),
                GameObjectKind::Light {
                    light: masonry::LightState::default(),
                },
            ),
        ],
        object(1),
    );
    snapshot.global_keys = Vec::new();
    Response::new(session_id, vec![ResponseMessage::Snapshot(snapshot)])
}

fn response(
    session_id: masonry::SessionId,
    id: u128,
    body: CommandBody,
    blocking: bool,
) -> Response {
    let command = if blocking {
        Command::new(command(id), body)
    } else {
        Command::new(command(id), body).nonblocking()
    };
    Response::new(
        session_id,
        vec![ResponseMessage::Batch(Batch::new(
            batch(id),
            session_id,
            vec![ParallelCommandGroup::new(vec![command])],
        ))],
    )
}

fn ping_pong(additional_traversals: u32) -> masonry::Tween {
    masonry::Tween {
        duration_ms: 1,
        repeat: masonry::TweenRepeat::Count {
            additional_traversals,
            mode: masonry::RepeatMode::PingPong,
        },
        ..masonry::Tween::default()
    }
}

#[test]
fn numeric_vector_color_audio_and_forever_tweens_collapse_immediately() {
    let session_id = session(1);
    let audio_id = command(105);
    let responses = vec![
        response(
            session_id,
            101,
            CommandBody::TransformTweenLocalPosition(masonry::PropertyCommand::canceling(
                masonry::TweenPositionPayload {
                    object_id: object(2),
                    position: Vector3::new(5.0, 0.0, 0.0),
                    tween: ping_pong(1),
                },
            )),
            true,
        ),
        response(
            session_id,
            102,
            CommandBody::TransformTweenLocalPosition(masonry::PropertyCommand::canceling(
                masonry::TweenPositionPayload {
                    object_id: object(2),
                    position: Vector3::new(5.0, 0.0, 0.0),
                    tween: ping_pong(2),
                },
            )),
            true,
        ),
        response(
            session_id,
            103,
            CommandBody::ImageTweenTint(masonry::PropertyCommand::canceling(
                masonry::TweenTintPayload {
                    object_id: object(3),
                    tint: masonry::RgbColor::BLACK,
                    tween: ping_pong(1),
                },
            )),
            true,
        ),
        response(
            session_id,
            104,
            CommandBody::LightTweenColor(masonry::PropertyCommand::canceling(
                masonry::TweenColorPayload {
                    object_id: object(4),
                    color: masonry::Color::BLACK,
                    tween: ping_pong(2),
                },
            )),
            true,
        ),
        response(
            session_id,
            105,
            CommandBody::AudioPlay(masonry::AudioPlayPayload {
                address: "temporal/audio".into(),
                volume: 0.25,
                pitch: 1.0,
                r#loop: false,
                fade_in_ms: 0,
            }),
            true,
        ),
        response(
            session_id,
            106,
            CommandBody::AudioTweenVolume(masonry::PropertyCommand::canceling(
                masonry::TweenAudioVolumePayload {
                    audio_command_id: audio_id,
                    volume: 1.0,
                    tween: ping_pong(1),
                },
            )),
            true,
        ),
        response(
            session_id,
            107,
            CommandBody::TransformTweenLocalPosition(masonry::PropertyCommand::canceling(
                masonry::TweenPositionPayload {
                    object_id: object(2),
                    position: Vector3::new(8.0, 0.0, 0.0),
                    tween: masonry::Tween {
                        duration_ms: 1,
                        repeat: masonry::TweenRepeat::Forever(masonry::RepeatMode::Restart),
                        ..masonry::Tween::default()
                    },
                },
            )),
            false,
        ),
        response(
            session_id,
            108,
            CommandBody::TransformTweenLocalPosition(masonry::PropertyCommand::canceling(
                masonry::TweenPositionPayload {
                    object_id: object(2),
                    position: Vector3::new(9.0, 0.0, 0.0),
                    tween: masonry::Tween {
                        duration_ms: 1,
                        repeat: masonry::TweenRepeat::Forever(masonry::RepeatMode::PingPong),
                        ..masonry::Tween::default()
                    },
                },
            )),
            false,
        ),
    ];
    let engine = ScriptedEngine::new([initial(session_id)], [], responses.into_iter().map(Some));
    let mut client = FakeClient::connect(engine, catalog());

    client.poll();
    client.assert_world_position(object(2), Vector3::ZERO, 0.0);
    client.poll();
    client.assert_world_position(object(2), Vector3::new(5.0, 0.0, 0.0), 0.0);
    client.poll();
    assert_eq!(
        client.assert_object(object(3)).kind(),
        &GameObjectKind::Image {
            image: masonry::ImageState::new("temporal/texture", 1.0, 1.0)
        }
    );
    client.poll();
    assert_eq!(
        client.assert_object(object(4)).light().unwrap().color,
        masonry::Color::BLACK
    );
    client.poll();
    assert_eq!(client.world().audio(audio_id).unwrap().volume(), 0.25);
    client.poll();
    assert_eq!(client.world().audio(audio_id).unwrap().volume(), 0.25);
    client.poll();
    client.assert_world_position(object(2), Vector3::new(8.0, 0.0, 0.0), 0.0);
    client.poll();
    client.assert_world_position(object(2), Vector3::new(9.0, 0.0, 0.0), 0.0);
}
