//! Declarative board, square, and piece components.

use std::time::Duration;

use battlement::{
  DragMode, GridLayout, ImageFit, MaterialAssignment, ParentScene, PrefabAddress, Quaternion,
  Vector3,
};
use cozy_chess::{Color, File, GameStatus, Piece, Rank, Square};
use reactant::{
  GameStatus as RulesStatus, SnapshotAnimation,
  animation_controls::{self, AnimationSequence, MotionSelector, SequencePosition},
  app_context, hooks,
  prelude::{
    AnimationPlayback, Component, Easing, EventCallback, IdentityRenderExt, KeyRenderExt,
    MotionComponentExt, MotionProps, ObjectRef, Render, StyleTarget, Transition, use_object_ref,
  },
  world::{BoxHitRegion, Group, Plane, Prefab, SceneRoot, Sprite},
};

use trox::ls;

use crate::{
  assets::{black, white},
  chess_ui_state::{ChessUiController, ChessUiState, SessionStart, UiAction},
  position::{ChessPiece, Movement, PieceIdentity},
  reactant_game::{ChessAnimation, ChessGame},
};
use battlement::ObjectId;

const REFRESH_BUTTON_ID: ObjectId = battlement::object_id!("35b288b3-6d72-48af-aeb9-e8f11d63e3ea");
const PIECE_SPAWN_SEQUENCE_DURATION_MS: u64 = 5_070;
const CAMERA_BUTTON_DEPTH: f64 = 1.5;
const CAMERA_VERTICAL_FOV_RADIANS: f64 = std::f64::consts::PI / 3.0;
const CRITICAL_BEAT_INTERVAL_MS: u64 = 570;
const CRITICAL_FIRST_BEAT_OFFSET_MS: u64 = 80;
const HIGHLIGHT_HEIGHT: f64 = 0.02;
const HIGHLIGHT_SCALE: f64 = 0.09;
const PIECE_SPAWN_BEAT_COUNT: usize = 8;
const PIECE_SPAWN_EFFECT_LIFETIME_MS: u64 = 1_000;
const REFRESH_BUTTON_MARGIN: f64 = 0.12;
const REFRESH_BUTTON_SIZE: f64 = 0.16;

/// Declarative world-space board composed from rules state and app-local UI state.
///
/// The component receives callbacks and handles as props instead of reaching into
/// a global model. This keeps rendering reusable while the parent owns session
/// orchestration, persistence, and input policy.
pub struct ChessBoard {
  /// Current app-local selection, cursor, overlay, and opening state.
  pub ui: ChessUiState,
  /// Unified pointer activation callback for squares and opposing pieces.
  pub on_activate: EventCallback<Square>,
  /// Callback for the world-space refresh affordance.
  pub on_request_new_game: EventCallback<()>,
  /// Registers completion of the session-opening animation.
  pub on_opening_finished: std::rc::Rc<dyn Fn(AnimationPlayback, u64)>,
  /// Typed rules-session handle used by draggable pieces.
  pub game: reactant::GameHandle<ChessGame>,
  /// Reducer-style controller shared with other input paths.
  pub control: ChessUiController,
}

/// One board cell, including its interaction surface and optional piece.
struct ChessSquare {
  square: Square,
  legal_target: bool,
  piece: Option<ChessPiece>,
  piece_reference: ObjectRef,
  on_activate: EventCallback<Square>,
  spawning: bool,
  game: reactant::GameHandle<ChessGame>,
  control: ChessUiController,
  interactive: bool,
}

/// Hit region and prefab for a piece with stable identity and Motion targeting.
struct ChessPieceView {
  piece: ChessPiece,
  square: Square,
  reference: ObjectRef,
  on_activate: EventCallback<Square>,
  spawning: bool,
  game: reactant::GameHandle<ChessGame>,
  control: ChessUiController,
}

