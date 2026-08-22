use std::time::{Duration, Instant};

use fastrand::Rng;
use masonry::{
    ActionId, AudioPlayPayload, AudioStopPayload, AudioVolumePayload, Batch, BatchId, Command,
    CommandBody, CommandId, ParallelCommandGroup, PropertyCommand, Response, SessionId,
};

use crate::MUSIC_TRACKS;

const MUSIC_TRACK_DURATION: Duration = Duration::from_secs(120);
const MUSIC_CROSSFADE_MS: u64 = 5_000;
const DEFAULT_MUSIC_VOLUME: f64 = 0.35;
const SOUND_EFFECT_VOLUME: f64 = 0.8;

pub(crate) const PICKUP_SOUNDS: [&str; 4] = [
    "chess/sfx/click",
    "chess/sfx/click-2",
    "chess/sfx/click-3",
    "chess/sfx/click-4",
];
pub(crate) const CAPTURE_SOUNDS: [&str; 4] = [
    "chess/sfx/attack-a",
    "chess/sfx/attack-b",
    "chess/sfx/attack-c",
    "chess/sfx/attack-d",
];
pub(crate) const DROP_SOUNDS: [&str; 4] = [
    "chess/sfx/bounce-0",
    "chess/sfx/bounce-1",
    "chess/sfx/bounce-2",
    "chess/sfx/bounce-3",
];
pub(crate) const INVALID_DROP_SOUND: &str = "chess/sfx/error";
pub(crate) const CASTLE_SOUND: &str = "chess/sfx/powerup-a";
pub(crate) const PROMOTION_SOUND: &str = "chess/sfx/powerup-b";
pub(crate) const CHECK_SOUND: &str = "chess/sfx/alarm";
pub(crate) const START_SOUND: &str = "chess/sfx/accept";
pub(crate) const RESET_SOUND: &str = "chess/sfx/scene-transition";
pub(crate) const VOLUME_UP_SOUND: &str = "chess/sfx/chirp-a";
pub(crate) const VOLUME_DOWN_SOUND: &str = "chess/sfx/chirp-crunch";
pub(crate) const PLAYER_WIN_SOUND: &str = "chess/sfx/lap-complete";
pub(crate) const PLAYER_LOSS_SOUND: &str = "chess/sfx/fall-and-die";
pub(crate) const DRAW_SOUND: &str = "chess/sfx/wobble-falling-tone";

/// Addresses of NotJam's sound-effect collection.
pub const SOUND_EFFECTS: [&str; 41] = [
    "chess/sfx/accept",
    "chess/sfx/alarm",
    "chess/sfx/attack-a",
    "chess/sfx/attack-b",
    "chess/sfx/attack-c",
    "chess/sfx/attack-d",
    "chess/sfx/bleep-white-noise",
    "chess/sfx/boost-pad",
    "chess/sfx/bounce-0",
    "chess/sfx/bounce-1",
    "chess/sfx/bounce-2",
    "chess/sfx/bounce-3",
    "chess/sfx/chirp-a",
    "chess/sfx/chirp-crunch",
    "chess/sfx/chirp-white-noise",
    "chess/sfx/click",
    "chess/sfx/click-2",
    "chess/sfx/click-3",
    "chess/sfx/click-4",
    "chess/sfx/crunch-a",
    "chess/sfx/crunch-b",
    "chess/sfx/dash",
    "chess/sfx/error",
    "chess/sfx/exit-scene-transition",
    "chess/sfx/fall-and-die",
    "chess/sfx/grapple",
    "chess/sfx/green-light-tone",
    "chess/sfx/lap-complete",
    "chess/sfx/lockon-available",
    "chess/sfx/powerup-a",
    "chess/sfx/powerup-b",
    "chess/sfx/powerup-cursed",
    "chess/sfx/red-light-tone",
    "chess/sfx/rising-metallic",
    "chess/sfx/rising-tone-explosion",
    "chess/sfx/rising-tone-metallic",
    "chess/sfx/scene-transition",
    "chess/sfx/siren-explosion",
    "chess/sfx/slingshot",
    "chess/sfx/swipe-metallic",
    "chess/sfx/wobble-falling-tone",
];

