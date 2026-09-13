use battlement::{ActionId, BatchId, CommandId, ObjectId, Quaternion, SessionId, Vector3};
use battlement_native::{
  CoreCommandOffset, EngineError, EngineResponse, GameObjectOffset, MessageWriter,
  NativeControllerButton, NativeControllerInput, NativeDragMode, NativeImageFit,
  NativeObjectPlacement, NativeParentScene, NativePhysicalKey, NativePointerEvent,
  NativePreparedAssetKind, NativeSnapshotInput, NativeTransform,
};
use cozy_chess::{Color, File, GameStatus, Move, Piece, Rank, Square};

use crate::{
  ChessEngine, HIGHLIGHT_HEIGHT, HIGHLIGHT_SCALE, MUSIC_TRACKS, PLAY_BUTTON_ID, REFRESH_BUTTON_ID,
  SCENE_ID, assets, audio, cursor, visual_state,
};

const CLICK: &[NativePointerEvent] = &[NativePointerEvent::Click];
const GLOBAL_KEYS: &[NativePhysicalKey] = &[
  NativePhysicalKey::ARROW_LEFT,
  NativePhysicalKey::ARROW_RIGHT,
  NativePhysicalKey::ARROW_UP,
  NativePhysicalKey::ARROW_DOWN,
  NativePhysicalKey::ENTER,
  NativePhysicalKey::NUMPAD_ENTER,
  NativePhysicalKey::SPACE,
  NativePhysicalKey::ESCAPE,
  NativePhysicalKey::MINUS,
  NativePhysicalKey::EQUAL,
  NativePhysicalKey::KEY_R,
  NativePhysicalKey::SHIFT_LEFT,
  NativePhysicalKey::SHIFT_RIGHT,
  NativePhysicalKey::CONTROL_LEFT,
  NativePhysicalKey::CONTROL_RIGHT,
  NativePhysicalKey::META_LEFT,
  NativePhysicalKey::META_RIGHT,
  NativePhysicalKey::KEY_L,
];
const CONTROLLER_BUTTONS: &[NativeControllerButton] = &[
  NativeControllerButton::SOUTH,
  NativeControllerButton::EAST,
  NativeControllerButton::LEFT_SHOULDER,
  NativeControllerButton::RIGHT_SHOULDER,
  NativeControllerButton::START,
];

pub(crate) fn snapshot(engine: &ChessEngine) -> Result<EngineResponse, EngineError> {
  let mut message = MessageWriter::with_capacity(64 * 1024);
  let mut prepared = vec![
    message.prepared_asset(NativePreparedAssetKind::Scene, assets::CONTENT.as_str()),
    message.prepared_asset(
      NativePreparedAssetKind::Texture,
      assets::PLAY_BUTTON.as_str(),
    ),
    message.prepared_asset(
      NativePreparedAssetKind::Material,
      assets::LEGAL_SQUARE.as_str(),
    ),
    message.prepared_asset(
      NativePreparedAssetKind::Texture,
      assets::REFRESH_BUTTON.as_str(),
    ),
    message.prepared_asset(
      NativePreparedAssetKind::Prefab,
      assets::effects::PIECE_SELECTED.as_str(),
    ),
    message.prepared_asset(
      NativePreparedAssetKind::ParticleEffect,
      assets::effects::PIECE_SPAWN.as_str(),
    ),
    message.prepared_asset(
      NativePreparedAssetKind::ParticleEffect,
      assets::effects::CAPTURE.as_str(),
    ),
  ];
  for address in MUSIC_TRACKS {
    prepared.push(message.prepared_asset(NativePreparedAssetKind::AudioClip, address.as_str()));
  }
  for address in audio::SOUND_EFFECTS {
    prepared.push(message.prepared_asset(NativePreparedAssetKind::AudioClip, address.as_str()));
  }
  for color in Color::ALL {
    for piece in Piece::ALL {
      prepared.push(message.prepared_asset(
        NativePreparedAssetKind::Prefab,
        crate::address(color, piece).as_str(),
      ));
    }
  }

  let scene = message
    .scene(id(SCENE_ID), assets::CONTENT.as_str())
    .map_err(protocol)?;
  let mut objects = Vec::with_capacity(68);
  for (&object_id, square) in engine.highlight_ids.iter().zip(Square::ALL) {
    objects.push(highlight(&mut message, object_id, square)?);
  }
  if engine.started {
    objects.push(refresh_button(&mut message, engine.screen_aspect)?);
    objects.push(cursor_object(
      &mut message,
      engine.cursor,
      engine.cursor_visible,
    )?);
    for square in Square::ALL {
      let Some(object_id) = engine.objects[square as usize] else {
        continue;
      };
      objects.push(piece(
        &mut message,
        object_id,
        square,
        engine
          .board
          .color_on(square)
          .expect("mapped pieces have a color"),
        engine
          .board
          .piece_on(square)
          .expect("mapped pieces have a type"),
      )?);
    }
  } else {
    objects.push(play_button(&mut message)?);
  }

  let document =
    visual_state::write_document(&mut message, engine.visual_state).map_err(protocol)?;
  objects.push(
    message
      .ui_document_object(
        id(visual_state::DOCUMENT_ID),
        NativeObjectPlacement {
          parent_scene: NativeParentScene::Persistent,
          ..Default::default()
        },
        id(visual_state::ROOT_ID),
      )
      .map_err(protocol)?,
  );
  let session_id = id(engine.session_id);
  let snapshot = message
    .snapshot_with_configuration(
      session_id,
      &prepared,
      &[scene],
      None,
      &objects,
      None,
      &[document],
      NativeSnapshotInput {
        disabled: false,
        global_keys: GLOBAL_KEYS,
        controller: Some(NativeControllerInput {
          buttons: CONTROLLER_BUTTONS,
          navigation_enabled: true,
          stick_dead_zone: Some(0.35),
          repeat_delay_ms: Some(275),
          repeat_interval_ms: Some(125),
        }),
      },
    )
    .map_err(protocol)?;
  let mut messages = vec![snapshot];
  if engine.diagnostics_enabled {
    let metadata = [
      ("sample.name", "chess"),
      ("sample.rules_version", env!("CARGO_PKG_VERSION")),
      ("chess.opponent", "computer"),
      (
        "chess.game_origin",
        if engine.started { "saved" } else { "new" },
      ),
      (
        "chess.game_status",
        match engine.board.status() {
          cozy_chess::GameStatus::Ongoing => "ongoing",
          cozy_chess::GameStatus::Drawn => "drawn",
          cozy_chess::GameStatus::Won => "won",
        },
      ),
    ];
    let mut groups = Vec::with_capacity(metadata.len());
    for (key, value) in metadata {
      let command = message
        .set_diagnostics_metadata(id(CommandId::new_v4()), true, key, value)
        .map_err(protocol)?;
      groups.push(message.parallel_group(&[command]).map_err(protocol)?);
    }
    messages.push(
      message
        .batch(
          id(BatchId::new_v4()),
          session_id,
          None,
          battlement_native::NativeBatchStart::Now,
          &groups,
        )
        .map_err(protocol)?,
    );
  }
  let finished = message.finish(session_id, &messages).map_err(protocol)?;
  EngineResponse::from_core(session_id, finished)
}

