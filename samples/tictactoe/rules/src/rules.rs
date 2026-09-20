use reactant::rules::{ChoiceOwner, ChoicePolicy, DisplayConnection, ExecutionMode, Game};

/// Canonical seed used by the sample and its Ditto scenarios.
pub const DITTO_SEED: u64 = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mark {
  X,
  O,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Outcome {
  InProgress,
  XWins,
  OWins,
  Draw,
}

#[derive(Clone)]
pub struct State {
  pub(crate) round: u32,
  pub(crate) board: [Option<Mark>; 9],
  pub(crate) outcome: Outcome,
  pub(crate) visual_state: VisualState,
  rng: fastrand::Rng,
}

#[derive(Clone, Copy)]
pub enum Action {
  Cell(usize),
  Reset,
}

#[derive(Clone, Copy)]
pub enum Animation {
  HumanMove,
  PlayerFinished,
  AiResponse,
}

pub struct TicTacToe;
pub(crate) struct Policy;
pub struct Context(ExecutionMode<TicTacToe, Policy>);

/// Finite user-visible states recognized by the Tic-Tac-Toe engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisualState {
  /// A new empty board.
  EmptyBoard,
  /// The immediate player mark while the AI response is pending.
  HumanMove,
  /// A completed AI response with player input restored.
  AiResponse,
  /// A player-winning board.
  PlayerWin,
  /// A computer-winning board.
  ComputerWin,
  /// A completed drawn board.
  Draw,
  /// The empty board after advancing to the next round.
  RestoredBoard,
}

impl VisualState {
  /// Every visual state in registry order.
  pub const ALL: [Self; 7] = [
    Self::EmptyBoard,
    Self::HumanMove,
    Self::AiResponse,
    Self::PlayerWin,
    Self::ComputerWin,
    Self::Draw,
    Self::RestoredBoard,
  ];

  /// Returns the canonical Ditto registry key.
  pub const fn registry_key(self) -> &'static str {
    match self {
      Self::EmptyBoard => "board.empty",
      Self::HumanMove => "turn.human-move",
      Self::AiResponse => "turn.ai-response",
      Self::PlayerWin => "terminal.player-win",
      Self::ComputerWin => "terminal.computer-win",
      Self::Draw => "terminal.draw",
      Self::RestoredBoard => "board.restored",
    }
  }

  pub(crate) fn semantic_fixture(name: &str) -> Option<Self> {
    match name {
      "human move" => Some(Self::HumanMove),
      _ => None,
    }
  }
}

impl State {
  pub(crate) fn new(seed: u64, fixture: Option<VisualState>) -> Self {
    let visual_state = fixture.unwrap_or(VisualState::EmptyBoard);
    let mut board = [None; 9];
    if visual_state == VisualState::HumanMove {
      board[2] = Some(Mark::X);
    }
    Self {
      round: 1,
      board,
      outcome: Outcome::InProgress,
      visual_state,
      rng: fastrand::Rng::with_seed(seed),
    }
  }

  pub(crate) fn accepts_cell(&self, index: usize) -> bool {
    if index >= self.board.len() || self.visual_state == VisualState::HumanMove {
      return false;
    }
    self.outcome == Outcome::InProgress && self.board[index].is_none()
  }

  pub(crate) fn input_cells(&self) -> impl Iterator<Item = usize> + '_ {
    (0..self.board.len()).filter(|index| self.accepts_cell(*index))
  }

  pub(crate) fn is_terminal(&self) -> bool {
    self.outcome != Outcome::InProgress
  }

  pub(crate) fn status_text(&self) -> &'static str {
    match self.visual_state {
      VisualState::EmptyBoard | VisualState::AiResponse | VisualState::RestoredBoard => {
        "Your turn — click an empty square"
      }
      VisualState::HumanMove => "Computer thinking…",
      VisualState::PlayerWin => "You win! Click the board to play again.",
      VisualState::ComputerWin => "Computer wins. Click the board to play again.",
      VisualState::Draw => "Draw! Click the board to play again.",
    }
  }

  fn reset(&mut self) {
    self.round += 1;
    self.board = [None; 9];
    self.outcome = Outcome::InProgress;
    self.visual_state = VisualState::RestoredBoard;
  }
}

impl Context {
  pub(crate) fn new(connection: DisplayConnection<TicTacToe>) -> Self {
    Self(ExecutionMode::Interactive {
      connection,
      policy: Policy,
    })
  }
}

impl ChoicePolicy<TicTacToe> for Policy {
  fn owner(&self, _: &State, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }

  fn choose(&mut self, _: &State, _: &()) -> usize {
    unreachable!("Tic-Tac-Toe has no prompts")
  }
}

impl Game for TicTacToe {
  type State = State;
  type Action = Action;
  type StateAnimation = Animation;
  type Prompt<'a> = ();
  type Context = Context;

  fn logical_clone(state: &State) -> State {
    state.clone()
  }

  fn is_legal_action(state: &State, action: &Action) -> bool {
    match action {
      Action::Cell(index) => state.accepts_cell(*index),
      Action::Reset => state.is_terminal(),
    }
  }

  fn execute(context: &mut Context, state: &mut State, action: Action) {
    let Action::Cell(index) = action else {
      state.reset();
      return;
    };

    state.board[index] = Some(Mark::X);
    state.outcome = outcome(&state.board);
    state.visual_state = after_player(state.outcome);
    if state.outcome != Outcome::InProgress {
      context.0.present(state, || Animation::PlayerFinished);
      return;
    }

    context.0.present(state, || Animation::HumanMove);
    let empty = empty_cells(&state.board);
    let ai = empty[state.rng.usize(..empty.len())];
    state.board[ai] = Some(Mark::O);
    state.outcome = outcome(&state.board);
    state.visual_state = after_ai(state.outcome);
    context.0.present(state, || Animation::AiResponse);
  }
}

fn after_player(outcome: Outcome) -> VisualState {
  match outcome {
    Outcome::InProgress => VisualState::HumanMove,
    Outcome::XWins => VisualState::PlayerWin,
    Outcome::OWins => VisualState::ComputerWin,
    Outcome::Draw => VisualState::Draw,
  }
}

fn after_ai(outcome: Outcome) -> VisualState {
  match outcome {
    Outcome::InProgress => VisualState::AiResponse,
    Outcome::XWins => VisualState::PlayerWin,
    Outcome::OWins => VisualState::ComputerWin,
    Outcome::Draw => VisualState::Draw,
  }
}

fn outcome(board: &[Option<Mark>; 9]) -> Outcome {
  const LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
  ];
  for [first, second, third] in LINES {
    if board[first].is_some() && board[first] == board[second] && board[second] == board[third] {
      return if board[first] == Some(Mark::X) {
        Outcome::XWins
      } else {
        Outcome::OWins
      };
    }
  }
  if board.iter().all(Option::is_some) {
    Outcome::Draw
  } else {
    Outcome::InProgress
  }
}

fn empty_cells(board: &[Option<Mark>; 9]) -> Vec<usize> {
  board
    .iter()
    .enumerate()
    .filter_map(|(index, mark)| mark.is_none().then_some(index))
    .collect()
}
