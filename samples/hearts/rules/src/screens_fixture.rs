use reactant::Application;

use crate::{
  app,
  domain::{
    HeartsState, IgnorePresentation, Intention, Phase, RandomStreams, Seat, cards, transition,
  },
  layout_fixture,
};

pub(crate) fn application(name: &str) -> Application {
  let state = match name {
    "teaching" | "screens" => HeartsState::new(43),
    "hand-results" => self::finish(layout_fixture::playing_state()),
    "moon-results" => self::finish(self::suit_deal([0; 4])),
    "match-results" => self::finish(self::suit_deal([60, 60, 80, 80])),
    "tied-results" => self::finish(self::suit_deal([86, 60, 80, 80])),
    "hold" => self::suit_deal([0; 4]),
    _ => panic!("unknown screen fixture"),
  };
  app::screen_application(state, name == "teaching")
}

fn suit_deal(totals: [u16; 4]) -> HeartsState {
  let deck = cards::deck();
  HeartsState::from_deal(
    std::array::from_fn(|seat| deck[seat * 13..(seat + 1) * 13].to_vec()),
    3,
    totals,
    RandomStreams::from_seed(43),
  )
}

fn finish(mut state: HeartsState) -> HeartsState {
  while let Phase::Playing { turn } = state.phase() {
    let intention = Intention::PlayCard {
      seat: turn,
      card: state.observe(turn).legal_plays[0],
    };
    transition::apply(&mut state, intention, &mut IgnorePresentation).expect("fixture play");
  }
  assert!(matches!(
    state.phase(),
    Phase::HandOver | Phase::MatchOver { .. }
  ));
  assert_eq!(state.hand(Seat::South).len(), 0);
  state
}
