//! Declarative board, square, and piece components.

use std::time::Duration;

use battlement::{DragMode, MaterialAssignment, ObjectId, ParentScene, Quaternion, Vector3};
use cozy_chess::{Color, GameStatus, Square};
use reactant::{
  GameStatus as RulesStatus, SnapshotAnimation,
  animation_controls::{AnimationSequence, MotionSelector, SequencePosition},
  prelude::{
    AnimationPlayback, Button, Component, Easing, EventCallback, IdentityRenderExt, KeyRenderExt,
    MotionComponentExt, MotionProps, Position, Render, Style, StyleTarget, Transition,
    use_object_ref,
  },
  world,
};
use trox::ls;

use crate::{
  PIECE_SPAWN_EFFECT_LIFETIME_MS, PIECE_SPAWN_SEQUENCE_DURATION_MS,
  chess_ui_state::{ChessUiController, ChessUiState, SessionStart, UiAction},
  position::{ChessPiece, Movement},
  reactant_game::{ChessAnimation, ChessGame},
};

pub(crate) struct ChessBoard {
  pub(crate) ui: ChessUiState,
  pub(crate) on_activate: EventCallback<Square>,
  pub(crate) on_move: EventCallback<(Square, Square)>,
  pub(crate) on_request_new_game: EventCallback<()>,
  pub(crate) on_opening_finished: std::rc::Rc<dyn Fn(AnimationPlayback, u64)>,
  pub(crate) game: reactant::GameHandle<ChessGame>,
  pub(crate) control: ChessUiController,
}

struct ChessSquare {
  square: Square,
  legal_target: bool,
  piece: Option<ChessPiece>,
  piece_reference: reactant::prelude::ObjectRef,
  on_activate: EventCallback<Square>,
  spawning: bool,
  game: reactant::GameHandle<ChessGame>,
  control: ChessUiController,
  interactive: bool,
}

struct ChessPieceView {
  piece: ChessPiece,
  square: Square,
  reference: reactant::prelude::ObjectRef,
  on_activate: EventCallback<Square>,
  spawning: bool,
  game: reactant::GameHandle<ChessGame>,
  control: ChessUiController,
}

impl Component for ChessBoard {
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<ChessGame>();
    let status = reactant::use_game_status::<ChessGame>();
    let screen = reactant::app_context::use_viewport_size();
    let local = &self.ui;
    let scope = reactant::animation_controls::use_animation_scope();
    let event_scope = scope.clone();
    let references: [reactant::prelude::ObjectRef; 64] = std::array::from_fn(|_| use_object_ref());
    let event_references = references.clone();
    reactant::use_animate::<ChessGame>(move |animation| {
      Some(SnapshotAnimation::sequence(
        event_scope.clone(),
        sequence(animation, &event_references),
      ))
    });

    let opening_scope = scope.clone();
    let opening_references = references.clone();
    let on_opening_finished = self.on_opening_finished.clone();
    let opening = local.opening;
    let opening_generation = local.opening_generation;
    let opening_pieces = Square::ALL
      .into_iter()
      .filter_map(|square| state.piece(square))
      .collect::<Vec<_>>();
    reactant::hooks::use_commit_effect(
      move || {
        if let Some(mode) = opening {
          let playback = opening_scope.start_blocking(opening_sequence(
            mode,
            &opening_pieces,
            &opening_references,
          ));
          on_opening_finished(playback, opening_generation);
        } else {
          opening_scope.stop(MotionSelector::Descendants);
        }
      },
      (opening_generation, opening),
    );

    let restore_scope = scope.clone();
    let restore_references = references.clone();
    let drag_restore = local.drag_restore;
    reactant::hooks::use_commit_effect(
      move || {
        if let Some((piece, square)) = drag_restore {
          restore_scope.set(
            MotionSelector::object(piece_reference(&restore_references, piece)),
            crate::motion::position_target(square),
          );
        }
      },
      local.drag_restore_generation,
    );

