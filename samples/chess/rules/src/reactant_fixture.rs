//! Deterministic Reactant review fixture for chess board movement.

use std::{cell::RefCell, time::Duration};

use battlement::{MaterialAssignment, ObjectId, ParentScene, Quaternion, Vector3, object_id};
use battlement_native::{Engine, EngineError, EngineResponse, FlatBufferSubmitError};
use cozy_chess::{Board, Color, File, Move, Piece, Rank, Square};
use reactant::{
  GameConsumer, GameHandle,
  animation_controls::{AnimationSequence, MotionSelector, SequencePosition},
  app::App,
  key::KeyRenderExt,
  prelude::{
    Button, Component, Easing, GameApp, GameRoot, MotionComponentExt, MotionProps, Render,
    SnapshotAnimation, StyleTarget, Transition, use_object_ref,
  },
  rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game},
  world,
};

use crate::{assets, player_move};
use trox::ls;

const MOVE_DURATION: Duration = Duration::from_millis(300);
const KNIGHT_FIRST_LEG: Duration = Duration::from_millis(200);
const KNIGHT_SECOND_LEG: Duration = Duration::from_millis(120);
const CAPTURE_EFFECT_LIFETIME: Duration = Duration::from_millis(2_000);
const REVIEW_ROOT: ObjectId = object_id!("43000000-0000-4000-8400-000000000001");

/// A visible-square move accepted by the deterministic fixture adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixtureMove {
  /// Occupied source square.
  pub from: Square,
  /// Player-visible destination, including C/G for castling.
  pub to: Square,
}

/// Stable identity and visible kind of one chess piece.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixturePiece {
  /// Stable outer Reactant presentation identity.
  pub id: ObjectId,
  /// Piece color.
  pub color: Color,
  /// Visible opaque prefab kind.
  pub kind: Piece,
}

/// Immutable logical snapshot rendered by the fixture board.
#[derive(Clone)]
pub struct FixtureState {
  pub(crate) board: Board,
  pub(crate) pieces: [Option<FixturePiece>; 64],
}

/// Typed movement description authored by the rules worker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FixtureAnimation {
  /// One straight or diagonal move.
  Move {
    /// Moving identity.
    piece: ObjectId,
    /// Destination square.
    to: Square,
  },
  /// One L-shaped move with an explicit corner.
  Knight {
    /// Moving identity.
    piece: ObjectId,
    /// Intermediate corner.
    corner: Square,
    /// Destination square.
    to: Square,
  },
  /// A move that retains its victim until arrival.
  Capture {
    /// Moving identity.
    piece: ObjectId,
    /// Captured identity.
    captured: ObjectId,
    /// Capture-effect square, distinct from the destination for en passant.
    capture_at: Square,
    /// Destination square.
    to: Square,
    /// Whether the moving piece follows the two-leg knight path.
    knight_corner: Option<Square>,
  },
  /// Parallel king and rook movement.
  Castle {
    /// King identity.
    king: ObjectId,
    /// King's visible destination.
    king_to: Square,
    /// Rook identity.
    rook: ObjectId,
    /// Rook destination.
    rook_to: Square,
  },
  /// Pawn arrival followed by an inner-prefab replacement.
  Promotion {
    /// Stable pawn/promoted-piece identity.
    piece: ObjectId,
    /// Optional captured identity.
    captured: Option<ObjectId>,
    /// Destination square.
    to: Square,
  },
}

struct FixtureGame;
struct FixturePolicy;
struct FixtureContext(ExecutionMode<FixtureGame, FixturePolicy>);
struct FixtureBoard {
  game: GameHandle<FixtureGame>,
  review_action: Option<FixtureMove>,
}
struct PieceView {
  piece: FixturePiece,
  square: Square,
  reference: reactant::prelude::ObjectRef,
}

/// App-owned worker handles for public fixture synchronization.
pub struct FixtureModel {
  consumer: RefCell<Option<GameConsumer<FixtureGame>>>,
  game: RefCell<Option<GameHandle<FixtureGame>>>,
}

