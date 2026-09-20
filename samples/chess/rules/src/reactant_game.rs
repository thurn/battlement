//! Complete chess rules state and worker actions.

use std::time::Duration;

use battlement::AudioClipAddress;
use cozy_chess::{Board, Color, GameStatus, Move, Square};
use fastrand::Rng;
use reactant::rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game};

use crate::{
  ai,
  chess_prompt::{ChessPrompt, PromotionPrompt},
  position::{ChessPiece, ChessPosition, Movement},
};

/// One complete user action admitted to the bounded rules worker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChessAction {
  /// Completes a legal player move to one visible destination.
  MoveTo {
    /// Selected source square.
    from: Square,
    /// Visible destination square.
    to: Square,
  },
  /// Searches and commits one computer reply from an accepted player move.
  ComputerMove,
}

/// Semantic checkpoint consumed exactly once by the board presentation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChessAnimation {
  /// One player or computer movement and its composed sounds.
  Movement {
    /// Movement description consumed by the shared Motion owner.
    movement: Movement,
    /// Move, capture, castle, or promotion sound selected in Rust.
    sound: AudioClipAddress,
    /// Check or terminal sound scheduled after arrival.
    final_sound: Option<AudioClipAddress>,
    /// Semantic classification recorded with this accepted move.
    result: crate::visual_state::VisualState,
    /// Post-move position available before any following rules action.
    accepted_board: Board,
  },
}

/// Immutable logical chess snapshot rendered by the app.
#[derive(Clone)]
pub struct ChessState {
  position: ChessPosition,
  result: Option<crate::visual_state::VisualState>,
}

pub struct ChessGame;
pub(crate) struct ChessPolicy;
pub struct ChessContext {
  execution: ExecutionMode<ChessGame, ChessPolicy>,
  think_time: Duration,
  rng: Rng,
}

impl ChessState {
  /// Creates one complete playable chess position.
  pub fn new(board: Board) -> Self {
    Self::with_generation(board, 0)
  }

  pub(crate) fn with_generation(board: Board, generation: u32) -> Self {
    Self {
      position: ChessPosition::from_board(board, generation),
      result: None,
    }
  }

  /// Creates a restored playable position.
  pub fn resumed(board: Board) -> Self {
    Self::new(board)
  }

  /// Returns the current rules board.
  pub fn board(&self) -> &Board {
    &self.position.board
  }

  /// Returns one visible piece identity.
  pub fn piece(&self, square: Square) -> Option<ChessPiece> {
    self.position.piece(square)
  }

  pub(crate) fn legal_moves(&self, from: Square, to: Square) -> Vec<Move> {
    self.position.legal_moves(from, to)
  }

  /// Returns the semantic outcome recorded with the most recent accepted move.
  pub const fn result(&self) -> Option<crate::visual_state::VisualState> {
    self.result
  }
}

impl ChessAnimation {
  pub(crate) const fn result(&self) -> crate::visual_state::VisualState {
    match self {
      Self::Movement { result, .. } => *result,
    }
  }

  pub(crate) fn accepted_board(&self) -> &Board {
    match self {
      Self::Movement { accepted_board, .. } => accepted_board,
    }
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

  fn movement(&mut self, state: &ChessState, movement: Move) -> ChessAnimation {
    let moving = state
      .position
      .piece(movement.from)
      .expect("legal mover has a presentation identity");
    let description = state.position.movement(movement);
    let mut board_after = state.position.board.clone();
    board_after.play_unchecked(movement);
    let sound = match &description {
      Movement::Castle { .. } => crate::CASTLE_SOUND,
      Movement::Promotion { .. } => crate::PROMOTION_SOUND,
      Movement::Capture { .. } => {
        crate::CAPTURE_SOUNDS[self.rng.usize(..crate::CAPTURE_SOUNDS.len())].clone()
      }
      Movement::Move { .. } | Movement::Knight { .. } => {
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
      result: crate::visual_state::after_move(
        &state.position.board,
        &board_after,
        movement,
        moving.color,
      ),
      accepted_board: board_after,
    }
  }

  fn apply_move(&mut self, state: &mut ChessState, movement: Move) {
    let animation = self.movement(state, movement);
    let result = animation.result();
    self.execution.present(state, || animation);
    state.position.apply(movement);
    state.result = Some(result);
  }
}

impl ChoicePolicy<ChessGame> for ChessPolicy {
  fn owner(&self, _: &ChessState, prompt: &ChessPrompt<'_>) -> ChoiceOwner {
    match prompt {
      ChessPrompt::Promotion(_) => ChoiceOwner::Human,
    }
  }

  fn choose(&mut self, _: &ChessState, _: &ChessPrompt<'_>) -> usize {
    unreachable!("interactive chess prompts are owned by the player")
  }
}

impl Game for ChessGame {
  type State = ChessState;
  type Action = ChessAction;
  type StateAnimation = ChessAnimation;
  type Prompt<'a> = ChessPrompt<'a>;
  type Context = ChessContext;

  fn logical_clone(state: &ChessState) -> ChessState {
    state.clone()
  }

  fn is_legal_action(state: &ChessState, action: &ChessAction) -> bool {
    match action {
      ChessAction::MoveTo { from, to } => {
        state.board().side_to_move() == Color::White && !state.legal_moves(*from, *to).is_empty()
      }
      ChessAction::ComputerMove => {
        state.board().side_to_move() == Color::Black
          && state.board().status() == GameStatus::Ongoing
      }
    }
  }

  fn execute(context: &mut ChessContext, state: &mut ChessState, action: ChessAction) {
    match action {
      ChessAction::MoveTo { from, to } => {
        let candidates = state.legal_moves(from, to);
        let movement = if candidates.len() == 1 {
          candidates[0]
        } else {
          let promotion = context
            .execution
            .choose(state, PromotionPrompt::new(from, to));
          candidates
            .into_iter()
            .find(|movement| movement.promotion == Some(promotion))
            .expect("promotion response selects one legal move")
        };
        context.apply_move(state, movement);
      }
      ChessAction::ComputerMove => {
        let reply = ai::choose_move(&state.position.board, context.think_time)
          .expect("ongoing chess position has a computer move");
        context.apply_move(state, reply);
      }
    }
  }
}
