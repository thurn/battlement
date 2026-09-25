//! Live locale replacement through the application handle preserves ongoing chess work.

use std::{cell::RefCell, rc::Rc};

use battlement::{
  Connect, KeyEvent, ObjectId, PhysicalKey, ScreenSize, UiDocument, UiEvent, UiEventBody,
};
use cozy_chess::{Board, BoardBuilder, Color, Piece, Square};
use reactant::{
  ApplicationEngine,
  app_context::{self, AppHandle},
  asset_generator, hooks,
  prelude::*,
  rules::RulesWorker,
};
use reactant_testing::{Display, assets as testing_assets};

use chess_rules::{self, ChessGame, EngineDependencies, Opponent, assets, settings::Language};
use trox::{Bundle, Localizer};

#[derive(Clone)]
struct ObserveApplication(Rc<RefCell<Option<AppHandle>>>);

impl Component for ObserveApplication {
  fn render(&self) -> impl Render {
    let app = app_context::use_app();
    let handle = self.0.clone();
    hooks::use_effect(
      move || {
        handle.replace(Some(app));
      },
      (),
    );
  }
}

fn display(board: Option<Board>) -> (Display, Rc<RefCell<Option<AppHandle>>>) {
  let handle = Rc::new(RefCell::new(None));
  let observer = handle.clone();
  let mut catalog = testing_assets::catalog(assets::ASSET_CATALOG);
  catalog.add_textures(asset_generator::registrations().map(|asset| asset.address));
  let display = Display::connect_application::<ChessGame>(
    move |clock| {
      ApplicationEngine::with_clock(
        move || {
          chess_rules::create_application(&EngineDependencies {
            position: board.clone(),
            persistence: None,
            rules_worker: RulesWorker::default(),
            now: Rc::new(std::time::Instant::now),
            opponent: Opponent::scripted(),
            rng_seed: Some(43),
          })
          .additional_document(
            UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4()),
            ObserveApplication(observer.clone()),
          )
        },
        move || clock.now(),
      )
    },
    catalog,
    Connect::new("test", "test", ScreenSize::new(1920, 1080)),
  );
  (display, handle)
}

fn change_language(
  display: &mut Display,
  handle: &Rc<RefCell<Option<AppHandle>>>,
  language: Language,
) {
  handle
    .borrow()
    .as_ref()
    .unwrap()
    .set_localizer(self::localizer(language));
  display.settle();
}

#[test]
fn language_replacement_keeps_pending_promotion_and_piece_identity() {
  let mut board = BoardBuilder::empty();
  for (square, piece, color) in [
    (Square::E1, Piece::King, Color::White),
    (Square::E8, Piece::King, Color::Black),
    (Square::A7, Piece::Pawn, Color::White),
    (Square::B8, Piece::Rook, Color::Black),
  ] {
    *board.square_mut(square) = Some((piece, color));
  }
  let (mut display, handle) = self::display(Some(board.build().unwrap()));
  let pawn = display
    .semantic_node("\u{2068}White Pawn\u{2069} at \u{2068}a7\u{2069}")
    .object_id;
  display.activate_accessible("\u{2068}White Pawn\u{2069} at \u{2068}a7\u{2069}");
  display.activate_accessible("Move to \u{2068}b8\u{2069}");
  let choice = display.semantic_node("Knight").object_id;
  self::change_language(&mut display, &handle, Language::French);
  assert_eq!(display.semantic_node("Cavalier").object_id, choice);
  assert_eq!(
    display
      .semantic_node("\u{2068}Pion blanc\u{2069} en \u{2068}a7\u{2069}")
      .object_id,
    pawn
  );
  display.click_button("Cavalier");
  assert_eq!(
    display
      .game_state::<ChessGame>()
      .unwrap()
      .board()
      .piece_on(Square::B8),
    Some(Piece::Knight)
  );
  self::change_language(&mut display, &handle, Language::English);
  display.expect_button("\u{2068}White Knight\u{2069} at \u{2068}b8\u{2069}");
}

#[test]
fn language_replacement_keeps_binding_dialog_focus_and_conflict() {
  let (mut display, handle) = self::display(None);
  display.activate_accessible("SETTINGS");
  display.activate_accessible("Input");
  display.activate_accessible("Change \u{2068}Left\u{2069} keyboard binding");
  let focus = display
    .focused()
    .expect("capture focuses its waiting input");
  display.deliver_ui_event(UiEvent {
    target_id: display
      .semantic_node("Waiting for keyboard input")
      .object_id,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::KeyDown(KeyEvent {
      physical_key: Some(PhysicalKey::ArrowRight),
      ..KeyEvent::default()
    }),
  });
  display.settle();
  display.semantic_node("Already used by \u{2068}Right\u{2069}");
  self::change_language(&mut display, &handle, Language::French);
  assert_eq!(display.focused(), Some(focus));
  display.semantic_node("Déjà utilisée pour « \u{2068}Droite\u{2069} »");
  display.expect_button("Réinitialiser");
  display.semantic_node("Appuyez sur une touche pour « \u{2068}Gauche\u{2069} »");
  self::change_language(&mut display, &handle, Language::English);
  assert_eq!(display.focused(), Some(focus));
  display.semantic_node("Already used by \u{2068}Right\u{2069}");
  display.activate_accessible("Cancel");
  display.expect_button("Change \u{2068}Left\u{2069} keyboard binding");
}

fn localizer(language: Language) -> Localizer {
  let source =
    Bundle::from_canonical_json(include_str!("../localization/bundles/en-US.trox.json")).unwrap();
  let target = match language {
    Language::English => source.clone(),
    Language::French => {
      Bundle::from_canonical_json(include_str!("../localization/bundles/fr.trox.json")).unwrap()
    }
  };
  Localizer::new(target, source).unwrap()
}
