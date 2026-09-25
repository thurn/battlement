mod support;

use std::{sync::Arc, time::Duration};

use battlement::{
  Batch, CameraState, Command, CommandBody, GameObject, GameObjectKind, ParallelCommandGroup,
  PreparedAsset, Response, ResponseMessage, Scene, Snapshot, Vector3,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use support::ScriptedEngine;
use uuid::Uuid;

fn session(value: u128) -> battlement::SessionId {
  battlement::SessionId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn object(value: u128) -> battlement::ObjectId {
  battlement::ObjectId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn batch(value: u128) -> battlement::BatchId {
  battlement::BatchId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn command(value: u128) -> battlement::CommandId {
  battlement::CommandId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn catalog() -> Arc<FakeAssetCatalog> {
  let mut value = FakeAssetCatalog::new();
  value.add_scene("temporal/scene");
  value.add_audio_clip("temporal/audio");
  value.add_particle_effect("temporal/particles");
  Arc::new(value)
}

fn initial(session_id: battlement::SessionId) -> Response {
  let snapshot = Snapshot::new(
    session_id,
    vec![
      PreparedAsset::Scene("temporal/scene".into()),
      PreparedAsset::AudioClip("temporal/audio".into()),
      PreparedAsset::ParticleEffect("temporal/particles".into()),
    ],
    vec![Scene::new(
      battlement::SceneId::from_uuid(Uuid::from_u128(10)).unwrap(),
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
    ],
    object(1),
  );
  Response::new(session_id, vec![ResponseMessage::Snapshot(snapshot)])
}

fn response(
  session_id: battlement::SessionId,
  batch_id: u128,
  groups: Vec<Vec<Command>>,
) -> Response {
  Response::new(
    session_id,
    vec![ResponseMessage::Batch(Batch::new(
      batch(batch_id),
      session_id,
      groups.into_iter().map(ParallelCommandGroup::new).collect(),
    ))],
  )
}

#[test]
fn finite_tween_exposes_its_midpoint_and_settles_to_its_endpoint() {
  let session_id = session(1);
  let tween = Command::new(
    command(101),
    CommandBody::TransformTweenWorldPosition(battlement::PropertyCommand::canceling(
      battlement::TweenPositionPayload {
        object_id: object(2),
        position: Vector3::new(10.0, 0.0, 0.0),
        tween: battlement::Tween {
          duration_ms: 250,
          easing: battlement::Easing::Linear,
          ..battlement::Tween::default()
        },
      },
    )),
  );
  let engine = ScriptedEngine::new(
    [initial(session_id)],
    [],
    [Some(response(session_id, 101, vec![vec![tween]]))],
  );
  let mut client = FakeClient::connect(engine, catalog());

  client.poll();
  client.assert_world_position(object(2), Vector3::ZERO, 0.0);
  client.advance_time(Duration::from_millis(125));
  client.assert_world_position(object(2), Vector3::new(5.0, 0.0, 0.0), 0.0);
  assert_eq!(client.frame(), 0);

  client.settle();
  client.assert_world_position(object(2), Vector3::new(10.0, 0.0, 0.0), 0.0);
  assert_eq!(client.presentation_time(), Duration::from_millis(250));
}

#[test]
fn tween_delay_and_ping_pong_repetition_follow_the_virtual_timeline() {
  let session_id = session(5);
  let tween = Command::new(
    command(501),
    CommandBody::TransformTweenLocalPosition(battlement::PropertyCommand::canceling(
      battlement::TweenPositionPayload {
        object_id: object(2),
        position: Vector3::new(10.0, 0.0, 0.0),
        tween: battlement::Tween {
          delay_ms: 25,
          duration_ms: 50,
          easing: battlement::Easing::Linear,
          repeat: battlement::TweenRepeat::Count {
            additional_traversals: 1,
            mode: battlement::RepeatMode::PingPong,
          },
        },
      },
    )),
  );
  let engine = ScriptedEngine::new(
    [initial(session_id)],
    [],
    [Some(response(session_id, 501, vec![vec![tween]]))],
  );
  let mut client = FakeClient::connect(engine, catalog());

  client.poll();
  client.advance_time(Duration::from_millis(25));
  client.assert_world_position(object(2), Vector3::ZERO, 0.0);
  client.advance_time(Duration::from_millis(50));
  client.assert_world_position(object(2), Vector3::new(10.0, 0.0, 0.0), 0.0);
  client.advance_time(Duration::from_millis(25));
  client.assert_world_position(object(2), Vector3::new(5.0, 0.0, 0.0), 0.0);
  client.settle();
  client.assert_world_position(object(2), Vector3::ZERO, 0.0);
}

#[test]
fn blocking_wait_delays_its_successor_until_the_exact_deadline() {
  let session_id = session(2);
  let engine = ScriptedEngine::new(
    [initial(session_id)],
    [],
    [Some(response(
      session_id,
      201,
      vec![
        vec![Command::new(
          command(201),
          CommandBody::TimeWait(battlement::WaitPayload { duration_ms: 100 }),
        )],
        vec![Command::new(
          command(202),
          CommandBody::ObjectSetActive(battlement::ObjectSetActivePayload {
            object_id: object(2),
            active: false,
          }),
        )],
      ],
    ))],
  );
  let mut client = FakeClient::connect(engine, catalog());

  client.poll();
  assert!(client.assert_object(object(2)).active_self());
  client.advance_time(Duration::from_millis(99));
  assert!(client.assert_object(object(2)).active_self());
  client.advance_time(Duration::from_millis(1));
  assert!(!client.assert_object(object(2)).active_self());
}

#[test]
fn duplicate_delivery_records_one_audio_and_particle_occurrence() {
  let session_id = session(3);
  let audio = Command::new(
    command(301),
    CommandBody::AudioPlay(battlement::AudioPlayPayload {
      bus: battlement::AudioBus::Effects,
      address: "temporal/audio".into(),
      volume: 0.25,
      pitch: 1.0,
      r#loop: false,
      fade_in_ms: 0,
    }),
  );
  let particle = Command::new(
    command(302),
    CommandBody::ParticleSpawn(battlement::ParticleSpawnPayload {
      address: "temporal/particles".into(),
      location: battlement::ParticleSpawnLocation::WorldPosition(Vector3::new(1.0, 2.0, 3.0)),
      lifetime_ms: 500,
    }),
  );
  let delivered = response(session_id, 301, vec![vec![audio, particle]]);
  let engine = ScriptedEngine::new(
    [initial(session_id)],
    [],
    [Some(delivered.clone()), Some(delivered)],
  );
  let mut client = FakeClient::connect(engine, catalog());

  client.poll();
  client.poll();

  assert_eq!(client.audio_occurrences().len(), 1);
  assert_eq!(client.audio_occurrences()[0].command_id, command(301));
  assert_eq!(
    client.audio_occurrences()[0].address.as_str(),
    "temporal/audio"
  );
  assert_eq!(client.particle_occurrences().len(), 1);
  assert_eq!(client.particle_occurrences()[0].command_id, command(302));
  assert_eq!(
    client.particle_occurrences()[0].address.as_str(),
    "temporal/particles"
  );
}

#[test]
fn frame_advance_and_finite_settle_leave_infinite_cosmetic_work_running() {
  let session_id = session(4);
  let tween = Command::new(
    command(401),
    CommandBody::TransformTweenLocalPosition(battlement::PropertyCommand::canceling(
      battlement::TweenPositionPayload {
        object_id: object(2),
        position: Vector3::new(8.0, 0.0, 0.0),
        tween: battlement::Tween {
          duration_ms: 1_000,
          easing: battlement::Easing::Linear,
          repeat: battlement::TweenRepeat::Forever(battlement::RepeatMode::PingPong),
          ..battlement::Tween::default()
        },
      },
    )),
  )
  .nonblocking();
  let engine = ScriptedEngine::new(
    [initial(session_id)],
    [],
    [Some(response(session_id, 401, vec![vec![tween]]))],
  );
  let mut client = FakeClient::connect(engine, catalog());

  client.poll();
  client.advance_time(Duration::from_millis(250));
  client.assert_world_position(object(2), Vector3::new(2.0, 0.0, 0.0), 0.0);
  client.advance_frame();
  assert_eq!(client.frame(), 1);
  assert_eq!(client.presentation_time(), Duration::from_millis(250));
  client.settle();
  client.assert_world_position(object(2), Vector3::new(2.0, 0.0, 0.0), 0.0);
}

#[test]
fn shared_audio_gains_update_live_and_future_sources_without_replaying_them() {
  let session_id = session(5);
  let play = |id, bus, volume| {
    Command::new(
      command(id),
      CommandBody::AudioPlay(battlement::AudioPlayPayload {
        address: "temporal/audio".into(),
        bus,
        volume,
        pitch: 1.0,
        r#loop: true,
        fade_in_ms: 0,
      }),
    )
    .nonblocking()
  };
  let mix = |id, muted| {
    Command::new(
      command(id),
      CommandBody::AudioSetMix(battlement::AudioMix {
        master: 0.5,
        music: 0.25,
        effects: 0.8,
        muted,
      }),
    )
  };
  let engine = ScriptedEngine::new(
    [initial(session_id)],
    [],
    [
      Some(response(
        session_id,
        501,
        vec![vec![
          mix(501, false),
          play(502, battlement::AudioBus::Music, 0.8),
          play(503, battlement::AudioBus::Effects, 0.5),
        ]],
      )),
      Some(response(
        session_id,
        504,
        vec![vec![
          mix(504, true),
          play(505, battlement::AudioBus::Music, 1.0),
        ]],
      )),
      Some(response(session_id, 506, vec![vec![mix(506, false)]])),
    ],
  );
  let mut client = FakeClient::connect(engine, catalog());
  client.poll();
  assert_eq!(
    client.world().audio(command(502)).unwrap().output_volume(),
    0.1
  );
  assert_eq!(
    client.world().audio(command(503)).unwrap().output_volume(),
    0.2
  );
  client.poll();
  for id in [502, 503, 505] {
    assert_eq!(
      client.world().audio(command(id)).unwrap().output_volume(),
      0.0
    );
  }
  client.poll();
  assert_eq!(client.world().audio(command(502)).unwrap().volume(), 0.8);
  assert_eq!(
    client.world().audio(command(502)).unwrap().output_volume(),
    0.1
  );
  assert_eq!(
    client.world().audio(command(505)).unwrap().output_volume(),
    0.125
  );
  assert_eq!(client.audio_occurrences().len(), 3);
  assert_eq!(
    client.world().audio(command(505)).unwrap().bus(),
    battlement::AudioBus::Music
  );
}