pub(crate) fn show_log_viewer(
  session_id: SessionId,
  action_id: ActionId,
) -> Result<EngineResponse, EngineError> {
  let mut message = MessageWriter::default();
  let command = message
    .set_debug_ui(id(CommandId::new_v4()), true, false, true)
    .map_err(protocol)?;
  let group = message.parallel_group(&[command]).map_err(protocol)?;
  let session = id(session_id);
  let batch = message
    .batch(
      id(BatchId::new_v4()),
      session,
      Some(id(action_id)),
      battlement_native::NativeBatchStart::Now,
      &[group],
    )
    .map_err(protocol)?;
  let finished = message.finish(session, &[batch]).map_err(protocol)?;
  EngineResponse::from_core(session, finished)
}

pub(crate) fn action_response(
  session_id: SessionId,
  action_id: ActionId,
  build: impl FnOnce(
    &mut MessageWriter,
  ) -> Result<Vec<CoreCommandOffset>, battlement_native::ProtocolError>,
) -> Result<EngineResponse, EngineError> {
  batch_response(
    session_id,
    Some(action_id),
    battlement_native::NativeBatchStart::Now,
    move |message| {
      build(message)
        .map(|commands| vec![commands])
        .map_err(protocol)
    },
  )
}

pub(crate) fn batch_response(
  session_id: SessionId,
  action_id: Option<ActionId>,
  start: battlement_native::NativeBatchStart,
  build: impl FnOnce(&mut MessageWriter) -> Result<Vec<Vec<CoreCommandOffset>>, EngineError>,
) -> Result<EngineResponse, EngineError> {
  batch_response_with_metadata(session_id, action_id, start, &[], build)
}

pub(crate) fn batch_response_with_metadata(
  session_id: SessionId,
  action_id: Option<ActionId>,
  start: battlement_native::NativeBatchStart,
  metadata: &[(&str, &str)],
  build: impl FnOnce(&mut MessageWriter) -> Result<Vec<Vec<CoreCommandOffset>>, EngineError>,
) -> Result<EngineResponse, EngineError> {
  let mut message = MessageWriter::default();
  let command_groups = build(&mut message)?;
  if command_groups.iter().all(Vec::is_empty) {
    return EngineResponse::empty(id(session_id));
  }
  if command_groups.iter().any(Vec::is_empty) {
    return Err(EngineError::new(
      "direct Chess response contains an empty command group",
    ));
  }
  let groups = command_groups
    .iter()
    .map(|commands| message.parallel_group(commands))
    .collect::<Result<Vec<_>, _>>()
    .map_err(protocol)?;
  let session = id(session_id);
  let batch = message
    .batch(
      id(BatchId::new_v4()),
      session,
      action_id.map(id),
      start,
      &groups,
    )
    .map_err(protocol)?;
  let mut messages = vec![batch];
  if !metadata.is_empty() {
    let mut metadata_groups = Vec::with_capacity(metadata.len());
    for &(key, value) in metadata {
      let command = message
        .set_diagnostics_metadata(id(CommandId::new_v4()), true, key, value)
        .map_err(protocol)?;
      metadata_groups.push(message.parallel_group(&[command]).map_err(protocol)?);
    }
    messages.push(
      message
        .batch(
          id(BatchId::new_v4()),
          session,
          None,
          battlement_native::NativeBatchStart::Now,
          &metadata_groups,
        )
        .map_err(protocol)?,
    );
  }
  let finished = message.finish(session, &messages).map_err(protocol)?;
  EngineResponse::from_core(session, finished)
}

