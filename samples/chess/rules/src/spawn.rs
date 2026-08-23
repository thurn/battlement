use battlement::{
    ActionId, Batch, BatchId, Command, CommandBody, GameObject, ParallelCommandGroup,
    ParticleSpawnLocation, ParticleSpawnPayload, SessionId, WaitPayload,
};
use fastrand::Rng;

use crate::{
    CRITICAL_BEAT_INTERVAL_MS, CRITICAL_FIRST_BEAT_OFFSET_MS, PIECE_SPAWN_BEAT_COUNT,
    PLAY_BUTTON_ID, assets::effects, audio,
};

const EFFECT_LIFETIME_MS: u64 = 1_000;

pub fn batch(
    session_id: SessionId,
    action_id: ActionId,
    pieces: [Vec<GameObject>; 2],
    interface_objects: [GameObject; 2],
    music: Command,
    enable_input_on_complete: bool,
    rng: &mut Rng,
) -> Batch<Command> {
    let [refresh_button, selected_effect] = interface_objects;
    let [mut white, mut black] = pieces;
    rng.shuffle(&mut white);
    rng.shuffle(&mut black);
    let maximum_side_size = white.len().max(black.len());
    let pieces_per_color_per_beat = maximum_side_size.div_ceil(PIECE_SPAWN_BEAT_COUNT).max(1);
    let stage_count = maximum_side_size.div_ceil(pieces_per_color_per_beat);
    let mut start = ParallelCommandGroup::from_bodies([
        CommandBody::object_destroy(PLAY_BUTTON_ID),
        CommandBody::object_create(refresh_button),
        CommandBody::object_create(selected_effect),
        CommandBody::set_input_enabled(false),
    ]);
    start
        .commands
        .push(Command::new_v4(audio::play_sound(audio::START_SOUND)).nonblocking());
    start.commands.push(music);
    let mut groups = vec![start];
    groups.push(ParallelCommandGroup::from_bodies([CommandBody::TimeWait(
        WaitPayload {
            duration_ms: CRITICAL_FIRST_BEAT_OFFSET_MS,
        },
    )]));

    for index in 0..stage_count {
        let range_start = index * pieces_per_color_per_beat;
        let range_end = range_start + pieces_per_color_per_beat;
        let objects = [
            white.get(range_start..range_end.min(white.len())),
            black.get(range_start..range_end.min(black.len())),
        ]
        .into_iter()
        .flatten()
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
                    address: effects::PIECE_SPAWN,
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
                    duration_ms: CRITICAL_BEAT_INTERVAL_MS,
                },
            )]));
        }
    }

    Batch::new(BatchId::new_v4(), session_id, groups).caused_by_action_id(action_id)
}