/// Alternate Reactant engine used only by review and public display scenarios.
pub struct ReactantChessFixture {
  fen: String,
  review_action: Option<FixtureMove>,
  app: App<FixtureModel>,
}

impl FixtureState {
  /// Builds a deterministic identity map for an existing legal chess position.
  pub fn from_fen(fen: &str) -> Result<Self, String> {
    let board = fen
      .parse::<Board>()
      .map_err(|error| format!("invalid chess fixture position: {error}"))?;
    Ok(Self::from_board(board, 0))
  }

  pub(crate) fn from_board(board: Board, generation: u32) -> Self {
    let pieces = std::array::from_fn(|index| {
      let square = Square::index(index);
      Some(FixturePiece {
        id: crate::piece_id(index, generation),
        color: board.color_on(square)?,
        kind: board.piece_on(square)?,
      })
    });
    Self { board, pieces }
  }

  /// Returns the piece currently occupying a square.
  pub fn piece(&self, square: Square) -> Option<FixturePiece> {
    self.pieces[square as usize]
  }

  pub(crate) fn legal_move(&self, action: FixtureMove) -> Option<Move> {
    player_move(&self.board, action.from, action.to)
  }

  pub(crate) fn apply(&mut self, movement: Move) {
    let color = self
      .board
      .color_on(movement.from)
      .expect("legal mover has a color");
    let piece = self
      .board
      .piece_on(movement.from)
      .expect("legal mover has a kind");
    if piece == Piece::King && self.board.color_on(movement.to) == Some(color) {
      self.apply_castle(movement, color);
    } else {
      let capture = capture_square(&self.board, movement, piece);
      self.pieces[capture as usize] = None;
      let mut moving = self.pieces[movement.from as usize]
        .take()
        .expect("legal mover has an identity");
      if let Some(promoted) = movement.promotion {
        moving.kind = promoted;
      }
      self.pieces[movement.to as usize] = Some(moving);
    }
    self.board.play_unchecked(movement);
  }

  fn apply_castle(&mut self, movement: Move, color: Color) {
    let (king_to, rook_to) = castle_destinations(movement, color);
    let king = self.pieces[movement.from as usize]
      .take()
      .expect("castling king has an identity");
    let rook = self.pieces[movement.to as usize]
      .take()
      .expect("castling rook has an identity");
    self.pieces[king_to as usize] = Some(king);
    self.pieces[rook_to as usize] = Some(rook);
  }

  pub(crate) fn animation(&self, movement: Move) -> FixtureAnimation {
    let moving = self
      .piece(movement.from)
      .expect("legal mover has an identity");
    if moving.kind == Piece::King && self.board.color_on(movement.to) == Some(moving.color) {
      let (king_to, rook_to) = castle_destinations(movement, moving.color);
      return FixtureAnimation::Castle {
        king: moving.id,
        king_to,
        rook: self
          .piece(movement.to)
          .expect("castling rook has an identity")
          .id,
        rook_to,
      };
    }
    let capture_at = capture_square(&self.board, movement, moving.kind);
    let captured = self.piece(capture_at).map(|piece| piece.id);
    if movement.promotion.is_some() {
      return FixtureAnimation::Promotion {
        piece: moving.id,
        captured,
        to: movement.to,
      };
    }
    let knight_corner =
      (moving.kind == Piece::Knight).then(|| knight_corner(movement.from, movement.to));
    if let Some(captured) = captured {
      return FixtureAnimation::Capture {
        piece: moving.id,
        captured,
        capture_at,
        to: movement.to,
        knight_corner,
      };
    }
    if let Some(corner) = knight_corner {
      FixtureAnimation::Knight {
        piece: moving.id,
        corner,
        to: movement.to,
      }
    } else {
      FixtureAnimation::Move {
        piece: moving.id,
        to: movement.to,
      }
    }
  }
}

impl ReactantChessFixture {
  /// Creates the named alternate fixture without changing the sample's default engine.
  pub fn from_fen(fen: &str) -> Result<Self, String> {
    Self::with_scripted_action(fen, None)
  }