pub(crate) fn write_cursor(
  message: &mut MessageWriter,
  square: Square,
  restart: bool,
  visible: bool,
) -> Result<Vec<CoreCommandOffset>, battlement_native::ProtocolError> {
  if !visible {
    return Ok(vec![message.set_object_active(
      id(CommandId::new_v4()),
      true,
      id(cursor::EFFECT_ID),
      false,
    )?]);
  }
  let position = crate::square_position(square);
  Ok(vec![
    message.set_world_position(
      id(CommandId::new_v4()),
      true,
      id(cursor::EFFECT_ID),
      [position.x, position.y, position.z],
    )?,
    message.set_object_active(id(CommandId::new_v4()), true, id(cursor::EFFECT_ID), true)?,
    message.play_particles(
      id(CommandId::new_v4()),
      false,
      id(cursor::EFFECT_ID),
      restart,
    )?,
  ])
}

pub(crate) fn write_hidden_highlights(
  message: &mut MessageWriter,
  highlight_ids: &[ObjectId; 64],
) -> Result<Vec<CoreCommandOffset>, battlement_native::ProtocolError> {
  highlight_ids
    .iter()
    .map(|object_id| {
      message.set_object_active(id(CommandId::new_v4()), true, id(*object_id), false)
    })
    .collect()
}

pub(crate) fn write_sound(
  message: &mut MessageWriter,
  address: &str,
) -> Result<CoreCommandOffset, battlement_native::ProtocolError> {
  message.play_audio(
    id(CommandId::new_v4()),
    false,
    address,
    audio::SOUND_EFFECT_VOLUME,
    1.0,
    false,
    0,
  )
}

pub(crate) fn write_move(
  message: &mut MessageWriter,
  object_id: ObjectId,
  square: Square,
  animate: bool,
) -> Result<CoreCommandOffset, battlement_native::ProtocolError> {
  let position = crate::square_position(square);
  if animate {
    message.tween_world_position(
      id(CommandId::new_v4()),
      true,
      id(object_id),
      [position.x, position.y, position.z],
      300,
      battlement_native::NativeEasing::InOutSine,
    )
  } else {
    message.set_world_position(
      id(CommandId::new_v4()),
      true,
      id(object_id),
      [position.x, position.y, position.z],
    )
  }
}

pub(crate) fn write_knight_first_leg(
  message: &mut MessageWriter,
  object_id: ObjectId,
  from: Square,
  to: Square,
) -> Result<CoreCommandOffset, battlement_native::ProtocolError> {
  let from = crate::square_position(from);
  let to = crate::square_position(to);
  let corner = if (to.x - from.x).abs() > (to.z - from.z).abs() {
    Vector3::new(to.x, to.y, from.z)
  } else {
    Vector3::new(from.x, to.y, to.z)
  };
  message.tween_world_position(
    id(CommandId::new_v4()),
    true,
    id(object_id),
    [corner.x, corner.y, corner.z],
    200,
    battlement_native::NativeEasing::InOutSine,
  )
}

pub(crate) fn write_knight_second_leg(
  message: &mut MessageWriter,
  object_id: ObjectId,
  to: Square,
) -> Result<CoreCommandOffset, battlement_native::ProtocolError> {
  let position = crate::square_position(to);
  message.tween_world_position(
    id(CommandId::new_v4()),
    true,
    id(object_id),
    [position.x, position.y, position.z],
    120,
    battlement_native::NativeEasing::InOutSine,
  )
}

pub(crate) fn write_capture_effect(
  message: &mut MessageWriter,
  square: Square,
) -> Result<CoreCommandOffset, battlement_native::ProtocolError> {
  let position = crate::square_position(square);
  message.spawn_particle_at_world_position(
    id(CommandId::new_v4()),
    false,
    assets::effects::CAPTURE.as_str(),
    [position.x, position.y, position.z],
    crate::CAPTURE_EFFECT_LIFETIME_MS,
  )
}

pub(crate) fn start_game(
  engine: &mut ChessEngine,
  action_id: ActionId,
  cursor_visible: bool,
) -> Result<EngineResponse, EngineError> {
  if engine.started {
    return EngineResponse::empty(id(engine.session_id));
  }
  animated_opening(
    engine,
    action_id,
    cursor_visible,
    Opening::Fresh,
    true,
    crate::VisualState::Initial,
  )
}