    let legal = local
      .selected
      .map(|square| crate::legal_destinations(state.board(), square))
      .unwrap_or_default();
    let interactive = status == RulesStatus::Ready
      && state.board().status() == GameStatus::Ongoing
      && state.board().side_to_move() == Color::White
      && !local.pause_open();
    let squares = Square::ALL
      .into_iter()
      .map(|square| {
        ChessSquare {
          square,
          legal_target: legal.contains(&square),
          piece: state.piece(square),
          piece_reference: state
            .piece(square)
            .map(|piece| piece_reference(&references, piece.entity_id))
            .unwrap_or_else(|| references[square as usize].clone()),
          on_activate: self.on_activate.clone(),
          spawning: local.spawning,
          interactive,
          game: self.game.clone(),
          control: self.control.clone(),
        }
        .key(square)
      })
      .collect::<Vec<_>>();
    let semantic_moves = Square::ALL
      .into_iter()
      .filter(|square| state.board().color_on(*square) == Some(Color::White))
      .flat_map(|from| {
        crate::legal_destinations(state.board(), from)
          .into_iter()
          .map(move |to| (from, to))
      })
      .collect::<Vec<_>>();
    let semantics = semantic_moves
      .into_iter()
      .enumerate()
      .map(|(index, (from, to))| {
        Button::new(ls(format!("Move {from} to {to}")))
          .host_name(format!(
            "move-{}-{}",
            from.to_string().to_ascii_lowercase(),
            to.to_string().to_ascii_lowercase()
          ))
          .style(
            Style::new()
              .position(Position::Absolute)
              .left((index % 16) as f32 * 20.0)
              .top((index / 16) as f32 * 20.0)
              .width(18.0)
              .height(18.0)
              .opacity(0.0),
          )
          .on_press(self.on_move.clone().map_input(move |()| (from, to)))
          .key((from, to))
      })
      .collect::<Vec<_>>();
    let cursor_active = local.cursor_visible || local.selected.is_some();
    let cursor_scale = if status == RulesStatus::Busy {
      0.55
    } else {
      1.0
    };
    let cursor = world::Group::new()
      .id(*crate::cursor::EFFECT_ID.as_uuid())
      .position(crate::square_position(local.cursor))
      .scale(Vector3::new(cursor_scale, cursor_scale, cursor_scale))
      .active(cursor_active)
      .child(world::Prefab::at(crate::assets::effects::PIECE_SELECTED));
    let refresh = local.pause_open().then(|| {
      let aspect = if screen.height == 0 {
        1.0
      } else {
        f64::from(screen.width) / f64::from(screen.height)
      };
      let half_height =
        crate::CAMERA_BUTTON_DEPTH * (crate::CAMERA_VERTICAL_FOV_RADIANS / 2.0).tan();
      let right =
        half_height * aspect - crate::REFRESH_BUTTON_SIZE / 2.0 - crate::REFRESH_BUTTON_MARGIN;
      let up = half_height - crate::REFRESH_BUTTON_SIZE / 2.0 - crate::REFRESH_BUTTON_MARGIN;
      world::Sprite::new()
        .id(*crate::REFRESH_BUTTON_ID.as_uuid())
        .texture(crate::assets::REFRESH_BUTTON)
        .size(crate::REFRESH_BUTTON_SIZE, crate::REFRESH_BUTTON_SIZE)
        .fit(battlement::ImageFit::Stretch)
        .position(Vector3::new(
          right,
          8.0 - 0.946201 * crate::CAMERA_BUTTON_DEPTH + 0.323579 * up,
          -3.75 + 0.323579 * crate::CAMERA_BUTTON_DEPTH + 0.946201 * up,
        ))
        .rotation(crate::CAMERA_ROTATION)
        .on_click(self.on_request_new_game.clone())
    });

    (
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .child((squares, cursor, refresh))
          .motion(MotionProps::new().animation_scope(scope)),
      ),
      semantics,
    )
  }
}

impl Component for ChessSquare {
  fn render(&self) -> impl Render {
    let square = self.square;
    let mut position = crate::square_position(square);
    position.y = crate::HIGHLIGHT_HEIGHT;
    let surface = world::Plane::new()
      .position(position)
      .scale(Vector3::new(
        crate::HIGHLIGHT_SCALE,
        1.0,
        crate::HIGHLIGHT_SCALE,
      ))
      .active(self.legal_target && self.interactive)
      .materials([MaterialAssignment::new(0, crate::assets::LEGAL_SQUARE)])
      .on_click(self.on_activate.clone().map_input(move |()| square));
    let piece = self.piece.map(|piece| {
      ChessPieceView {
        piece,
        square,
        reference: self.piece_reference.clone(),
        on_activate: self.on_activate.clone(),
        spawning: self.spawning,
        game: self.game.clone(),
        control: self.control.clone(),
      }
      .id(*piece.entity_id.as_uuid())
    });
    (surface, piece)
  }
}