  /// Resolves a native review fixture selected through Ditto's semantic-fixture input.
  pub fn named_review(name: &str) -> Result<Option<Self>, String> {
    let (fen, action) = match name {
      "chess-reactant-normal" => (
        "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
        FixtureMove {
          from: Square::E2,
          to: Square::E4,
        },
      ),
      "chess-reactant-capture" => (
        "4k3/8/8/4p3/3B4/8/8/4K3 w - - 0 1",
        FixtureMove {
          from: Square::D4,
          to: Square::E5,
        },
      ),
      "chess-reactant-castle" => (
        "4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1",
        FixtureMove {
          from: Square::E1,
          to: Square::G1,
        },
      ),
      "chess-reactant-en-passant" => (
        "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
        FixtureMove {
          from: Square::E5,
          to: Square::D6,
        },
      ),
      "chess-reactant-promotion" => (
        "1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1",
        FixtureMove {
          from: Square::A7,
          to: Square::B8,
        },
      ),
      _ => return Ok(None),
    };
    Self::with_scripted_action(fen, Some(action)).map(Some)
  }

  fn with_scripted_action(fen: &str, review_action: Option<FixtureMove>) -> Result<Self, String> {
    let state = FixtureState::from_fen(fen)?;
    Ok(Self {
      fen: fen.to_owned(),
      review_action,
      app: fixture_app(state, review_action),
    })
  }

  /// Dispatches one visible-square action through the rules worker.
  pub fn dispatch(&self, action: FixtureMove) -> reactant::DispatchResult {
    self
      .app
      .model()
      .game
      .borrow()
      .as_ref()
      .unwrap()
      .dispatch(action)
  }

  /// Waits for a publication without advancing virtual presentation time or frames.
  pub fn wait_for_output(&self, timeout: Duration) -> bool {
    self
      .app
      .model()
      .consumer
      .borrow()
      .as_ref()
      .unwrap()
      .wait_for_output(timeout)
  }

  /// Waits for current rules execution to stop.
  pub fn wait_for_worker_stopped(&self, timeout: Duration) -> bool {
    self
      .app
      .model()
      .consumer
      .borrow()
      .as_ref()
      .unwrap()
      .wait_for_worker_stopped(timeout)
  }

  /// Returns the accepted immutable state.
  pub fn accepted_state(&self) -> FixtureState {
    self
      .app
      .model()
      .game
      .borrow()
      .as_ref()
      .unwrap()
      .accepted_state()
  }

  /// Returns current worker/submission readiness.
  pub fn status(&self) -> reactant::GameStatus {
    self.app.model().game.borrow().as_ref().unwrap().status()
  }

  /// Resolves one committed presentation identity to its stable outer native host.
  pub fn native_piece(&self, piece: ObjectId) -> Option<ObjectId> {
    self
      .app
      .presentation(*piece.as_uuid())
      .and_then(|observation| observation.native_objects.first().copied())
  }
}

impl Engine for ReactantChessFixture {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_CONTRACT_DIGEST_C;

  fn connect(
    &mut self,
    message: battlement_native::ConnectView<'_>,
  ) -> Result<EngineResponse, EngineError> {
    self.app = fixture_app(
      FixtureState::from_fen(&self.fen).expect("stored fixture FEN remains valid"),
      self.review_action,
    );
    Engine::connect(&mut self.app, message)
  }

  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    Engine::submit(&mut self.app, bytes)
  }

  fn submit_ui_event(
    &mut self,
    action: battlement_native::UiEventActionView<'_>,
  ) -> Result<battlement_native::UiEventResult, EngineError> {
    Engine::submit_ui_event(&mut self.app, action)
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    Engine::poll(&mut self.app)
  }
}

impl FixtureModel {
  fn new() -> Self {
    Self {
      consumer: RefCell::new(None),
      game: RefCell::new(None),
    }
  }
}

impl ChoicePolicy<FixtureGame> for FixturePolicy {
  fn owner(&self, _: &FixtureState, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }

  fn choose(&mut self, _: &FixtureState, _: &()) -> usize {
    unreachable!("the chess movement fixture has no prompts")
  }
}