pub(crate) fn restart_game(
  engine: &mut ChessEngine,
  action_id: ActionId,
  cursor_visible: bool,
) -> Result<EngineResponse, EngineError> {
  engine.clear_persisted_board()?;
  engine.piece_generation += 1;
  let was_started = engine.started;
  let previous_objects = engine.objects.iter().flatten().copied().collect();
  engine.board = engine.starting_board.clone();
  animated_opening(
    engine,
    action_id,
    cursor_visible,
    Opening::Restart {
      was_started,
      previous_objects,
    },
    false,
    crate::VisualState::Restarted,
  )
}

pub(crate) fn new_game(
  engine: &mut ChessEngine,
  action_id: ActionId,
  cursor_visible: bool,
) -> Result<EngineResponse, EngineError> {
  let was_started = engine.started;
  let previous_objects = engine.objects.iter().flatten().copied().collect::<Vec<_>>();
  engine.ai_move = None;
  engine.ai_poll_deferrals = 0;
  engine.cursor = cursor::START;
  engine.cursor_visible = cursor_visible;
  engine.selected = None;
  engine.pause_open = false;
  engine.confirm_new_game = false;
  engine.board = engine.starting_board.clone();
  engine.piece_generation += 1;
  engine.objects = crate::objects_for_board(&engine.board, engine.piece_generation);
  engine.started = true;
  let previous_state = engine.change_visual_state(crate::VisualState::Refreshed);
  engine.persist_board()?;
  tracing::info!(side_to_move = ?engine.board.side_to_move(), "New chess game started");
  if !was_started {
    engine.music.reset((engine.now)());
  }
  let pieces = Square::ALL
    .into_iter()
    .filter_map(|square| {
      engine.objects[square as usize].map(|object_id| {
        (
          object_id,
          square,
          engine
            .board
            .color_on(square)
            .expect("mapped pieces have a color"),
          engine
            .board
            .piece_on(square)
            .expect("mapped pieces have a type"),
        )
      })
    })
    .collect::<Vec<_>>();
  let session_id = engine.session_id;
  let highlight_ids = engine.highlight_ids;
  let cursor_square = engine.cursor;
  let metadata = if engine.diagnostics_enabled {
    vec![
      ("chess.game_origin", "new"),
      ("chess.game_status", "ongoing"),
    ]
  } else {
    Vec::new()
  };
  let response = batch_response_with_metadata(
    session_id,
    Some(action_id),
    battlement_native::NativeBatchStart::Now,
    &metadata,
    |message| {
      let mut commands = Vec::new();
      for object_id in previous_objects {
        commands.push(
          message
            .destroy_object(id(CommandId::new_v4()), true, id(object_id))
            .map_err(protocol)?,
        );
      }
      if was_started {
        commands.extend(write_hidden_highlights(message, &highlight_ids).map_err(protocol)?);
        commands.push(
          message
            .set_object_active(id(CommandId::new_v4()), true, id(REFRESH_BUTTON_ID), false)
            .map_err(protocol)?,
        );
      } else {
        let cursor = cursor_object(message, cursor_square, false)?;
        commands.push(
          message
            .create_object(id(CommandId::new_v4()), true, cursor)
            .map_err(protocol)?,
        );
        commands.push(
          message
            .destroy_object(id(CommandId::new_v4()), true, id(PLAY_BUTTON_ID))
            .map_err(protocol)?,
        );
      }
      for (object_id, square, color, piece_kind) in pieces {
        let object = piece(message, object_id, square, color, piece_kind)?;
        commands.push(
          message
            .create_object(id(CommandId::new_v4()), true, object)
            .map_err(protocol)?,
        );
      }
      commands
        .extend(write_cursor(message, cursor_square, true, cursor_visible).map_err(protocol)?);
      commands.push(
        message
          .set_input_enabled(id(CommandId::new_v4()), true, true)
          .map_err(protocol)?,
      );
      commands.push(write_sound(message, crate::RESET_SOUND.as_str()).map_err(protocol)?);
      commands.extend(
        visual_state::write_transition(message, previous_state, crate::VisualState::Refreshed)
          .map_err(protocol)?,
      );
      Ok(vec![commands])
    },
  )?;
  if engine.board.side_to_move() == Color::Black && engine.board.status() == GameStatus::Ongoing {
    engine.start_ai();
  }
  Ok(response)
}

enum Opening {
  Fresh,
  Restart {
    was_started: bool,
    previous_objects: Vec<ObjectId>,
  },
}

