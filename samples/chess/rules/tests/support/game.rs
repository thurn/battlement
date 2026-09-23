//! User-intent helpers isolate scenarios from the current gesture and layout.
use crate::support::board;
use battlement::{Connect, PhysicalKey, PrefabAddress, ScreenSize, Vector3};
use battlement_fake::assets::FakeAssetCatalog;
use chess_rules::{
  self, ChessGame, ChessPrompt, EngineDependencies, Opponent, PersistenceBackend, assets,
};
use cozy_chess::{Board, Color, Move, Piece, Square};
use reactant::{asset_generator, rules::RulesWorker};
use reactant_testing::{ActionResult, Display, assets as testing_assets};
use std::{
  collections::VecDeque,
  rc::Rc,
  sync::{Arc, Mutex, OnceLock},
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
    Self::configured(position, persistence, modules, Opponent::scripted(), false)
  }

  pub fn live_dialog(board: Board) -> Self {
    Self::configured(Some(board), None, &[], Opponent::scripted(), true)
  }

  pub fn with_computer(board: Board) -> Self {
    Self::configured(
      Some(board),
      None,
      &[],
      Opponent::search(Duration::ZERO),
      true,
    )
  }

  fn configured(
    position: Option<Board>,
    persistence: Option<Rc<dyn PersistenceBackend>>,
    modules: &[&str],
    opponent: Opponent,
    live: bool,
  ) -> Self {
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
    let runner = if live { RulesWorker::default() } else { runner };
    let computer = opponent.clone();
    let mut connect =
      Connect::new("test", "test", ScreenSize::new(1920, 1080)).persistent_data_path("memory");
    connect.modules = modules.iter().map(|m| (*m).to_owned()).collect();
    let display = Display::connect_application::<ChessGame>(
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
      Self::assets(),
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
      self.attempt_move(from, to),
      ActionResult::Completed,
      "visible input did not admit a chess action"
    );
    board::expect_empty(&self.display, from);
    assert_eq!(
      board::at(&self.display, to).len(),
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
    self.display.refresh();
    self.opponent.assert_reply_consumed();
    board::expect_empty(&self.display, from);
  }

  pub fn expect_piece(&self, square: Square, color: Color, piece: Piece) {
    board::expect_piece(&self.display, square, color, piece);
  }

  pub fn expect_empty(&self, square: Square) {
    board::expect_empty(&self.display, square);
  }

  pub fn expect_board(&self, expected: &Board) {
    for square in Square::ALL {
      match (expected.color_on(square), expected.piece_on(square)) {
        (Some(color), Some(piece)) => self.expect_piece(square, color, piece),
        _ => self.expect_empty(square),
      }
    }
    assert_eq!(self.pieces().len(), expected.occupied().len() as usize);
  }

  pub fn pieces(&self) -> Vec<(Vector3, PrefabAddress)> {
    board::pieces(&self.display)
  }

  pub fn attempt_move(&mut self, from: Square, to: Square) -> ActionResult {
    assert_eq!(
      board::at(&self.display, from).len(),
      1,
      "source must have one piece"
    );
    self
      .display
      .action(|display| display.drag_world(board::grab_point(from), board::center(to)))
  }

  pub fn drop_off_board(&mut self, from: Square) {
    self
      .display
      .drag_world(board::grab_point(from), Vector3::new(8.0, 0.0, 0.0));
  }

  pub fn cancel_move(&mut self, from: Square, to: Square) {
    self
      .display
      .begin_drag_world(board::grab_point(from), board::center(to));
    self.display.cancel_drag();
  }

  pub fn start(&mut self) {
    self.display.activate_accessible("PLAY");
  }

  pub fn show_menu(&mut self) {
    self.display.click_button("Main menu");
    self.display.refresh();
  }

  pub fn pause(&mut self) {
    self.display.send_key(PhysicalKey::Escape);
  }

  pub fn new_game(&mut self) {
    self.pause();
    self.display.click_image(assets::REFRESH_BUTTON);
    self.display.click_image(assets::REFRESH_BUTTON);
  }

  pub fn restart(&mut self) {
    self.display.send_shortcut(&[
      PhysicalKey::ControlLeft,
      PhysicalKey::ShiftLeft,
      PhysicalKey::KeyR,
    ]);
    self.choices.lock().unwrap().clear();
  }

  pub fn advance(&mut self, time: Duration) {
    self.display.advance(time);
  }

  pub fn assets() -> Arc<FakeAssetCatalog> {
    static ASSETS: OnceLock<Arc<FakeAssetCatalog>> = OnceLock::new();
    ASSETS
      .get_or_init(|| {
        let mut catalog = testing_assets::catalog(assets::ASSET_CATALOG);
        catalog.add_textures(asset_generator::registrations().map(|asset| asset.address));
        Arc::new(catalog)
      })
      .clone()
  }
}