impl Component for ChessBoard {
  /// Renders the board and declares effects that bridge committed state to Motion.
  ///
  /// Reactant components describe both native UI and world objects. Hooks stay at
  /// the top level of `render`, while event closures capture cloned handles rather
  /// than borrowing the transient render frame.
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<ChessGame>();
    let status = reactant::use_game_status::<ChessGame>();
    let screen = app_context::use_viewport_size();
    let local = &self.ui;
    let scope = animation_controls::use_animation_scope();
    let event_scope = scope.clone();
    // Reserve one hook-backed reference per original piece slot. Piece identities
    // retain that slot across moves, so animations never depend on tree position.
    let references: [ObjectRef; 64] = std::array::from_fn(|_| use_object_ref());
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
    // Starting Motion after commit guarantees all newly mounted piece references
    // are attached before the sequence tries to animate them.
    hooks::use_commit_effect(
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
    let restore_state = state.clone();
    let drag_restore = local.drag_restore;
    hooks::use_commit_effect(
      move || {
        if let Some((piece, square)) = drag_restore
          && let Some(piece) = restore_state.piece_with_id(piece)
        {
          restore_scope.set(
            MotionSelector::object(piece_reference(&restore_references, piece.identity)),
            crate::motion::position_target(square),
          );
        }
      },
      local.drag_restore_generation,
    );

    let legal = local
      .selected
      .map(|square| state.legal_destinations(square))
      .unwrap_or_default();
    let interactive = status == RulesStatus::Ready
      && state.board().status() == GameStatus::Ongoing
      && state.board().side_to_move() == Color::White
      && !local.pause_open()
      && local.screen == crate::chess_ui_state::AppScreen::Game;
    let squares = Square::ALL
      .into_iter()
      .filter(|square| state.piece(*square).is_some() || legal.contains(square))
      .map(|square| {
        ChessSquare {
          square,
          legal_target: legal.contains(&square),
          piece: state.piece(square),
          piece_reference: state
            .piece(square)
            .map(|piece| piece_reference(&references, piece.identity))
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
    let cursor_active = local.cursor_visible || local.selected.is_some();
    let cursor_scale = if status == RulesStatus::Busy {
      0.55
    } else {
      1.0
    };
    let cursor = Group::new()
      .id(*crate::cursor::EFFECT_ID.as_uuid())
      .position(square_position(local.cursor))
      .scale(Vector3::new(cursor_scale, cursor_scale, cursor_scale))
      .active(cursor_active)
      .child(Prefab::at(crate::assets::effects::PIECE_SELECTED));
    let refresh = local.pause_open().then(|| {
      let aspect = if screen.height == 0 {
        1.0
      } else {
        f64::from(screen.width) / f64::from(screen.height)
      };
      let half_height = CAMERA_BUTTON_DEPTH * (CAMERA_VERTICAL_FOV_RADIANS / 2.0).tan();
      let right = half_height * aspect - REFRESH_BUTTON_SIZE / 2.0 - REFRESH_BUTTON_MARGIN;
      let up = half_height - REFRESH_BUTTON_SIZE / 2.0 - REFRESH_BUTTON_MARGIN;
      BoxHitRegion::new()
        .size(Vector3::new(REFRESH_BUTTON_SIZE, REFRESH_BUTTON_SIZE, 0.02))
        .position(Vector3::new(
          right,
          8.0 - 0.946201 * CAMERA_BUTTON_DEPTH + 0.323579 * up,
          -3.75 + 0.323579 * CAMERA_BUTTON_DEPTH + 0.946201 * up,
        ))
        .rotation(crate::reactant_view::CAMERA_ROTATION)
        .on_click(self.on_request_new_game.clone())
        .accessible_button(
          ls(if local.confirm_new_game() {
            "Confirm new game"
          } else {
            "New game"
          }),
          self.on_request_new_game.clone(),
        )
        .child(
          Sprite::new()
            .id(*REFRESH_BUTTON_ID.as_uuid())
            .texture(crate::assets::REFRESH_BUTTON)
            .size(REFRESH_BUTTON_SIZE, REFRESH_BUTTON_SIZE)
            .fit(ImageFit::Stretch),
        )
    });

    (SceneRoot::new(ParentScene::PrimaryScene).child(
      Group::new()
        .child((squares, cursor, refresh))
        .motion(MotionProps::new().animation_scope(scope)),
    ),)
  }
}

impl Component for ChessSquare {
  /// Renders a legal-target surface and the keyed piece occupying this square.
  fn render(&self) -> impl Render {
    let square = self.square;
    let mut position = square_position(square);
    position.y = HIGHLIGHT_HEIGHT;
    let surface = (self.legal_target && self.interactive).then(|| {
      Plane::new()
        .position(position)
        .scale(Vector3::new(HIGHLIGHT_SCALE, 1.0, HIGHLIGHT_SCALE))
        .materials([MaterialAssignment::new(0, crate::assets::LEGAL_SQUARE)])
        .on_click(self.on_activate.clone().map_input(move |()| square))
        .accessible_button(
          ls(format!("Move to {square}")),
          self.on_activate.clone().map_input(move |()| square),
        )
    });
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
      .id(*piece.identity.object_id.as_uuid())
    });
    (surface, piece)
  }
}

impl Component for ChessPieceView {
  /// Renders one stable hit region and chooses interaction by piece ownership.
  fn render(&self) -> impl Render {
    let square = self.square;
    let hit = BoxHitRegion::new()
      .size(Vector3::new(0.9, 1.5, 0.9))
      .position(square_position(self.square))
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
      let entity = self.piece.identity.object_id;
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
            UiAction::EndDrag(entity, square_at(position)),
          );
        })
    } else if self.piece.color == Color::Black {
      hit.on_click(self.on_activate.clone().map_input(move |()| square))
    } else {
      hit
    };
    hit
      .accessible_button(
        ls(format!(
          "{:?} {:?} at {}",
          self.piece.color, self.piece.kind, square
        )),
        self.on_activate.clone().map_input(move |()| square),
      )
      .child(Prefab::at(address(self.piece.color, self.piece.kind)))
      .motion(MotionProps::new().motion_name("chess-piece"))
      .key(self.spawning)
  }
}