impl Game for FixtureGame {
  type State = FixtureState;
  type Action = FixtureMove;
  type StateAnimation = FixtureAnimation;
  type Prompt<'a> = ();
  type Context = FixtureContext;

  fn logical_clone(state: &FixtureState) -> FixtureState {
    state.clone()
  }

  fn is_legal_action(state: &FixtureState, action: &FixtureMove) -> bool {
    state.legal_move(*action).is_some()
  }

  fn execute(context: &mut FixtureContext, state: &mut FixtureState, action: FixtureMove) {
    let movement = state
      .legal_move(action)
      .expect("worker receives a checked move");
    let animation = state.animation(movement);
    context.0.present(state, || animation);
    state.apply(movement);
  }
}

impl Component for FixtureBoard {
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<FixtureGame>();
    let scope = reactant::animation_controls::use_animation_scope();
    let event_scope = scope.clone();
    let references: [reactant::prelude::ObjectRef; 64] = std::array::from_fn(|_| use_object_ref());
    let event_references = references.clone();
    reactant::use_animate::<FixtureGame>(move |animation| {
      Some(SnapshotAnimation::sequence(
        event_scope.clone(),
        sequence(animation, &event_references),
      ))
    });
    let pieces = Square::ALL
      .into_iter()
      .filter_map(|square| {
        state.piece(square).map(|piece| {
          PieceView {
            piece,
            square,
            reference: piece_reference(&references, piece.id),
          }
          .key(piece.id)
        })
      })
      .collect::<Vec<_>>();
    let highlights = Square::ALL
      .into_iter()
      .map(|square| {
        let mut position = crate::square_position(square);
        position.y = crate::HIGHLIGHT_HEIGHT;
        world::Plane::new()
          .id(*crate::highlight_id(square as usize).as_uuid())
          .position(position)
          .scale(Vector3::new(
            crate::HIGHLIGHT_SCALE,
            1.0,
            crate::HIGHLIGHT_SCALE,
          ))
          .active(false)
          .materials([MaterialAssignment::new(0, assets::LEGAL_SQUARE)])
      })
      .collect::<Vec<_>>();
    let review_control = self.review_action.map(|action| {
      let game = self.game.clone();
      Button::new(ls("Run chess movement")).on_press(move || {
        assert_eq!(game.dispatch(action), reactant::DispatchResult::Started);
      })
    });
    (
      review_control,
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .child(highlights)
          .child(pieces)
          .motion(MotionProps::new().animation_scope(scope)),
      ),
    )
  }
}

fn fixture_app(state: FixtureState, review_action: Option<FixtureMove>) -> App<FixtureModel> {
  let mut app = App::with_model(assets::CONTENT, FixtureModel::new());
  let game = app.start_game::<FixtureGame>(state, |connection| {
    FixtureContext(ExecutionMode::Interactive {
      connection,
      policy: FixturePolicy,
    })
  });
  let consumer = app.game_consumer::<FixtureGame>();
  consumer.resume_automatic_submission();
  app.model().consumer.replace(Some(consumer));
  app.model().game.replace(Some(game.clone()));
  app
    .ui(GameRoot::new(FixtureBoard {
      game,
      review_action,
    }))
    .document(|mut document| {
      document.root_id = REVIEW_ROOT;
      document
    })
    .camera(|camera| {
      world::Camera::new()
        .perspective(60.0)
        .position(Vector3::new(0.0, 8.0, -3.75))
        .rotation(crate::CAMERA_ROTATION)
        .into_object(camera.object_id)
    })
}

impl Component for PieceView {
  fn render(&self) -> impl Render {
    world::Group::new()
      .id(*self.piece.id.as_uuid())
      .position(crate::square_position(self.square))
      .rotation(if self.piece.color == Color::Black {
        Quaternion::new(0.0, 1.0, 0.0, 0.0)
      } else {
        Quaternion::IDENTITY
      })
      .reference(self.reference.clone())
      .child(world::Prefab::at(crate::address(
        self.piece.color,
        self.piece.kind,
      )))
      .motion(MotionProps::new().motion_name("chess-piece"))
  }
}

