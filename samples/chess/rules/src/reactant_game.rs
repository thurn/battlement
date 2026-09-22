//! Complete chess rules state and worker actions.

use battlement::{AudioClipAddress, ObjectId};
use cozy_chess::{Board, Color, GameStatus, Move, Square};
use fastrand::Rng;
use reactant::rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game};

use crate::{
  chess_prompt::{ChessPrompt, PromotionPrompt},
  opponent::Opponent,
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

/// Type-level description connecting chess state, actions, prompts, and animations.
pub struct ChessGame;

/// Choice policy used when rules execution encounters a typed prompt.
pub struct ChessPolicy;

/// Mutable services available only while the bounded rules worker executes.
///
/// Keeping presentation randomness and the display connection here avoids putting
/// nondeterministic or host-specific data into the clonable logical state.
pub struct ChessContext {
  execution: ExecutionMode<ChessGame, ChessPolicy>,
  opponent: Opponent,
  rng: Rng,
}

impl ChessState {
  /// Creates one complete playable chess position.
  pub fn new(board: Board) -> Self {
    Self::with_generation(board, 0)
  }

  /// Creates a position with identities owned by one mounted session.
  pub fn with_generation(board: Board, generation: u64) -> Self {
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

  /// Finds a piece by its host object identity.
  pub fn piece_with_id(&self, object_id: ObjectId) -> Option<ChessPiece> {
    self.position.piece_with_id(object_id)
  }

  /// Returns unique player-visible destinations for one source square.
  pub fn legal_destinations(&self, from: Square) -> Vec<Square> {
    self.position.legal_destinations(from)
  }

  /// Returns legal engine moves matching one player-visible source and target.
  pub fn legal_moves(&self, from: Square, to: Square) -> Vec<Move> {
    self.position.legal_moves(from, to)
  }

  /// Returns the semantic outcome recorded with the most recent accepted move.
  pub const fn result(&self) -> Option<crate::visual_state::VisualState> {
    self.result
  }
}

impl ChessAnimation {
  /// Returns the semantic result published alongside this animation checkpoint.
  pub const fn result(&self) -> crate::visual_state::VisualState {
    match self {
      Self::Movement { result, .. } => *result,
    }
  }

  /// Returns the post-move board captured before any following action can run.
  ///
  /// Persistence reads publications rather than waiting for rendered state, which
  /// keeps saved data aligned with the exact animation currently being presented.
  pub fn accepted_board(&self) -> &Board {
    match self {
      Self::Movement { accepted_board, .. } => accepted_board,
    }
  }
}

impl ChessContext {
  /// Creates the services used by one mounted rules session.
  pub fn new(
    execution: ExecutionMode<ChessGame, ChessPolicy>,
    opponent: Opponent,
    rng: Rng,
  ) -> Self {
    Self {
      execution,
      opponent,
      rng,
    }
  }

  /// Derives the presentation checkpoint before committing a logical move.
  ///
  /// A state animation should contain all transient facts the component tree
  /// needs. Reading them later from mutated state would lose captured identities
  /// and blur which move produced a sound or semantic outcome.
  fn movement(&mut self, state: &ChessState, movement: Move) -> ChessAnimation {
    let moving = state
      .position
      .piece(movement.from)
      .expect("legal mover has a presentation identity");
    let description = state.position.movement(movement);
    let mut board_after = state.position.board.clone();
    board_after.play_unchecked(movement);
    let sound = match &description {
      Movement::Castle { .. } => crate::audio::CASTLE_SOUND,
      Movement::Promotion { .. } => crate::audio::PROMOTION_SOUND,
      Movement::Capture { .. } => {
        crate::audio::CAPTURE_SOUNDS[self.rng.usize(..crate::audio::CAPTURE_SOUNDS.len())].clone()
      }
      Movement::Move { .. } | Movement::Knight { .. } => {
        crate::audio::DROP_SOUNDS[self.rng.usize(..crate::audio::DROP_SOUNDS.len())].clone()
      }
    };
    let final_sound = match board_after.status() {
      GameStatus::Won if board_after.side_to_move() == Color::Black => {
        Some(crate::audio::PLAYER_WIN_SOUND)
      }
      GameStatus::Won => Some(crate::audio::PLAYER_LOSS_SOUND),
      GameStatus::Drawn => Some(crate::audio::DRAW_SOUND),
      GameStatus::Ongoing if !board_after.checkers().is_empty() => Some(crate::audio::CHECK_SOUND),
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

  /// Publishes an animation checkpoint, then commits the matching logical state.
  ///
  /// `present` is the Reactant boundary that lets the display animate an accepted
  /// transition while the rules worker remains the sole owner of mutation.
  fn apply_move(&mut self, state: &mut ChessState, movement: Move) {
    let animation = self.movement(state, movement);
    let result = animation.result();
    self.execution.present(state, || animation);
    state.position.apply(movement);
    state.result = Some(result);
  }
}

impl ChoicePolicy<ChessGame> for ChessPolicy {
  /// Assigns promotion decisions to the human-facing Reactant prompt UI.
  fn owner(&self, _: &ChessState, prompt: &ChessPrompt<'_>) -> ChoiceOwner {
    match prompt {
      ChessPrompt::Promotion(_) => ChoiceOwner::Human,
    }
  }

  /// Rejects automatic choice because every current prompt is human-owned.
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

  /// Clones only deterministic logical state for speculative rules execution.
  fn logical_clone(state: &ChessState) -> ChessState {
    state.clone()
  }

  /// Validates actions again at the worker boundary before execution.
  ///
  /// UI disabling improves affordance, but rules legality is still authoritative;
  /// callers may be stale or actions may arrive from other input sources.
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

  /// Executes one complete action, pausing on a typed prompt when necessary.
  ///
  /// A player move and the computer reply are separate actions so the display can
  /// commit and animate each checkpoint before the next bounded action begins.
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
        let reply = context.opponent.choose(&state.position.board);
        context.apply_move(state, reply);
      }
    }
  }
}