/// Adds event-specific audio timing to the shared movement animation.
///
/// Motion owns spatial composition; this board-level adapter owns sounds because
/// it can schedule them against the same sequence clock delivered to the host.
fn sequence(animation: &ChessAnimation, references: &[ObjectRef; 64]) -> AnimationSequence {
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

/// Builds the staged piece reveal used when mounting a fresh or restarted session.
///
/// White and black pieces enter in parallel groups aligned to the music beats.
/// A refresh skips the reveal because its existing board is already visible.
fn opening_sequence(
  mode: SessionStart,
  pieces: &[ChessPiece],
  references: &[ObjectRef; 64],
) -> AnimationSequence {
  let sound = if mode == SessionStart::Fresh {
    crate::audio::START_SOUND
  } else {
    crate::audio::RESET_SOUND
  };
  let mut sequence = AnimationSequence::new().play_sound(sound);
  if mode == SessionStart::Refresh {
    return sequence;
  }
  let white = pieces
    .iter()
    .filter(|piece| piece.color == Color::White)
    .map(|piece| piece.identity)
    .collect::<Vec<_>>();
  let black = pieces
    .iter()
    .filter(|piece| piece.color == Color::Black)
    .map(|piece| piece.identity)
    .collect::<Vec<_>>();
  let maximum = white.len().max(black.len());
  let per_beat = maximum.div_ceil(PIECE_SPAWN_BEAT_COUNT).max(1);
  let stages = maximum.div_ceil(per_beat);
  for beat in 0..stages {
    let start = beat * per_beat;
    let end = start + per_beat;
    let at = Duration::from_millis(
      CRITICAL_FIRST_BEAT_OFFSET_MS + beat as u64 * CRITICAL_BEAT_INTERVAL_MS,
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

/// Resolves a piece's explicit reference slot into the board's hook-backed reference.
fn piece_reference(references: &[ObjectRef; 64], piece: PieceIdentity) -> ObjectRef {
  references[piece.reference_slot].clone()
}

fn square_at(position: Vector3) -> Option<Square> {
  if !(-4.0..4.0).contains(&position.x) || !(-4.0..4.0).contains(&position.z) {
    return None;
  }
  let file = (position.x + 3.5).round() as usize;
  let rank = (position.z + 3.5).round() as usize;
  Some(Square::new(File::index(file), Rank::index(rank)))
}

pub(super) fn square_position(square: Square) -> Vector3 {
  GridLayout::centered(
    Vector3::ZERO,
    8,
    8,
    Vector3::new(1.0, 0.0, 0.0),
    Vector3::new(0.0, 0.0, 1.0),
  )
  .position(square.file() as u32, square.rank() as u32)
}

fn address(color: Color, piece: Piece) -> PrefabAddress {
  match (color, piece) {
    (Color::White, Piece::Pawn) => white::PAWN,
    (Color::White, Piece::Rook) => white::ROOK,
    (Color::White, Piece::Knight) => white::KNIGHT,
    (Color::White, Piece::Bishop) => white::BISHOP,
    (Color::White, Piece::Queen) => white::QUEEN,
    (Color::White, Piece::King) => white::KING,
    (Color::Black, Piece::Pawn) => black::PAWN,
    (Color::Black, Piece::Rook) => black::ROOK,
    (Color::Black, Piece::Knight) => black::KNIGHT,
    (Color::Black, Piece::Bishop) => black::BISHOP,
    (Color::Black, Piece::Queen) => black::QUEEN,
    (Color::Black, Piece::King) => black::KING,
  }
}
