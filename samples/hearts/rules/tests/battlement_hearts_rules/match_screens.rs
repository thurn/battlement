use std::time::{Duration, Instant};

use battlement::{NavigationDirection, Prop, SemanticRole, UiVisualElementProperties, object_id};
use battlement_hearts_rules::{
  self as hearts, HeartsReducer,
  domain::{HeartsState, Phase, Seat},
};
use reactant_rules::ReducerGame;
use reactant_testing::Display;

use crate::shell;

type Game = ReducerGame<HeartsReducer>;

#[test]
fn settings_and_confirmation_keep_the_match_and_trap_focus() {
  let mut display = Display::mount(
    || hearts::application_from_state(HeartsState::new(43)),
    shell::catalog(),
  );
  self::pump(&mut display);
  let before = display.game_state::<Game>().unwrap();
  self::press(&mut display, "Seven of Clubs");
  self::press(&mut display, "Menu");
  self::press(&mut display, "Settings");
  self::press(&mut display, "Text size: standard");
  self::press(&mut display, "Reduced motion: automatic");
  self::press(&mut display, "Animation pace: relaxed");
  assert_eq!(display.game_state::<Game>().unwrap(), before);
  assert!(
    display
      .accessibility()
      .nodes
      .iter()
      .all(|node| node.role != SemanticRole::Button
        || !node.label.as_deref().unwrap().ends_with("of Clubs"))
  );
  let close = display.semantic_node("Back to game").object_id;
  for _ in 0..12 {
    display.navigate(NavigationDirection::Next);
    let focused = display.focused().unwrap();
    assert!(
      display
        .accessibility()
        .nodes
        .iter()
        .any(|node| node.object_id == focused)
    );
  }
  self::press(&mut display, "Back to game");
  assert!(!display.contains_ui(close));
  assert_eq!(display.game_state::<Game>().unwrap(), before);
  assert!(
    display
      .accessibility()
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Choose three to pass · 1 / 3"))
  );
  self::press(&mut display, "New game");
  assert_eq!(
    display.focused(),
    Some(display.semantic_node("Cancel").object_id)
  );
  display.activate_focused();
  self::pump(&mut display);
  assert_eq!(display.game_state::<Game>().unwrap(), before);
}

#[test]
fn final_hand_through_visible_controls_reaches_results_without_next_hand() {
  let initial = HeartsState::new(43);
  let final_hand = HeartsState::from_deal(
    std::array::from_fn(|seat| initial.hand(Seat::ALL[seat]).to_vec()),
    0,
    [99, 99, 99, 93],
    initial.random().clone(),
  );
  let mut display = Display::mount(
    move || hearts::application_from_state(final_hand.clone()),
    shell::catalog(),
  );
  for _ in 0..200 {
    self::pump(&mut display);
    let state = display.game_state::<Game>().unwrap();
    match state.phase() {
      Phase::Passing => {
        for card in state.hand(Seat::South).iter().take(3) {
          self::press(&mut display, &format!("{:?} of {:?}", card.rank, card.suit));
        }
        self::press(&mut display, "Pass three cards");
      }
      Phase::Playing { turn: Seat::South } => {
        let card = state.observe(Seat::South).legal_plays[0];
        self::press(&mut display, &format!("{:?} of {:?}", card.rank, card.suit));
        self::press(&mut display, "Play selected card");
      }
      Phase::Playing { .. } => continue,
      Phase::HandOver => panic!("the seeded final hand must cross the match threshold"),
      Phase::MatchOver { winners } => {
        assert!(winners.contains(&true));
        display.expect_button("New game");
        assert!(
          !display
            .accessibility()
            .nodes
            .iter()
            .any(|node| node.label.as_deref() == Some("Next hand"))
        );
        let before = display.game_state::<Game>().unwrap();
        self::press(&mut display, "New game");
        self::press(&mut display, "Cancel");
        assert_eq!(display.game_state::<Game>().unwrap(), before);
        return;
      }
    }
  }
  panic!("match did not finish within bounded UI actions");
}

fn pump(display: &mut Display) {
  let deadline = Instant::now() + Duration::from_secs(10);
  loop {
    display.flush();
    display.settle();
    let state = display.game_state::<Game>().unwrap();
    let nodes = &display.accessibility().nodes;
    let dialog = nodes.iter().find(|node| node.role == SemanticRole::Dialog);
    let ready = match dialog.and_then(|node| node.label.as_deref()) {
      Some("Results") => {
        matches!(state.phase(), Phase::HandOver | Phase::MatchOver { .. })
          && nodes
            .iter()
            .any(|node| node.role == SemanticRole::Button && !node.state.disabled)
      }
      Some(_) => true,
      None => {
        display
          .ui_element(object_id!("6644ed66-12dc-4590-9af8-19d174a47014"))
          .element()
          .visual_element()
          .enabled
          == Prop::Set(true)
      }
    };
    if ready && display.game_status::<Game>() == Some(reactant::GameStatus::Ready) {
      return;
    }
    assert!(
      Instant::now() < deadline,
      "UI failed to become ready in {:?}",
      state.phase()
    );
    std::thread::sleep(Duration::from_millis(1));
  }
}

fn press(display: &mut Display, label: &str) {
  assert!(
    !display.semantic_node(label).state.disabled,
    "{label} must be ready"
  );
  display.activate_accessible(label);
  self::pump(display);
}