pub(crate) fn sequence(
  animation: &FixtureAnimation,
  references: &[reactant::prelude::ObjectRef; 64],
) -> AnimationSequence {
  match *animation {
    FixtureAnimation::Move { piece, to } => move_step(references, piece, to, MOVE_DURATION),
    FixtureAnimation::Knight { piece, corner, to } => knight_steps(references, piece, corner, to),
    FixtureAnimation::Capture {
      piece,
      captured,
      capture_at: _,
      to,
      knight_corner,
    } => {
      let sequence = knight_corner.map_or_else(
        || move_step(references, piece, to, MOVE_DURATION),
        |corner| knight_steps(references, piece, corner, to),
      );
      sequence.particle_for(
        assets::effects::CAPTURE,
        piece_reference(references, captured)
          .local_point(Vector3::ZERO)
          .capture_at_start(),
        CAPTURE_EFFECT_LIFETIME,
      )
    }
    FixtureAnimation::Castle {
      king,
      king_to,
      rook,
      rook_to,
    } => move_step(references, king, king_to, MOVE_DURATION)
      .then(
        MotionSelector::object(piece_reference(references, rook)),
        position_target(rook_to),
        movement_transition(MOVE_DURATION),
      )
      .at(SequencePosition::WithPrevious(0.0)),
    FixtureAnimation::Promotion {
      piece,
      captured,
      to,
    } => {
      let mut sequence = move_step(references, piece, to, MOVE_DURATION);
      if let Some(captured) = captured {
        sequence = sequence.particle_for(
          assets::effects::CAPTURE,
          piece_reference(references, captured)
            .local_point(Vector3::ZERO)
            .capture_at_start(),
          CAPTURE_EFFECT_LIFETIME,
        );
      }
      sequence
    }
  }
}

fn move_step(
  references: &[reactant::prelude::ObjectRef; 64],
  piece: ObjectId,
  to: Square,
  duration: Duration,
) -> AnimationSequence {
  AnimationSequence::new().animate(
    MotionSelector::object(piece_reference(references, piece)),
    position_target(to),
    movement_transition(duration),
  )
}

fn knight_steps(
  references: &[reactant::prelude::ObjectRef; 64],
  piece: ObjectId,
  corner: Square,
  to: Square,
) -> AnimationSequence {
  move_step(references, piece, corner, KNIGHT_FIRST_LEG).then(
    MotionSelector::object(piece_reference(references, piece)),
    position_target(to),
    movement_transition(KNIGHT_SECOND_LEG),
  )
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
  references
    .get(index)
    .unwrap_or_else(|| panic!("fixture piece identity is outside the deterministic map: {piece}"))
    .clone()
}

fn position_target(square: Square) -> StyleTarget {
  let position = crate::square_position(square);
  StyleTarget::new()
    .local_position_x(position.x as f32)
    .local_position_y(position.y as f32)
    .local_position_z(position.z as f32)
}

fn movement_transition(duration: Duration) -> Transition {
  Transition::tween()
    .duration_secs(duration.as_secs_f64())
    .ease(Easing::InOutSine)
}

fn capture_square(board: &Board, movement: Move, piece: Piece) -> Square {
  if piece == Piece::Pawn
    && movement.from.file() != movement.to.file()
    && board.piece_on(movement.to).is_none()
  {
    Square::new(movement.to.file(), movement.from.rank())
  } else {
    movement.to
  }
}

fn castle_destinations(movement: Move, color: Color) -> (Square, Square) {
  let rank = if color == Color::White {
    Rank::First
  } else {
    Rank::Eighth
  };
  let short = movement.to.file() > movement.from.file();
  (
    Square::new(if short { File::G } else { File::C }, rank),
    Square::new(if short { File::F } else { File::D }, rank),
  )
}

fn knight_corner(from: Square, to: Square) -> Square {
  if (from.file() as i8 - to.file() as i8).unsigned_abs()
    > (from.rank() as i8 - to.rank() as i8).unsigned_abs()
  {
    Square::new(to.file(), from.rank())
  } else {
    Square::new(from.file(), to.rank())
  }
}
