use std::{collections::BTreeSet, sync::mpsc, time::Duration};

use battlement_hearts_rules::ai::sampling::{self, SAMPLE_COUNT, SampleBatch};
use battlement_hearts_rules::domain::{
  AiObservation, CardId, HeartsState, IgnorePresentation, Intention, Phase, RandomStream,
  RandomStreams, Rank, Seat, Suit, cards, transition,
};
use reactant_rules::{ComputationLane, ComputationStatus};

const TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn common_samples_obey_known_cards_voids_counts_and_a_finite_assignment_bound() {
  let mut state = HeartsState::new(12);
  for seat in Seat::ALL {
    let cards = state.hand(seat)[..3].to_vec();
    transition::apply(
      &mut state,
      Intention::SubmitPass { seat, cards },
      &mut IgnorePresentation,
    )
    .unwrap();
  }
  let before = state.random().clone();
  for played in [0, 7, 31, 48] {
    self::play_until(&mut state, played);
    for seat in Seat::ALL {
      let observation = state.observe(seat);
      let mut boundaries = 0;
      let batch = sampling::sample_deals(&observation, before.ai[seat.index()], || boundaries += 1);
      self::assert_consistent(&observation, &batch);
      assert!(boundaries <= SAMPLE_COUNT * (52 * 52 + 1) + 2);
      assert!(boundaries > SAMPLE_COUNT);
      assert_ne!(batch.next_stream, before.ai[seat.index()]);
    }
  }
  assert_eq!(state.random(), &before);
}

#[test]
fn matching_reassigns_flexible_slots_to_preserve_a_constrained_late_hand() {
  let mut observation = self::late_observation();
  observation.table.voids[Seat::North.index()] = [true, false, true, true];
  observation.table.voids[Seat::East.index()] = [true, true, true, false];
  observation.table.voids[Seat::West.index()] = [true, false, false, true];
  let batch = sampling::sample_deals(&observation, RandomStream::from_seed(71), || {});
  self::assert_consistent(&observation, &batch);
  for deal in batch.deals {
    assert_eq!(
      deal.hands[Seat::West.index()],
      [CardId::new(Suit::Spades, Rank::Ace)]
    );
    assert_eq!(
      deal.hands[Seat::North.index()],
      [CardId::new(Suit::Diamonds, Rank::Ace)]
    );
    assert_eq!(
      deal.hands[Seat::East.index()],
      [CardId::new(Suit::Hearts, Rank::Ace)]
    );
  }
}

#[test]
#[should_panic(expected = "observation has no feasible hidden deal")]
fn impossible_capacity_panics_instead_of_relaxing_constraints() {
  let mut observation = self::late_observation();
  observation.table.voids[Seat::West.index()] = [true, false, true, true];
  observation.table.voids[Seat::North.index()] = [true, false, true, true];
  sampling::sample_deals(&observation, RandomStream::from_seed(71), || {});
}

#[test]
fn hidden_assignments_and_secret_passes_never_change_sampled_search_input() {
  let original = HeartsState::new(27);
  let mut hands = Seat::ALL.map(|seat| original.hand(seat).to_vec());
  let left = hands[1][0];
  hands[1][0] = hands[2][0];
  hands[2][0] = left;
  let mut other = HeartsState::from_deal(hands, 0, [0; 4], RandomStreams::from_seed(999));
  let cards = other.hand(Seat::West)[..3].to_vec();
  transition::apply(
    &mut other,
    Intention::SubmitPass {
      seat: Seat::West,
      cards,
    },
    &mut IgnorePresentation,
  )
  .unwrap();
  let first = original.observe(Seat::South);
  let second = other.observe(Seat::South);
  assert_eq!(first, second);
  let stream = original.random().ai[Seat::South.index()];
  let expected = sampling::sample_deals(&first, stream, || {});
  assert_eq!(sampling::sample_deals(&second, stream, || {}), expected);
  let distinct: BTreeSet<_> = expected
    .deals
    .iter()
    .map(|deal| deal.hands.clone())
    .collect();
  assert!(distinct.len() > 1);
  let mut reordered = second;
  reordered.hand.reverse();
  assert_eq!(sampling::sample_deals(&reordered, stream, || {}), expected);
}

#[test]
fn the_public_opening_leader_fixes_two_of_clubs_after_passing() {
  let hands = Suit::ALL.map(|suit| Rank::ALL.map(|rank| CardId::new(suit, rank)).to_vec());
  let state = HeartsState::from_deal(hands, 3, [0; 4], RandomStreams::from_seed(1));
  let observation = state.observe(Seat::East);
  let batch = sampling::sample_deals(&observation, state.random().ai[Seat::East.index()], || {});
  for deal in batch.deals {
    assert!(deal.hands[Seat::South.index()].contains(&CardId::TWO_OF_CLUBS));
  }
}