fn animated_opening(
  engine: &mut ChessEngine,
  action_id: ActionId,
  cursor_visible: bool,
  opening: Opening,
  persist: bool,
  state: crate::VisualState,
) -> Result<EngineResponse, EngineError> {
  engine.started = true;
  engine.cursor = cursor::START;
  engine.cursor_visible = cursor_visible;
  engine.selected = None;
  engine.pause_open = false;
  engine.confirm_new_game = false;
  engine.ai_move = None;
  engine.objects = crate::objects_for_board(&engine.board, engine.piece_generation);
  let previous_state = engine.change_visual_state(state);
  if persist {
    engine.persist_board()?;
  }
  tracing::info!(side_to_move = ?engine.board.side_to_move(), "New chess game started");

  let mut white = Vec::new();
  let mut black = Vec::new();
  for square in Square::ALL {
    let Some(object_id) = engine.objects[square as usize] else {
      continue;
    };
    let color = engine
      .board
      .color_on(square)
      .expect("mapped pieces have a color");
    let piece_kind = engine
      .board
      .piece_on(square)
      .expect("mapped pieces have a type");
    if color == Color::White {
      white.push((object_id, square, color, piece_kind));
    } else {
      black.push((object_id, square, color, piece_kind));
    }
  }
  engine.rng.shuffle(&mut white);
  engine.rng.shuffle(&mut black);
  let maximum_side_size = white.len().max(black.len());
  let per_beat = maximum_side_size
    .div_ceil(crate::PIECE_SPAWN_BEAT_COUNT)
    .max(1);
  let stage_count = maximum_side_size.div_ceil(per_beat);
  let session_id = engine.session_id;
  let cursor_square = engine.cursor;
  let now = (engine.now)();
  let screen_aspect = engine.screen_aspect;
  let response = batch_response(
    session_id,
    Some(action_id),
    battlement_native::NativeBatchStart::Now,
    |message| {
      let mut start = Vec::new();
      match opening {
        Opening::Fresh => {
          start.push(
            message
              .destroy_object(id(CommandId::new_v4()), true, id(PLAY_BUTTON_ID))
              .map_err(protocol)?,
          );
          let refresh = refresh_button(message, screen_aspect)?;
          start.push(
            message
              .create_object(id(CommandId::new_v4()), true, refresh)
              .map_err(protocol)?,
          );
          let cursor = cursor_object(message, cursor_square, false)?;
          start.push(
            message
              .create_object(id(CommandId::new_v4()), true, cursor)
              .map_err(protocol)?,
          );
        }
        Opening::Restart {
          was_started,
          previous_objects,
        } => {
          for object_id in previous_objects {
            start.push(
              message
                .destroy_object(id(CommandId::new_v4()), true, id(object_id))
                .map_err(protocol)?,
            );
          }
          if was_started {
            start
              .extend(write_hidden_highlights(message, &engine.highlight_ids).map_err(protocol)?);
            start.push(
              message
                .set_object_active(id(CommandId::new_v4()), true, id(REFRESH_BUTTON_ID), false)
                .map_err(protocol)?,
            );
            start.extend(write_cursor(message, cursor::START, false, false).map_err(protocol)?);
          } else {
            start.push(
              message
                .destroy_object(id(CommandId::new_v4()), true, id(PLAY_BUTTON_ID))
                .map_err(protocol)?,
            );
            let refresh = refresh_button(message, screen_aspect)?;
            start.push(
              message
                .create_object(id(CommandId::new_v4()), true, refresh)
                .map_err(protocol)?,
            );
            let cursor = cursor_object(message, cursor::START, false)?;
            start.push(
              message
                .create_object(id(CommandId::new_v4()), true, cursor)
                .map_err(protocol)?,
            );
          }
        }
      }
      let [destroy_state, create_state] =
        visual_state::write_transition(message, previous_state, state).map_err(protocol)?;
      start.push(destroy_state);
      start.push(
        message
          .set_input_enabled(id(CommandId::new_v4()), true, false)
          .map_err(protocol)?,
      );
      start.push(write_sound(message, audio::START_SOUND.as_str()).map_err(protocol)?);
      start.extend(
        engine
          .music
          .write_initial_track(message, now)
          .map_err(protocol)?,
      );
      let mut groups = vec![start];
      groups.push(vec![
        message
          .wait(
            id(CommandId::new_v4()),
            crate::CRITICAL_FIRST_BEAT_OFFSET_MS,
          )
          .map_err(protocol)?,
      ]);

      for index in 0..stage_count {
        let range_start = index * per_beat;
        let range_end = range_start + per_beat;
        let pieces = [
          white.get(range_start..range_end.min(white.len())),
          black.get(range_start..range_end.min(black.len())),
        ]
        .into_iter()
        .flatten()
        .flatten();
        let mut creates = Vec::new();
        let mut effects = Vec::new();
        for &(object_id, square, color, piece_kind) in pieces {
          let object = piece(message, object_id, square, color, piece_kind)?;
          creates.push(
            message
              .create_object(id(CommandId::new_v4()), true, object)
              .map_err(protocol)?,
          );
          effects.push(
            message
              .spawn_particle_at_object(
                id(CommandId::new_v4()),
                false,
                assets::effects::PIECE_SPAWN.as_str(),
                id(object_id),
                crate::PIECE_SPAWN_EFFECT_LIFETIME_MS,
              )
              .map_err(protocol)?,
          );
        }
        groups.push(creates);
        groups.push(effects);
        if index + 1 < stage_count {
          groups.push(vec![
            message
              .wait(id(CommandId::new_v4()), crate::CRITICAL_BEAT_INTERVAL_MS)
              .map_err(protocol)?,
          ]);
        }
      }
      groups.push(vec![
        message
          .wait(
            id(CommandId::new_v4()),
            crate::PIECE_SPAWN_EFFECT_LIFETIME_MS,
          )
          .map_err(protocol)?,
      ]);
      let mut final_group =
        write_cursor(message, cursor_square, true, cursor_visible).map_err(protocol)?;
      final_group.push(create_state);
      final_group.push(
        message
          .set_input_enabled(id(CommandId::new_v4()), true, true)
          .map_err(protocol)?,
      );
      groups.push(final_group);
      Ok(groups)
    },
  )?;
  if engine.board.side_to_move() == Color::Black && engine.board.status() == GameStatus::Ongoing {
    engine.start_ai();
  }
  Ok(response)
}

