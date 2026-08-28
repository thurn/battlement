use battlement::{
  ActionId, Batch, BatchId, Command, CommandBody, GameObject, ObjectSetActivePayload,
  ParallelCommandGroup, ParticleSpawnLocation, ParticleSpawnPayload, Response, SessionId,
  WaitPayload,
};
use battlement_native::EngineError;
use cozy_chess::{Color, GameStatus, Square};
use fastrand::Rng;
use tracing::info;

use crate::{
  CRITICAL_BEAT_INTERVAL_MS, CRITICAL_FIRST_BEAT_OFFSET_MS, ChessEngine, PIECE_SPAWN_BEAT_COUNT,
  PLAY_BUTTON_ID, REFRESH_BUTTON_ID, assets::effects, audio, cursor,
};

const EFFECT_LIFETIME_MS: u64 = 1_000;

pub fn batch(
  session_id: SessionId,
  action_id: ActionId,
  pieces: [Vec<GameObject>; 2],
  start_commands: Vec<CommandBody>,
  cursor_commands: Vec<CommandBody>,
  music: Vec<Command>,
  rng: &mut Rng,
) -> Batch<Command> {
  let [mut white, mut black] = pieces;
  rng.shuffle(&mut white);
  rng.shuffle(&mut black);
  let maximum_side_size = white.len().max(black.len());
  let pieces_per_color_per_beat = maximum_side_size.div_ceil(PIECE_SPAWN_BEAT_COUNT).max(1);
  let stage_count = maximum_side_size.div_ceil(pieces_per_color_per_beat);
  let mut start = ParallelCommandGroup::from_bodies(
    start_commands
      .into_iter()
      .chain([CommandBody::set_input_enabled(false)]),
  );
  start
    .commands
    .push(Command::new_v4(audio::play_sound(audio::START_SOUND)).nonblocking());
  start.commands.extend(music);
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
    if index + 1 == stage_count {
      effects.extend(audio::parallel_group(cursor_commands.iter().cloned()).commands);
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

impl ChessEngine {
  pub(crate) fn start_game(
    &mut self,
    action_id: ActionId,
    cursor_visible: bool,
  ) -> Result<Response<Command>, EngineError> {
    if self.started {
      return Ok(Response::empty(self.session_id));
    }
    self.start_with_animation(
      action_id,
      cursor_visible,
      vec![
        CommandBody::object_destroy(PLAY_BUTTON_ID),
        CommandBody::object_create(crate::refresh_button(self.screen_aspect).active(false)),
        CommandBody::object_create(cursor::object(self.cursor, false)),
      ],
      true,
    )
  }

  pub(crate) fn restart_game(
    &mut self,
    action_id: ActionId,
    cursor_visible: bool,
  ) -> Result<Response<Command>, EngineError> {
    self.clear_persisted_board()?;
    let mut start_commands = self
      .objects
      .iter()
      .flatten()
      .copied()
      .map(CommandBody::object_destroy)
      .collect::<Vec<_>>();
    if self.started {
      start_commands.extend(self.hide_highlight_commands());
      start_commands.push(CommandBody::ObjectSetActive(ObjectSetActivePayload {
        object_id: REFRESH_BUTTON_ID,
        active: false,
      }));
      start_commands.extend(cursor::state_commands(cursor::START, false, false));
    } else {
      start_commands.extend([
        CommandBody::object_destroy(PLAY_BUTTON_ID),
        CommandBody::object_create(crate::refresh_button(self.screen_aspect).active(false)),
        CommandBody::object_create(cursor::object(cursor::START, false)),
      ]);
    }
    self.board = self.starting_board.clone();
    self.start_with_animation(action_id, cursor_visible, start_commands, false)
  }

  fn start_with_animation(
    &mut self,
    action_id: ActionId,
    cursor_visible: bool,
    start_commands: Vec<CommandBody>,
    persist: bool,
  ) -> Result<Response<Command>, EngineError> {
    self.started = true;
    self.cursor = cursor::START;
    self.cursor_visible = cursor_visible;
    self.selected = None;
    self.pause_open = false;
    self.confirm_new_game = false;
    self.ai_move = None;
    self.objects = crate::objects_for_board(&self.board);
    if persist {
      self.persist_board()?;
    }
    info!(
        name: "chess.game.started",
        side_to_move = ?self.board.side_to_move(),
        "New chess game started"
    );
    let music = self.music.start_initial_track((self.now)());
    let mut white = Vec::new();
    let mut black = Vec::new();
    for square in Square::ALL {
      if let Some(object_id) = self.objects[square as usize] {
        let color = self
          .board
          .color_on(square)
          .expect("mapped pieces have a color");
        let object = crate::piece_object(
          object_id,
          square,
          color,
          self
            .board
            .piece_on(square)
            .expect("mapped pieces have a type"),
        );
        if color == Color::White {
          white.push(object);
        } else {
          black.push(object);
        }
      }
    }
    if self.board.side_to_move() == Color::Black && self.board.status() == GameStatus::Ongoing {
      self.start_ai();
    }
    Ok(Response::batch(self::batch(
      self.session_id,
      action_id,
      [white, black],
      start_commands,
      cursor::state_commands(self.cursor, true, self.cursor_visible),
      music,
      &mut self.rng,
    )))
  }
}