impl Component for ChessPieceView {
  fn render(&self) -> impl Render {
    let square = self.square;
    let hit = world::BoxHitRegion::new()
      .size(Vector3::new(0.9, 1.5, 0.9))
      .position(crate::square_position(self.square))
      .scale(if self.spawning {
        Vector3::ZERO
      } else {
        Vector3::ONE
      })
      .rotation(if self.piece.color == Color::Black {
        Quaternion::new(0.0, 1.0, 0.0, 0.0)
      } else {
        Quaternion::IDENTITY
      })
      .reference(self.reference.clone());
    let hit = if self.piece.color == Color::White && !self.spawning {
      let entity = self.piece.entity_id;
      let start_control = self.control.clone();
      let start_game = self.game.clone();
      let end_control = self.control.clone();
      let end_game = self.game.clone();
      hit
        .draggable(DragMode::SnapToPointer)
        .on_drag_start(move || {
          start_control.dispatch(Some(&start_game), UiAction::BeginDrag(entity));
        })
        .on_drag_end(move |position| {
          end_control.dispatch(
            Some(&end_game),
            UiAction::EndDrag(entity, crate::square_at(position)),
          );
        })
    } else if self.piece.color == Color::Black {
      hit.on_click(self.on_activate.clone().map_input(move |()| square))
    } else {
      hit
    };
    hit
      .child(world::Prefab::at(crate::address(
        self.piece.color,
        self.piece.kind,
      )))
      .motion(MotionProps::new().motion_name("chess-piece"))
      .key(self.spawning)
  }
}

fn sequence(
  animation: &ChessAnimation,
  references: &[reactant::prelude::ObjectRef; 64],
) -> AnimationSequence {
  match animation {
    ChessAnimation::Movement {
      movement,
      sound,
      final_sound,
      ..
    } => {
      let capture_or_promotion = matches!(
        movement,
        Movement::Capture { .. } | Movement::Promotion { .. }
      );
      let arrival = crate::motion::arrival_duration(movement);
      let mut sequence = crate::motion::sequence(movement, references);
      sequence = sequence
        .play_sound(sound.clone())
        .at(SequencePosition::Absolute(if capture_or_promotion {
          arrival
        } else {
          Duration::ZERO
        }));
      if let Some(sound) = final_sound {
        sequence = sequence
          .play_sound(sound.clone())
          .at(SequencePosition::Absolute(arrival));
      }
      sequence
    }
  }
}

fn opening_sequence(
  mode: SessionStart,
  pieces: &[ChessPiece],
  references: &[reactant::prelude::ObjectRef; 64],
) -> AnimationSequence {
  let sound = if mode == SessionStart::Fresh {
    crate::audio::START_SOUND
  } else {
    crate::RESET_SOUND
  };
  let mut sequence = AnimationSequence::new().play_sound(sound);
  if mode == SessionStart::Refresh {
    return sequence;
  }
  let white = pieces
    .iter()
    .filter(|piece| piece.color == Color::White)
    .map(|piece| piece.entity_id)
    .collect::<Vec<_>>();
  let black = pieces
    .iter()
    .filter(|piece| piece.color == Color::Black)
    .map(|piece| piece.entity_id)
    .collect::<Vec<_>>();
  let maximum = white.len().max(black.len());
  let per_beat = maximum.div_ceil(crate::PIECE_SPAWN_BEAT_COUNT).max(1);
  let stages = maximum.div_ceil(per_beat);
  for beat in 0..stages {
    let start = beat * per_beat;
    let end = start + per_beat;
    let at = Duration::from_millis(
      crate::CRITICAL_FIRST_BEAT_OFFSET_MS + beat as u64 * crate::CRITICAL_BEAT_INTERVAL_MS,
    );
    for piece in white
      .get(start..end.min(white.len()))
      .into_iter()
      .flatten()
      .chain(black.get(start..end.min(black.len())).into_iter().flatten())
    {
      let reference = piece_reference(references, *piece);
      sequence = sequence
        .animate(
          MotionSelector::object(reference.clone()),
          StyleTarget::new()
            .local_scale_x(1.0)
            .local_scale_y(1.0)
            .local_scale_z(1.0),
          Transition::tween().duration_secs(0.2).ease(Easing::EaseOut),
        )
        .at(SequencePosition::Absolute(at))
        .particle_for(
          crate::assets::effects::PIECE_SPAWN,
          reference.local_point(Vector3::ZERO).capture_at_start(),
          Duration::from_millis(PIECE_SPAWN_EFFECT_LIFETIME_MS),
        )
        .at(SequencePosition::Absolute(at));
    }
  }
  sequence
    .animate(
      MotionSelector::ScopeRoot,
      StyleTarget::new().local_scale_factor_x(1.0),
      Transition::tween()
        .duration_secs(PIECE_SPAWN_SEQUENCE_DURATION_MS as f64 / 1_000.0)
        .ease(Easing::Linear),
    )
    .at(SequencePosition::Absolute(Duration::ZERO))
}

fn piece_reference(
  references: &[reactant::prelude::ObjectRef; 64],
  piece: ObjectId,
) -> reactant::prelude::ObjectRef {
  let index = piece
    .as_uuid()
    .as_bytes()
    .iter()
    .skip(10)
    .fold(0_usize, |value, byte| (value << 8) | usize::from(*byte));
  references[index].clone()
}