pub(crate) fn apply_move(
  engine: &mut ChessEngine,
  message: &mut MessageWriter,
  movement: Move,
  animate: bool,
) -> Result<Vec<Vec<CoreCommandOffset>>, EngineError> {
  let board_before = engine.board.clone();
  let color = engine
    .board
    .color_on(movement.from)
    .expect("legal moves have a moving piece");
  let piece_kind = engine
    .board
    .piece_on(movement.from)
    .expect("legal moves have a moving piece");
  let is_castle = piece_kind == Piece::King && engine.board.color_on(movement.to) == Some(color);
  let mut groups = if is_castle {
    vec![apply_castle(engine, message, movement, color, animate)?]
  } else {
    apply_standard_move(engine, message, movement, color, piece_kind, animate)?
  };
  engine.board.play_unchecked(movement);
  let next_state = visual_state::after_move(&board_before, &engine.board, movement, color);
  let previous_state = engine.change_visual_state(next_state);
  groups
    .last_mut()
    .expect("a move has a final group")
    .extend(visual_state::write_transition(message, previous_state, next_state).map_err(protocol)?);
  engine.persist_board()?;
  tracing::info!(
      side = ?color,
      piece = ?piece_kind,
      from = %movement.from,
      to = %movement.to,
      promotion = ?movement.promotion,
      animated = animate,
      status = ?engine.board.status(),
      "Chess move applied"
  );
  let terminal_sound = match engine.board.status() {
    GameStatus::Won if engine.board.side_to_move() == Color::Black => Some(crate::PLAYER_WIN_SOUND),
    GameStatus::Won => Some(crate::PLAYER_LOSS_SOUND),
    GameStatus::Drawn => Some(crate::DRAW_SOUND),
    GameStatus::Ongoing if !engine.board.checkers().is_empty() => Some(crate::CHECK_SOUND),
    GameStatus::Ongoing => None,
  };
  if let Some(sound) = terminal_sound {
    groups
      .last_mut()
      .expect("a move has a final group")
      .push(write_sound(message, sound.as_str()).map_err(protocol)?);
  }
  Ok(groups)
}

fn apply_castle(
  engine: &mut ChessEngine,
  message: &mut MessageWriter,
  movement: Move,
  color: Color,
  animate: bool,
) -> Result<Vec<CoreCommandOffset>, EngineError> {
  let rank = if color == Color::White {
    Rank::First
  } else {
    Rank::Eighth
  };
  let short = movement.to.file() > movement.from.file();
  let king_to = Square::new(if short { File::G } else { File::C }, rank);
  let rook_to = Square::new(if short { File::F } else { File::D }, rank);
  let king = engine.objects[movement.from as usize]
    .take()
    .expect("castling king has an object");
  let rook = engine.objects[movement.to as usize]
    .take()
    .expect("castling rook has an object");
  engine.objects[king_to as usize] = Some(king);
  engine.objects[rook_to as usize] = Some(rook);
  Ok(vec![
    write_move(message, king, king_to, animate).map_err(protocol)?,
    write_move(message, rook, rook_to, animate).map_err(protocol)?,
    write_sound(message, crate::CASTLE_SOUND.as_str()).map_err(protocol)?,
  ])
}

