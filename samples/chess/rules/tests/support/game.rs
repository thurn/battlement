//! User-intent helpers isolate scenarios from the current gesture and layout.
use crate::support::{catalog, host};
use battlement::{Connect, PhysicalKey, ScreenSize};
use chess_rules::{
  self, ChessGame, ChessPrompt, EngineDependencies, Opponent, PersistenceBackend, assets,
};
use cozy_chess::{Board, Color, Move, Piece, Square};
use reactant::rules::RulesWorker;
use reactant_testing::{Display, InlineActionResult};
use std::{
  collections::VecDeque,
  rc::Rc,
  sync::{Arc, Mutex},
  time::Duration,
};

pub struct ChessTest {
  pub display: Display,
  opponent: Opponent,
  choices: Arc<Mutex<VecDeque<(Square, Square, Piece)>>>,
}

impl ChessTest {
  pub fn from_position(board: Board) -> Self {
    Self::assemble(Some(board), None, &[])
  }
  pub fn title() -> Self {
    Self::assemble(None, None, &[])
  }
  pub fn persisted(storage: Rc<dyn PersistenceBackend>) -> Self {
    Self::assemble(None, Some(storage), &[])
  }
  pub fn assemble(
    position: Option<Board>,
    persistence: Option<Rc<dyn PersistenceBackend>>,
    modules: &[&str],
  ) -> Self {
    let opponent = Opponent::scripted();
    let choices = Arc::new(Mutex::new(VecDeque::<(Square, Square, Piece)>::new()));
    let answers = choices.clone();
    let runner = RulesWorker::inline().answer_with::<ChessGame>(move |prompt| {
      let ChessPrompt::Promotion(prompt) = prompt;
      let (from, to, piece) = answers
        .lock()
        .unwrap()
        .pop_front()
        .expect("unscripted promotion");
      assert_eq!(
        (prompt.from, prompt.to),
        (from, to),
        "script must answer the requested move"
      );
      prompt
        .choices
        .iter()
        .position(|p| *p == piece)
        .expect("invalid promotion choice")
    });
    let computer = opponent.clone();
    let mut connect =
      Connect::new("test", "test", ScreenSize::new(1920, 1080)).persistent_data_path("memory");
    connect.modules = modules.iter().map(|m| (*m).to_owned()).collect();
    let display = Display::connect_inline(
      move |clock| {
        chess_rules::create_engine(EngineDependencies {
          position,
          persistence,
          rules_worker: runner,
          now: Rc::new(move || clock.now()),
          rng_seed: Some(43),
          opponent: computer,
        })
      },
      catalog::assets(),
      connect,
    );
    Self {
      display,
      opponent,
      choices,
    }
  }
  pub fn play(&mut self, from: Square, to: Square) {
    assert_eq!(
      self
        .display
        .action_inline(|display| host::move_piece(display, from, to)),
      InlineActionResult::Completed,
      "visible input did not admit a chess action"
    );
    host::expect_empty(&self.display, from);
    assert_eq!(
      host::at(&self.display, to).len(),
      1,
      "move was not visibly completed"
    );
    assert!(self.choices.lock().unwrap().is_empty(), "unused choice");
  }
  pub fn promote(&mut self, from: Square, to: Square, piece: Piece) {
    self.choices.lock().unwrap().push_back((from, to, piece));
    self.play(from, to);
  }
  pub fn reply(&mut self, from: Square, to: Square) {
    self.opponent.reply_with(Move {
      from,
      to,
      promotion: None,
    });
    self.display.refresh_inline();
    self.opponent.assert_reply_consumed();
    host::expect_empty(&self.display, from);
  }
  pub fn expect_piece(&self, square: Square, color: Color, piece: Piece) {
    host::expect_piece(&self.display, square, color, piece);
  }
  pub fn expect_empty(&self, square: Square) {
    host::expect_empty(&self.display, square);
  }
  pub fn start(&mut self) {
    host::image_click(&mut self.display, assets::PLAY_BUTTON);
    self.display.finish_inline();
  }
  pub fn pause(&mut self) {
    host::key(&mut self.display, PhysicalKey::Escape);
    self.display.finish_inline();
  }
  pub fn new_game(&mut self) {
    self.pause();
    for _ in 0..2 {
      host::image_click(&mut self.display, assets::REFRESH_BUTTON);
      self.display.finish_inline();
    }
  }
  pub fn restart(&mut self) {
    for key in [
      PhysicalKey::ControlLeft,
      PhysicalKey::ShiftLeft,
      PhysicalKey::KeyR,
    ] {
      self.display.key_down(key);
    }
    for key in [
      PhysicalKey::KeyR,
      PhysicalKey::ShiftLeft,
      PhysicalKey::ControlLeft,
    ] {
      self.display.key_up(key);
    }
    self.choices.lock().unwrap().clear();
    self.display.finish_inline();
  }
  pub fn advance(&mut self, time: Duration) {
    self.display.advance_inline(time);
  }
}
