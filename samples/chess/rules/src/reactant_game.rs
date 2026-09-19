//! Complete chess rules state and worker actions for the Reactant alternate app.

use std::time::Duration;

use battlement::{AudioClipAddress, ObjectId};
use cozy_chess::{Board, Color, GameStatus, Move, Square};
use fastrand::Rng;
use reactant::rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game};

use crate::{
  ai, audio,
  reactant_fixture::{FixtureAnimation, FixtureMove, FixturePiece, FixtureState},
  visual_state::VisualState,
};

/// File operation scheduled after a state becomes accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistenceDirective {
  /// Leave the durable save unchanged.
  None,
  /// Persist the accepted board.
  Save,
  /// Remove the durable save after accepting a shortcut restart.
  Clear,
}

/// Start presentation selected by app-owned replacement state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartMode {
  /// First play from the title screen.
  Fresh,
  /// Desktop shortcut restart with opening beats.
  Restart,
  /// Confirmed new game without replaying the spawn choreography.
  Refresh,
}

/// One complete user action admitted to the bounded rules worker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChessAction {
  /// Starts the app-owned board state.
  Start(StartMode),
  /// Commits one legal visible-square player move.
  Move(FixtureMove),
  /// Searches and commits one computer reply from an accepted player move.
  AiMove,
}

/// Semantic checkpoint consumed exactly once by the board presentation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChessAnimation {
  /// Opening or reset presentation.
  Opening {
    /// Whether to play the eight spawn beats.
    spawn: bool,
    /// Stable piece identities grouped by beat.
    beats: Vec<Vec<ObjectId>>,
    /// One-shot start or reset sound.
    sound: AudioClipAddress,
  },
  /// One player or computer movement and its composed sounds.
  Movement {
    /// Existing movement description shared with the task-31 fixture.
    movement: FixtureAnimation,
    /// Move, capture, castle, or promotion sound selected in Rust.
    sound: AudioClipAddress,
    /// Check or terminal sound scheduled after arrival.
    final_sound: Option<AudioClipAddress>,
  },
}

/// Immutable logical chess snapshot rendered by the alternate app.
#[derive(Clone)]
pub struct ChessState {
  position: FixtureState,
  starting_board: Board,
  started: bool,
  spawning: bool,
  origin_saved: bool,
  generation: u32,
  visual_state: VisualState,
  persistence_revision: u64,
  persistence: PersistenceDirective,
}

pub(crate) struct ChessGame;
pub(crate) struct ChessPolicy;
pub(crate) struct ChessContext {
  execution: ExecutionMode<ChessGame, ChessPolicy>,
  think_time: Duration,
  rng: Rng,
}

impl ChessState {
  /// Creates a title state with no visible pieces.
  pub fn title(starting_board: Board) -> Self {
    Self::title_generation(starting_board, 0)
  }

  pub(crate) fn title_generation(starting_board: Board, generation: u32) -> Self {
    Self {
      position: FixtureState::from_board(starting_board.clone(), generation),
      starting_board,
      started: false,
      spawning: false,
      origin_saved: false,
      generation,
      visual_state: VisualState::Title,
      persistence_revision: 0,
      persistence: PersistenceDirective::None,
    }
  }

  /// Creates a restored playable state without replaying transient effects.
  pub fn resumed(board: Board) -> Self {
    Self {
      position: FixtureState::from_board(board.clone(), 0),
      starting_board: Board::default(),
      started: true,
      spawning: false,
      origin_saved: true,
      generation: 0,
      visual_state: VisualState::Resumed,
      persistence_revision: 0,
      persistence: PersistenceDirective::None,
    }
  }

  pub(crate) fn review(board: Board, visual_state: VisualState) -> Self {
    Self {
      position: FixtureState::from_board(board.clone(), 0),
      starting_board: board,
      started: true,
      spawning: false,
      origin_saved: visual_state == VisualState::Resumed,
      generation: 0,
      visual_state,
      persistence_revision: 0,
      persistence: PersistenceDirective::None,
    }
  }

  /// Returns the current rules board.
  pub fn board(&self) -> &Board {
    &self.position.board
  }

  /// Returns one visible piece identity.
  pub fn piece(&self, square: Square) -> Option<FixturePiece> {
    self.position.piece(square)
  }

  /// Whether the playable board is mounted.
  pub const fn started(&self) -> bool {
    self.started
  }

  /// Whether the opening checkpoint initially hides all pieces.
  pub const fn spawning(&self) -> bool {
    self.spawning
  }

  /// Whether this session originated from a durable saved game.
  pub const fn origin_saved(&self) -> bool {
    self.origin_saved
  }

  /// Current semantic visual classification.
  pub const fn visual_state(&self) -> VisualState {
    self.visual_state
  }

  /// Monotonic accepted-save scheduling generation.
  pub const fn persistence_revision(&self) -> u64 {
    self.persistence_revision
  }

  /// File operation belonging to this accepted state.
  pub const fn persistence(&self) -> PersistenceDirective {
    self.persistence
  }

  pub(crate) fn legal_move(&self, action: FixtureMove) -> Option<Move> {
    self
      .started
      .then(|| self.position.legal_move(action))
      .flatten()
  }
}

impl ChessContext {
  pub(crate) fn new(
    execution: ExecutionMode<ChessGame, ChessPolicy>,
    think_time: Duration,
    rng: Rng,
  ) -> Self {
    Self {
      execution,
      think_time,
      rng,
    }
  }