fn apply_standard_move(
  engine: &mut ChessEngine,
  message: &mut MessageWriter,
  movement: Move,
  color: Color,
  piece_kind: Piece,
  animate: bool,
) -> Result<Vec<Vec<CoreCommandOffset>>, EngineError> {
  let capture = if piece_kind == Piece::Pawn
    && movement.from.file() != movement.to.file()
    && engine.board.piece_on(movement.to).is_none()
  {
    Square::new(movement.to.file(), movement.from.rank())
  } else {
    movement.to
  };
  let captured = engine.objects[capture as usize].take();
  let is_capture = captured.is_some();
  let mut first = Vec::new();
  if !animate && let Some(captured) = captured {
    first.push(
      message
        .destroy_object(id(CommandId::new_v4()), true, id(captured))
        .map_err(protocol)?,
    );
    first.push(write_capture_effect(message, capture).map_err(protocol)?);
  }
  let moving = engine.objects[movement.from as usize]
    .take()
    .expect("legal moving pieces have an object");
  if let Some(promotion) = movement.promotion {
    first.push(write_move(message, moving, movement.to, animate).map_err(protocol)?);
    let promoted = crate::promotion_id(movement.to);
    engine.objects[movement.to as usize] = Some(promoted);
    let mut promotion_commands = Vec::new();
    if animate && let Some(captured) = captured {
      promotion_commands.push(
        message
          .destroy_object(id(CommandId::new_v4()), true, id(captured))
          .map_err(protocol)?,
      );
    }
    if animate && is_capture {
      promotion_commands.push(write_capture_effect(message, capture).map_err(protocol)?);
    }
    promotion_commands.push(
      message
        .destroy_object(id(CommandId::new_v4()), true, id(moving))
        .map_err(protocol)?,
    );
    let promoted = piece(message, promoted, movement.to, color, promotion)?;
    promotion_commands.push(
      message
        .create_object(id(CommandId::new_v4()), true, promoted)
        .map_err(protocol)?,
    );
    promotion_commands
      .push(write_sound(message, crate::PROMOTION_SOUND.as_str()).map_err(protocol)?);
    return Ok(vec![first, promotion_commands]);
  }

  engine.objects[movement.to as usize] = Some(moving);
  if animate && piece_kind == Piece::Knight {
    first
      .push(write_knight_first_leg(message, moving, movement.from, movement.to).map_err(protocol)?);
  } else {
    first.push(write_move(message, moving, movement.to, animate).map_err(protocol)?);
  }
  let sounds = if is_capture {
    crate::CAPTURE_SOUNDS
  } else {
    crate::DROP_SOUNDS
  };
  let sound_index = engine.rng.usize(..sounds.len());
  let mut groups = if animate && piece_kind == Piece::Knight {
    vec![
      first,
      vec![write_knight_second_leg(message, moving, movement.to).map_err(protocol)?],
    ]
  } else {
    vec![first]
  };
  let sound = write_sound(message, sounds[sound_index].as_str()).map_err(protocol)?;
  if animate && let Some(captured) = captured {
    groups.push(vec![
      message
        .destroy_object(id(CommandId::new_v4()), true, id(captured))
        .map_err(protocol)?,
      write_capture_effect(message, capture).map_err(protocol)?,
      sound,
    ]);
  } else {
    groups
      .first_mut()
      .expect("a move has a first group")
      .push(sound);
  }
  Ok(groups)
}

fn highlight(
  message: &mut MessageWriter,
  object_id: ObjectId,
  square: Square,
) -> Result<GameObjectOffset, EngineError> {
  let position = crate::square_position(square);
  message
    .plane_object(
      id(object_id),
      placement(
        false,
        position_with_y(position, HIGHLIGHT_HEIGHT),
        Quaternion::IDENTITY,
        Vector3::new(HIGHLIGHT_SCALE, 1.0, HIGHLIGHT_SCALE),
        CLICK,
        NativeDragMode::None,
      ),
      &[(0, assets::LEGAL_SQUARE.as_str())],
    )
    .map_err(protocol)
}

pub(crate) fn piece(
  message: &mut MessageWriter,
  object_id: ObjectId,
  square: Square,
  color: Color,
  piece: Piece,
) -> Result<GameObjectOffset, EngineError> {
  message
    .prefab_object(
      id(object_id),
      placement(
        true,
        crate::square_position(square),
        if color == Color::Black {
          Quaternion::new(0.0, 1.0, 0.0, 0.0)
        } else {
          Quaternion::IDENTITY
        },
        Vector3::ONE,
        CLICK,
        if color == Color::White {
          NativeDragMode::SnapToPointer
        } else {
          NativeDragMode::None
        },
      ),
      crate::address(color, piece).as_str(),
      &[],
      None,
    )
    .map_err(protocol)
}

fn play_button(message: &mut MessageWriter) -> Result<GameObjectOffset, EngineError> {
  message
    .image_object(
      id(PLAY_BUTTON_ID),
      placement(
        true,
        Vector3::new(0.0, 6.38, -3.86),
        crate::CAMERA_ROTATION,
        Vector3::ONE,
        CLICK,
        NativeDragMode::None,
      ),
      assets::PLAY_BUTTON.as_str(),
      0.8,
      0.24,
      NativeImageFit::Stretch,
    )
    .map_err(protocol)
}