#[test]
fn serialized_stream_continuation_replays_samples_and_common_rollout_randomness() {
  let state = HeartsState::new(92);
  let observation = state.observe(Seat::South);
  let streams = state.random().clone();
  assert!(streams.ai.iter().all(|stream| *stream != streams.deck));
  for index in 1..4 {
    assert!(!streams.ai[..index].contains(&streams.ai[index]));
  }
  let first = sampling::sample_deals(&observation, streams.ai[0], || {});
  let saved = serde_json::to_string(&first.next_stream).unwrap();
  let resumed = serde_json::from_str(&saved).unwrap();
  let continued = sampling::sample_deals(&observation, first.next_stream, || {});
  assert_eq!(
    sampling::sample_deals(&observation, resumed, || {}),
    continued
  );
  assert_ne!(first.deals, continued.deals);
  for deal in &first.deals {
    let mut candidate_a = deal.rollout_stream;
    let mut candidate_b = deal.rollout_stream;
    assert_eq!(
      (0..52).map(|_| candidate_a.below(52)).collect::<Vec<_>>(),
      (0..52).map(|_| candidate_b.below(52)).collect::<Vec<_>>()
    );
  }
  assert_eq!(state.random(), &streams);
}

#[test]
fn cancellation_during_matching_discards_partial_random_advancement() {
  let state = HeartsState::new(12);
  let observation = state.observe(Seat::South);
  let stream = state.random().ai[0];
  let expected = sampling::sample_deals(&observation, stream, || {});
  let input = observation.clone();
  let (reached, boundary) = mpsc::sync_channel(0);
  let (resume, resumed) = mpsc::sync_channel(0);
  let lane = ComputationLane::default();
  let job = lane.replace(move |token| {
    let mut count = 0;
    sampling::sample_deals(&input, stream, || {
      count += 1;
      if count == 7 {
        reached.send(()).unwrap();
        resumed.recv().unwrap();
      }
      token.checkpoint();
    })
  });
  boundary.recv_timeout(TIMEOUT).unwrap();
  lane.cancel();
  resume.send(()).unwrap();
  assert!(job.wait_for_stopped(TIMEOUT));
  assert_eq!(job.status(), ComputationStatus::Cancelled);
  assert!(job.take_result().is_none());
  assert_eq!(state.random().ai[0], stream);
  let retry =
    lane.replace(move |token| sampling::sample_deals(&observation, stream, || token.checkpoint()));
  assert!(retry.wait_for_stopped(TIMEOUT));
  assert_eq!(retry.take_result(), Some(expected));
}

fn assert_consistent(observation: &AiObservation, batch: &SampleBatch) {
  for deal in &batch.deals {
    assert_eq!(deal.hands[observation.seat.index()], observation.hand);
    let mut all: Vec<_> = deal.hands.iter().flatten().copied().collect();
    all.extend(observation.table.history.iter().map(|played| played.card));
    all.sort_unstable();
    assert_eq!(all, cards::deck());
    for seat in Seat::ALL {
      assert_eq!(
        deal.hands[seat.index()].len(),
        observation.table.hand_counts[seat.index()]
      );
      assert!(
        deal.hands[seat.index()]
          .iter()
          .all(|card| !observation.table.voids[seat.index()][card.suit.index()])
      );
    }
    if let Some(known) = &observation.known_pass {
      assert!(
        known
          .cards
          .iter()
          .all(|card| deal.hands[known.recipient.index()].contains(card))
      );
    }
  }
}

fn late_observation() -> AiObservation {
  let hands = Suit::ALL.map(|suit| Rank::ALL.map(|rank| CardId::new(suit, rank)).to_vec());
  let mut state = HeartsState::from_deal(hands, 3, [0; 4], RandomStreams::from_seed(1));
  self::play_until(&mut state, 48);
  state.observe(Seat::South)
}

fn play_until(state: &mut HeartsState, played: usize) {
  while state.public_table().history.len() < played {
    let Phase::Playing { turn } = state.phase() else {
      panic!("fixture play")
    };
    let card = state.observe(turn).legal_plays[0];
    transition::apply(
      state,
      Intention::PlayCard { seat: turn, card },
      &mut IgnorePresentation,
    )
    .unwrap();
  }
}