  fn opening(&mut self, state: &ChessState, mode: StartMode) -> ChessAnimation {
    let spawn = mode != StartMode::Refresh;
    let mut white = Vec::new();
    let mut black = Vec::new();
    for square in Square::ALL {
      if let Some(piece) = state.position.piece(square) {
        if piece.color == Color::White {
          white.push(piece.id);
        } else {
          black.push(piece.id);
        }
      }
    }
    self.rng.shuffle(&mut white);
    self.rng.shuffle(&mut black);
    let maximum = white.len().max(black.len());
    let per_beat = maximum.div_ceil(crate::PIECE_SPAWN_BEAT_COUNT).max(1);
    let stages = maximum.div_ceil(per_beat);
    let beats = (0..stages)
      .map(|index| {
        let start = index * per_beat;
        let end = start + per_beat;
        white
          .get(start..end.min(white.len()))
          .into_iter()
          .flatten()
          .chain(black.get(start..end.min(black.len())).into_iter().flatten())
          .copied()
          .collect()
      })
      .collect();
    ChessAnimation::Opening {
      spawn,
      beats,
      sound: if mode == StartMode::Fresh {
        audio::START_SOUND
      } else {
        crate::RESET_SOUND
      },
    }
  }

  fn movement(&mut self, state: &ChessState, movement: Move) -> ChessAnimation {
    let moving = state
      .position
      .piece(movement.from)
      .expect("legal mover has a presentation identity");
    let description = state.position.animation(movement);
    let mut board_after = state.position.board.clone();
    board_after.play_unchecked(movement);
    let sound = match &description {
      FixtureAnimation::Castle { .. } => crate::CASTLE_SOUND,
      FixtureAnimation::Promotion { .. } => crate::PROMOTION_SOUND,
      FixtureAnimation::Capture { .. } => {
        crate::CAPTURE_SOUNDS[self.rng.usize(..crate::CAPTURE_SOUNDS.len())].clone()
      }
      FixtureAnimation::Move { .. } | FixtureAnimation::Knight { .. } => {
        crate::DROP_SOUNDS[self.rng.usize(..crate::DROP_SOUNDS.len())].clone()
      }
    };
    let final_sound = match board_after.status() {
      GameStatus::Won if board_after.side_to_move() == Color::Black => {
        Some(crate::PLAYER_WIN_SOUND)
      }
      GameStatus::Won => Some(crate::PLAYER_LOSS_SOUND),
      GameStatus::Drawn => Some(crate::DRAW_SOUND),
      GameStatus::Ongoing if !board_after.checkers().is_empty() => Some(crate::CHECK_SOUND),
      GameStatus::Ongoing => None,
    };
    debug_assert_eq!(
      moving.kind,
      state.position.board.piece_on(movement.from).unwrap()
    );
    ChessAnimation::Movement {
      movement: description,
      sound,
      final_sound,
    }
  }

  fn apply_move(&mut self, state: &mut ChessState, movement: Move) {
    let board_before = state.position.board.clone();
    let mover = board_before
      .color_on(movement.from)
      .expect("legal mover has a color");
    let animation = self.movement(state, movement);
    self.execution.present(state, || animation);
    state.position.apply(movement);
    state.visual_state =
      crate::visual_state::after_move(&board_before, &state.position.board, movement, mover);
  }
}

impl ChoicePolicy<ChessGame> for ChessPolicy {
  fn owner(&self, _: &ChessState, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }

  fn choose(&mut self, _: &ChessState, _: &()) -> usize {
    unreachable!("chess has no typed prompts")
  }
}

impl Game for ChessGame {
  type State = ChessState;
  type Action = ChessAction;
  type StateAnimation = ChessAnimation;
  type Prompt<'a> = ();
  type Context = ChessContext;

  fn logical_clone(state: &ChessState) -> ChessState {
    state.clone()
  }

  fn is_legal_action(state: &ChessState, action: &ChessAction) -> bool {
    match action {
      ChessAction::Start(_) => !state.started,
      ChessAction::Move(action) => {
        state.board().side_to_move() == Color::White && state.legal_move(*action).is_some()
      }
      ChessAction::AiMove => {
        state.started
          && state.board().side_to_move() == Color::Black
          && state.board().status() == GameStatus::Ongoing
      }
    }
  }

  fn execute(context: &mut ChessContext, state: &mut ChessState, action: ChessAction) {
    match action {
      ChessAction::Start(mode) => {
        state.generation = state
          .generation
          .checked_add(1)
          .expect("piece generation overflow");
        state.position = FixtureState::from_board(state.starting_board.clone(), state.generation);
        state.started = true;
        state.spawning = mode != StartMode::Refresh;
        state.origin_saved = false;
        state.visual_state = match mode {
          StartMode::Fresh => VisualState::Initial,
          StartMode::Restart => VisualState::Restarted,
          StartMode::Refresh => VisualState::Refreshed,
        };
        let opening = context.opening(state, mode);
        context.execution.present(state, || opening);
        state.spawning = false;
        state.persistence = if mode == StartMode::Restart {
          PersistenceDirective::Clear
        } else {
          PersistenceDirective::Save
        };
      }
      ChessAction::Move(action) => {
        let movement = state
          .legal_move(action)
          .expect("worker receives a checked player move");
        context.apply_move(state, movement);
        state.persistence = PersistenceDirective::Save;
      }
      ChessAction::AiMove => {
        let reply = ai::choose_move(&state.position.board, context.think_time)
          .expect("ongoing chess position has a computer move");
        context.apply_move(state, reply);
        state.persistence = PersistenceDirective::Save;
      }
    }
    state.persistence_revision = state
      .persistence_revision
      .checked_add(1)
      .expect("persistence revision overflow");
  }
}
