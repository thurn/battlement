use fastrand::Rng;
use masonry::{
    ActionId, Batch, BatchId, Command, CommandBody, GameObject, ParallelCommandGroup,
    ParticleEffectAddress, ParticleSpawnLocation, ParticleSpawnPayload, SessionId, WaitPayload,
};

use crate::{PIECE_SPAWN_EFFECT, PIECE_SPAWN_SEQUENCE_DURATION_MS, PLAY_BUTTON_ID, audio};

const EFFECT_LIFETIME_MS: u64 = 1_000;

pub fn batch(
    session_id: SessionId,
    action_id: ActionId,
    mut white: Vec<GameObject>,
    mut black: Vec<GameObject>,
    refresh_button: GameObject,
    enable_input_on_complete: bool,
    rng: &mut Rng,
) -> Batch<Command> {
    rng.shuffle(&mut white);
    rng.shuffle(&mut black);
    let stage_count = white.len().max(black.len());
    let interval_ms =
        PIECE_SPAWN_SEQUENCE_DURATION_MS / stage_count.saturating_sub(1).max(1) as u64;
    let mut start = ParallelCommandGroup::from_bodies([
        CommandBody::object_destroy(PLAY_BUTTON_ID),
        CommandBody::object_create(refresh_button),
        CommandBody::set_input_enabled(false),
    ]);
    start.commands.push(
        Command::new_v4(audio::play_sound(audio::START_SOUND)).nonblocking(),
    );
    let mut groups = vec![start];

    for index in 0..stage_count {
        let objects = [white.get(index), black.get(index)]
            .into_iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        let object_ids = objects
            .iter()
            .map(|object| object.object_id)
            .collect::<Vec<_>>();
        groups.push(ParallelCommandGroup::from_bodies(
            objects.into_iter().map(CommandBody::object_create),
        ));
        let mut effects = object_ids
            .into_iter()
            .map(|object_id| {
                Command::new_v4(CommandBody::ParticleSpawn(ParticleSpawnPayload {
                    address: ParticleEffectAddress::new(PIECE_SPAWN_EFFECT),
                    location: ParticleSpawnLocation::GameObject(object_id),
                    lifetime_ms: EFFECT_LIFETIME_MS,
                }))
                .nonblocking()
            })
            .collect::<Vec<_>>();
        if index + 1 == stage_count && enable_input_on_complete {
            effects.push(Command::new_v4(CommandBody::set_input_enabled(true)));
        }
        groups.push(ParallelCommandGroup::new(effects));
        if index + 1 < stage_count {
            groups.push(ParallelCommandGroup::from_bodies([CommandBody::TimeWait(
                WaitPayload {
                    duration_ms: interval_ms,
                },
            )]));
        }
    }

    Batch::new(BatchId::new_v4(), session_id, groups).caused_by_action_id(action_id)
}
