//! Menu scenarios exercise the same public host interface as gameplay.
mod support;

use battlement::{CheckedState, Length, Prop, SemanticRole, StyleValue};
use cozy_chess::{Color, Piece, Square};
use support::game::ChessTest;

#[test]
fn menu_opens_settings_and_starts_the_3d_game() {
  let mut chess = ChessTest::title();
  assert_eq!(
    chess.display.semantic_node("Chess Chess Revolution").role,
    SemanticRole::Heading
  );
  chess.display.expect_button("PLAY");
  chess.display.activate_accessible("SETTINGS");
  assert_eq!(
    chess.display.semantic_node("Settings").role,
    SemanticRole::Heading
  );
  chess.display.activate_accessible("RETURN");
  chess.display.expect_button("PLAY");

  chess.start();
  chess.expect_piece(Square::E2, Color::White, Piece::Pawn);
  chess.display.expect_button("Main menu");
}

#[test]
fn gameplay_can_return_to_menu_and_resume_the_retained_game() {
  let mut chess = ChessTest::title();
  chess.start();
  chess.play(Square::E2, Square::E4);
  chess.show_menu();
  chess.display.expect_button("PLAY");
  chess.start();
  chess.expect_piece(Square::E4, Color::White, Piece::Pawn);
  chess.expect_empty(Square::E2);
}

#[test]
fn settings_keep_player_choices_across_menu_navigation() {
  let mut chess = ChessTest::title();
  chess.display.activate_accessible("SETTINGS");
  chess.display.activate_accessible("Text Size 100%");
  chess.display.activate_accessible("200%");
  chess.display.activate_accessible("Reduce Motion");
  chess.display.activate_accessible("RETURN");
  chess.display.activate_accessible("SETTINGS");

  chess.display.expect_button("Text Size 200%");
  assert_eq!(
    chess.display.semantic_node("Reduce Motion").state.checked,
    Some(CheckedState::True)
  );
  chess.display.activate_accessible("Sound");
  assert_eq!(
    chess.display.semantic_node("Master Volume").role,
    SemanticRole::Slider
  );
  chess.display.activate_accessible("Input");
  assert!(chess.display.accessibility().nodes.iter().any(|node| {
    node.role == SemanticRole::Table && node.label.as_deref() == Some("Input bindings")
  }));
  chess.display.activate_accessible("RETURN");
  chess.start();
  chess.show_menu();
  chess.display.activate_accessible("SETTINGS");
  chess.display.expect_button("Text Size 200%");
  assert_eq!(
    chess.display.semantic_node("Reduce Motion").state.checked,
    Some(CheckedState::True)
  );
}

#[test]
fn every_text_size_keeps_localized_settings_and_binding_dialog_operable() {
  for french in [false, true] {
    let mut chess = ChessTest::title();
    chess.display.activate_accessible("SETTINGS");
    if french {
      chess.display.activate_accessible("Language English");
      chess.display.activate_accessible("Français");
    }
    let size_label = if french {
      "Taille du texte"
    } else {
      "Text Size"
    };
    let input = if french { "Entrées" } else { "Input" };
    let sound = if french { "Son" } else { "Sound" };
    let return_label = if french { "RETOUR" } else { "RETURN" };
    let settings = if french { "PARAMÈTRES" } else { "SETTINGS" };
    let binding = if french {
      "Modifier la touche pour « \u{2068}Gauche\u{2069} »"
    } else {
      "Change \u{2068}Left\u{2069} keyboard binding"
    };
    let mut previous = "100%";
    for size in ["100%", "150%", "200%", "100%"] {
      chess
        .display
        .activate_accessible(&format!("{size_label} {previous}"));
      chess.display.activate_accessible(size);
      assert_eq!(
        chess.display.focused(),
        Some(
          chess
            .display
            .semantic_node(&format!("{size_label} {size}"))
            .object_id
        )
      );
      let factor = size.trim_end_matches('%').parse::<f32>().unwrap() / 100.0;
      let selected = chess
        .display
        .semantic_node(&format!("{size_label} {size}"))
        .object_id;
      assert_eq!(
        chess.display.ui_element(selected).style().font_size,
        Prop::Set(StyleValue::Value(Length::Px(60.0 * factor)))
      );
      let value = chess.display.find_ui(selected, "select-value");
      assert_eq!(
        chess.display.ui_element(value).style().font_size,
        Prop::Unset,
        "the value inherits the scaled size exactly once"
      );
      chess.display.activate_accessible(input);
      chess.display.activate_accessible(binding);
      chess
        .display
        .expect_button(if french { "Réinitialiser" } else { "Reset" });
      chess
        .display
        .activate_accessible(if french { "Annuler" } else { "Cancel" });
      chess.display.expect_button(binding);
      chess.display.activate_accessible(sound);
      assert_eq!(
        chess
          .display
          .semantic_node(if french {
            "Volume général"
          } else {
            "Master Volume"
          })
          .role,
        SemanticRole::Slider
      );
      chess.display.activate_accessible(return_label);
      chess.display.activate_accessible(settings);
      chess.display.expect_button(&format!("{size_label} {size}"));
      chess.display.activate_accessible(return_label);
      chess
        .display
        .activate_accessible(if french { "JOUER" } else { "PLAY" });
      chess.pause();
      let menu = if french {
        "Menu principal"
      } else {
        "Main menu"
      };
      let menu_button = chess.display.semantic_node(menu).object_id;
      assert_eq!(
        chess.display.ui_element(menu_button).style().font_size,
        Prop::Set(StyleValue::Value(Length::Px(14.0 * factor)))
      );
      chess.pause();
      chess.display.activate_accessible(menu);
      chess.display.activate_accessible(settings);
      previous = size;
    }
  }
}