fn refresh_button(
  message: &mut MessageWriter,
  screen_aspect: f64,
) -> Result<GameObjectOffset, EngineError> {
  let half_height = crate::CAMERA_BUTTON_DEPTH * (crate::CAMERA_VERTICAL_FOV_RADIANS / 2.0).tan();
  let right =
    half_height * screen_aspect - crate::REFRESH_BUTTON_SIZE / 2.0 - crate::REFRESH_BUTTON_MARGIN;
  let up = half_height - crate::REFRESH_BUTTON_SIZE / 2.0 - crate::REFRESH_BUTTON_MARGIN;
  message
    .image_object(
      id(REFRESH_BUTTON_ID),
      placement(
        false,
        Vector3::new(
          right,
          8.0 - 0.946201 * crate::CAMERA_BUTTON_DEPTH + 0.323579 * up,
          -3.75 + 0.323579 * crate::CAMERA_BUTTON_DEPTH + 0.946201 * up,
        ),
        crate::CAMERA_ROTATION,
        Vector3::ONE,
        CLICK,
        NativeDragMode::None,
      ),
      assets::REFRESH_BUTTON.as_str(),
      crate::REFRESH_BUTTON_SIZE,
      crate::REFRESH_BUTTON_SIZE,
      NativeImageFit::Stretch,
    )
    .map_err(protocol)
}

fn cursor_object(
  message: &mut MessageWriter,
  square: Square,
  active: bool,
) -> Result<GameObjectOffset, EngineError> {
  message
    .prefab_object(
      id(cursor::EFFECT_ID),
      placement(
        active,
        crate::square_position(square),
        Quaternion::IDENTITY,
        Vector3::ONE,
        &[],
        NativeDragMode::None,
      ),
      assets::effects::PIECE_SELECTED.as_str(),
      &[],
      None,
    )
    .map_err(protocol)
}

fn placement<'a>(
  active: bool,
  position: Vector3,
  rotation: Quaternion,
  scale: Vector3,
  pointer_events: &'a [NativePointerEvent],
  drag_mode: NativeDragMode,
) -> NativeObjectPlacement<'a> {
  NativeObjectPlacement {
    active,
    transform: NativeTransform {
      position: [position.x, position.y, position.z],
      rotation: [rotation.x, rotation.y, rotation.z, rotation.w],
      scale: [scale.x, scale.y, scale.z],
    },
    pointer_events,
    drag_mode,
    ..Default::default()
  }
}

fn position_with_y(position: Vector3, y: f64) -> Vector3 {
  Vector3::new(position.x, y, position.z)
}

fn id<K>(value: battlement::ProtocolId<K>) -> [u8; 16] {
  *value.as_uuid().as_bytes()
}

pub(crate) fn protocol(error: battlement_native::ProtocolError) -> EngineError {
  EngineError::new(error.to_string())
}

#[cfg(test)]
mod tests {
  use battlement_native::ResponseView;
  use cozy_chess::{Move, Square};

  #[test]
  fn initial_snapshot_is_authored_and_verified_without_an_owned_protocol_graph() {
    let engine = crate::create_seeded_engine(7);
    let response = super::snapshot(&engine).expect("direct Chess snapshot should verify");
    let view =
      ResponseView::read(response.as_bytes()).expect("native response should remain valid");
    assert_eq!(view.session_id(), *engine.session_id.as_uuid().as_bytes());
    assert_eq!(view.message_count(), 1);
  }

  #[test]
  fn diagnostics_metadata_is_appended_as_a_direct_second_message() {
    let mut engine = crate::create_seeded_engine(7);
    engine.diagnostics_enabled = true;
    let response = super::snapshot(&engine).expect("direct diagnostics snapshot should verify");
    let view =
      ResponseView::read(response.as_bytes()).expect("native response should remain valid");
    assert_eq!(view.message_count(), 2);
  }

  #[test]
  fn legal_move_commands_are_authored_directly_and_verify() {
    let mut engine = crate::create_seeded_engine(7);
    engine.started = true;
    engine.objects = crate::objects_for_board(&engine.board, engine.piece_generation);
    engine.visual_state = crate::visual_state::VisualState::Initial;
    let session = engine.session_id;
    let movement = Move {
      from: Square::E2,
      to: Square::E4,
      promotion: None,
    };
    let response = super::batch_response(
      session,
      None,
      battlement_native::NativeBatchStart::Now,
      |message| super::apply_move(&mut engine, message, movement, true),
    )
    .expect("direct move response should verify");
    let view =
      ResponseView::read(response.as_bytes()).expect("native response should remain valid");
    assert_eq!(view.message_count(), 1);
    assert_eq!(
      engine.board.piece_on(Square::E4),
      Some(cozy_chess::Piece::Pawn)
    );
  }

  #[test]
  fn beat_timed_opening_is_authored_directly_and_verifies() {
    let mut engine = crate::create_seeded_engine(43);
    let action_id = battlement::ActionId::new_v4();
    let response = super::start_game(&mut engine, action_id, false)
      .expect("direct opening response should verify");
    let view =
      ResponseView::read(response.as_bytes()).expect("native response should remain valid");
    assert_eq!(view.message_count(), 1);
    assert!(engine.started);
    assert_eq!(engine.objects.iter().flatten().count(), 32);
  }
}
