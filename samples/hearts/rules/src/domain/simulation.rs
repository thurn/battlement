use std::array;

use crate::domain::{AiObservation, Hands, HeartsState, Phase, RandomStreams, cards, scoring};

/// Reconstructs one sampled world using public completed tricks, never private state.
pub fn from_observation(observation: &AiObservation, hands: Hands) -> HeartsState {
  let table = &observation.table;
  assert!(matches!(
    table.phase,
    Phase::Passing | Phase::Playing { .. }
  ));
  assert_eq!(hands.each_ref().map(Vec::len), table.hand_counts);
  let mut own = observation.hand.clone();
  own.sort_unstable();
  assert_eq!(hands[observation.seat.index()], own);
  let mut all: Vec<_> = hands.iter().flatten().copied().collect();
  all.extend(table.history.iter().map(|played| played.card));
  all.sort_unstable();
  assert_eq!(all, cards::deck());
  assert!(table.history.ends_with(&table.trick));
  let completed = table.history.len() - table.trick.len();
  assert_eq!(completed % 4, 0);
  let mut captured: Hands = array::from_fn(|_| Vec::new());
  for trick in table.history[..completed].as_chunks::<4>().0 {
    captured[scoring::trick_winner(trick).index()].extend(trick.iter().map(|played| played.card));
  }
  HeartsState {
    hands,
    passes: [None; 4],
    trick: table.trick.clone(),
    captured,
    history: table.history.clone(),
    voids: table.voids,
    leader: table.leader.unwrap_or(observation.seat),
    hearts_broken: table.hearts_broken,
    hand_index: table.hand_index,
    totals: table.totals,
    phase: table.phase,
    result: table.result,
    random: RandomStreams::from_seed(0),
  }
}