pub(crate) struct MusicPlaylist {
    active: Option<CommandId>,
    track_index: usize,
    transition_due: Option<Instant>,
    volume: f64,
}

impl MusicPlaylist {
    pub(crate) fn new() -> Self {
        Self {
            active: None,
            track_index: 0,
            transition_due: None,
            volume: DEFAULT_MUSIC_VOLUME,
        }
    }

    pub(crate) fn poll(
        &mut self,
        session_id: SessionId,
        now: Instant,
    ) -> Option<Response<Command>> {
        let due = self.transition_due?;
        if now < due {
            return None;
        }

        let previous = self.active;
        if previous.is_some() {
            self.track_index = (self.track_index + 1) % MUSIC_TRACKS.len();
        }
        let active = CommandId::new_v4();
        self.active = Some(active);
        self.transition_due = Some(now + MUSIC_TRACK_DURATION);

        let mut commands =
            vec![Command::new(active, self.play_body(previous.is_some())).nonblocking()];
        if let Some(previous) = previous {
            commands.push(
                Command::new_v4(CommandBody::AudioStop(AudioStopPayload {
                    audio_command_id: previous,
                    fade_out_ms: MUSIC_CROSSFADE_MS,
                }))
                .nonblocking(),
            );
        }
        Some(Response::batch(Batch::new(
            BatchId::new_v4(),
            session_id,
            vec![ParallelCommandGroup::new(commands)],
        )))
    }

    pub(crate) fn reset(&mut self, now: Instant) {
        self.active = None;
        self.track_index = 0;
        self.transition_due = Some(now);
        self.volume = DEFAULT_MUSIC_VOLUME;
    }

    pub(crate) fn set_volume(&mut self, volume: f64) -> Option<CommandBody> {
        self.volume = volume.clamp(0.0, 1.0);
        self.active.map(|active| {
            CommandBody::AudioSetVolume(PropertyCommand::canceling(
                AudioVolumePayload {
                    audio_command_id: active,
                    volume: self.volume,
                },
            ))
        })
    }

    pub(crate) fn volume(&self) -> f64 {
        self.volume
    }

    fn play_body(&self, fade_in: bool) -> CommandBody {
        CommandBody::AudioPlay(AudioPlayPayload {
            address: MUSIC_TRACKS[self.track_index].into(),
            volume: self.volume,
            pitch: 1.0,
            r#loop: true,
            fade_in_ms: if fade_in { MUSIC_CROSSFADE_MS } else { 0 },
        })
    }
}

pub(crate) fn parallel_group(
    bodies: impl IntoIterator<Item = CommandBody>,
) -> ParallelCommandGroup {
    ParallelCommandGroup::new(
        bodies
            .into_iter()
            .map(|body| {
                let command = Command::new_v4(body);
                if matches!(&command.body, CommandBody::AudioPlay(_)) {
                    command.nonblocking()
                } else {
                    command
                }
            })
            .collect(),
    )
}

pub(crate) fn play_sound(address: &str) -> CommandBody {
    CommandBody::AudioPlay(AudioPlayPayload {
        address: address.into(),
        volume: SOUND_EFFECT_VOLUME,
        pitch: 1.0,
        r#loop: false,
        fade_in_ms: 0,
    })
}

pub(crate) fn random_sound<'a>(rng: &mut Rng, sounds: &'a [&str]) -> &'a str {
    sounds[rng.usize(..sounds.len())]
}

pub(crate) fn response_for_action(
    session_id: SessionId,
    action_id: ActionId,
    bodies: impl IntoIterator<Item = CommandBody>,
) -> Response<Command> {
    Response::batch(
        Batch::new(
            BatchId::new_v4(),
            session_id,
            vec![self::parallel_group(bodies)],
        )
        .caused_by_action_id(action_id),
    )
}
