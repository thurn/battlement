use std::time::{Duration, Instant};

use fastrand::Rng;
use masonry::{
    ActionId, AudioClipAddress, AudioPlayPayload, AudioStopPayload, AudioVolumePayload, Batch,
    BatchId, Command, CommandBody, CommandId, ParallelCommandGroup, PropertyCommand, Response,
    SessionId,
};

use crate::{MUSIC_TRACKS, assets::sfx};

const MUSIC_TRACK_DURATION: Duration = Duration::from_secs(120);
const MUSIC_CROSSFADE_MS: u64 = 5_000;
const DEFAULT_MUSIC_VOLUME: f64 = 0.35;
const SOUND_EFFECT_VOLUME: f64 = 0.8;

pub(crate) const PICKUP_SOUNDS: [AudioClipAddress; 4] =
    [sfx::CLICK, sfx::CLICK_2, sfx::CLICK_3, sfx::CLICK_4];
pub(crate) const CAPTURE_SOUNDS: [AudioClipAddress; 4] =
    [sfx::ATTACK_A, sfx::ATTACK_B, sfx::ATTACK_C, sfx::ATTACK_D];
pub(crate) const DROP_SOUNDS: [AudioClipAddress; 4] =
    [sfx::BOUNCE_0, sfx::BOUNCE_1, sfx::BOUNCE_2, sfx::BOUNCE_3];
pub(crate) const INVALID_DROP_SOUND: AudioClipAddress = sfx::ERROR;
pub(crate) const CASTLE_SOUND: AudioClipAddress = sfx::POWERUP_A;
pub(crate) const PROMOTION_SOUND: AudioClipAddress = sfx::POWERUP_B;
pub(crate) const CHECK_SOUND: AudioClipAddress = sfx::ALARM;
pub(crate) const START_SOUND: AudioClipAddress = sfx::ACCEPT;
pub(crate) const RESET_SOUND: AudioClipAddress = sfx::SCENE_TRANSITION;
pub(crate) const VOLUME_UP_SOUND: AudioClipAddress = sfx::CHIRP_A;
pub(crate) const VOLUME_DOWN_SOUND: AudioClipAddress = sfx::CHIRP_CRUNCH;
pub(crate) const PLAYER_WIN_SOUND: AudioClipAddress = sfx::LAP_COMPLETE;
pub(crate) const PLAYER_LOSS_SOUND: AudioClipAddress = sfx::FALL_AND_DIE;
pub(crate) const DRAW_SOUND: AudioClipAddress = sfx::WOBBLE_FALLING_TONE;

/// Addresses of NotJam's sound-effect collection.
pub const SOUND_EFFECTS: [AudioClipAddress; 41] = [
    sfx::ACCEPT,
    sfx::ALARM,
    sfx::ATTACK_A,
    sfx::ATTACK_B,
    sfx::ATTACK_C,
    sfx::ATTACK_D,
    sfx::BLEEP_WHITE_NOISE,
    sfx::BOOST_PAD,
    sfx::BOUNCE_0,
    sfx::BOUNCE_1,
    sfx::BOUNCE_2,
    sfx::BOUNCE_3,
    sfx::CHIRP_A,
    sfx::CHIRP_CRUNCH,
    sfx::CHIRP_WHITE_NOISE,
    sfx::CLICK,
    sfx::CLICK_2,
    sfx::CLICK_3,
    sfx::CLICK_4,
    sfx::CRUNCH_A,
    sfx::CRUNCH_B,
    sfx::DASH,
    sfx::ERROR,
    sfx::EXIT_SCENE_TRANSITION,
    sfx::FALL_AND_DIE,
    sfx::GRAPPLE,
    sfx::GREEN_LIGHT_TONE,
    sfx::LAP_COMPLETE,
    sfx::LOCKON_AVAILABLE,
    sfx::POWERUP_A,
    sfx::POWERUP_B,
    sfx::POWERUP_CURSED,
    sfx::RED_LIGHT_TONE,
    sfx::RISING_METALLIC,
    sfx::RISING_TONE_EXPLOSION,
    sfx::RISING_TONE_METALLIC,
    sfx::SCENE_TRANSITION,
    sfx::SIREN_EXPLOSION,
    sfx::SLINGSHOT,
    sfx::SWIPE_METALLIC,
    sfx::WOBBLE_FALLING_TONE,
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

    pub(crate) fn start_initial_track(&mut self, now: Instant) -> Command {
        self.reset(now);
        let active = CommandId::new_v4();
        self.active = Some(active);
        self.transition_due = Some(now + MUSIC_TRACK_DURATION);
        Command::new(active, self.play_body(false)).nonblocking()
    }

    pub(crate) fn set_volume(&mut self, volume: f64) -> Option<CommandBody> {
        self.volume = volume.clamp(0.0, 1.0);
        self.active.map(|active| {
            CommandBody::AudioSetVolume(PropertyCommand::canceling(AudioVolumePayload {
                audio_command_id: active,
                volume: self.volume,
            }))
        })
    }

    pub(crate) fn volume(&self) -> f64 {
        self.volume
    }

    fn play_body(&self, fade_in: bool) -> CommandBody {
        CommandBody::AudioPlay(AudioPlayPayload {
            address: MUSIC_TRACKS[self.track_index].clone(),
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
                if matches!(
                    &command.body,
                    CommandBody::AudioPlay(_) | CommandBody::ParticleSpawn(_)
                ) {
                    command.nonblocking()
                } else {
                    command
                }
            })
            .collect(),
    )
}

pub(crate) fn play_sound(address: impl Into<AudioClipAddress>) -> CommandBody {
    CommandBody::AudioPlay(AudioPlayPayload {
        address: address.into(),
        volume: SOUND_EFFECT_VOLUME,
        pitch: 1.0,
        r#loop: false,
        fade_in_ms: 0,
    })
}

pub(crate) fn random_sound(rng: &mut Rng, sounds: &[AudioClipAddress]) -> AudioClipAddress {
    sounds[rng.usize(..sounds.len())].clone()
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
