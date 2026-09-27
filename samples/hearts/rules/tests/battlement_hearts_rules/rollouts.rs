use std::{collections::BTreeSet, time::Instant};

use battlement_hearts_rules::{
  ai::{
    policy,
    sampling::{self, SampleDeal},
    search,
  },
  domain::{
    CardId, HeartsState, IgnorePresentation, Intention, Phase, RandomStream, RandomStreams, Rank,
    Seat, Suit, simulation, transition,
  },
};

#[test]
fn search_is_private_and_resumes_identically_after_serialization() {
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
  let observation = original.observe(Seat::South);
  assert_eq!(observation, other.observe(Seat::South));
  let stream = original.random().ai[0];
  let start = Instant::now();
  let decision = search::evaluate(observation.clone(), stream, || {});
  eprintln!(
    "passing decision {:?} elapsed {:?}",
    decision.intention,
    start.elapsed()
  );
  assert_eq!(
    decision,
    search::evaluate(other.observe(Seat::South), stream, || {})
  );
  let saved = serde_json::to_string(&original).unwrap();
  let restored: HeartsState = serde_json::from_str(&saved).unwrap();
  assert_eq!(
    decision,
    search::evaluate(
      restored.observe(Seat::South),
      restored.random().ai[0],
      || {}
    )
  );
  let passes = policy::passes(&observation);
  assert_eq!(passes.len(), 8);
  assert_eq!(passes.iter().collect::<BTreeSet<_>>().len(), 8);
  let Intention::SubmitPass { cards, .. } = decision.intention else {
    panic!("pass")
  };
  assert!(passes.contains(&cards));
  assert_eq!(
    cards,
    vec![
      CardId::new(Suit::Spades, Rank::Queen),
      CardId::new(Suit::Hearts, Rank::Eight),
      CardId::new(Suit::Hearts, Rank::King)
    ]
  );
  assert_ne!(decision.next_stream, stream);
  assert_eq!(original.random().ai[0], stream);
}

#[test]
fn sampled_rollouts_match_live_shared_transitions_and_scoring() {
  let mut live = HeartsState::new(91);
  for seat in Seat::ALL {
    let cards = policy::passes(&live.observe(seat)).remove(0);
    transition::apply(
      &mut live,
      Intention::SubmitPass { seat, cards },
      &mut IgnorePresentation,
    )
    .unwrap();
  }
  let mut stream = RandomStream::from_seed(100);
  for _ in 0..17 {
    let Phase::Playing { turn } = live.phase() else {
      panic!("play")
    };
    let card = policy::play(&live.observe(turn), &mut stream);
    transition::apply(
      &mut live,
      Intention::PlayCard { seat: turn, card },
      &mut IgnorePresentation,
    )
    .unwrap();
  }
  let Phase::Playing { turn } = live.phase() else {
    panic!("play")
  };
  let observation = live.observe(turn);
  let candidate = Intention::PlayCard {
    seat: turn,
    card: observation.legal_plays[0],
  };
  let deal = SampleDeal {
    hands: Seat::ALL.map(|seat| live.hand(seat).to_vec()),
    rollout_stream: stream,
  };
  let reconstructed = simulation::from_observation(&observation, deal.hands.clone());
  assert_eq!(live.public_table(), reconstructed.public_table());
  for seat in Seat::ALL {
    assert_eq!(live.captured(seat), reconstructed.captured(seat));
  }
  let result = search::rollout(&observation, &deal, candidate.clone(), || {});
  transition::apply(&mut live, candidate, &mut IgnorePresentation).unwrap();
  while let Phase::Playing { turn } = live.phase() {
    let card = policy::play(&live.observe(turn), &mut stream);
    transition::apply(
      &mut live,
      Intention::PlayCard { seat: turn, card },
      &mut IgnorePresentation,
    )
    .unwrap();
  }
  assert_eq!(Some(result), live.public_table().result);
}

#[test]
fn a_complete_match_uses_fair_search_with_bounded_work_and_no_deck_advancement() {
  let mut state = HeartsState::new(43);
  let mut decisions = 0;
  let start = Instant::now();
  for _ in 0..16 * 57 {
    let seat = match state.phase() {
      Phase::Passing => Seat::ALL
        .into_iter()
        .find(|seat| state.observe(*seat).pending_pass.is_none())
        .unwrap(),
      Phase::Playing { turn } => turn,
      Phase::HandOver => {
        transition::apply(&mut state, Intention::NextHand, &mut IgnorePresentation).unwrap();
        continue;
      }
      Phase::MatchOver { winners } => {
        assert!(winners.contains(&true));
        assert!(
          state
            .public_table()
            .totals
            .iter()
            .any(|score| *score >= 100)
        );
        eprintln!(
          "complete match: {decisions} searches in {:?}, totals {:?}",
          start.elapsed(),
          state.public_table().totals
        );
        return;
      }
    };
    let before = state.random().clone();
    let observation = state.observe(seat);
    let mut boundaries = 0;
    let decision = search::evaluate(observation, before.ai[seat.index()], || boundaries += 1);
    assert!(
      boundaries
        <= sampling::SAMPLE_COUNT * (52 * 52 + 1) + 13 * (1 + sampling::SAMPLE_COUNT * 57) + 3
    );
    transition::apply(&mut state, decision.intention, &mut IgnorePresentation).unwrap();
    assert_eq!(state.random(), &before);
    // The real reducer acceptance tests cover committing this detached stream.
    let mut saved = serde_json::to_value(&state).unwrap();
    saved["random"]["ai"][seat.index()] = serde_json::to_value(decision.next_stream).unwrap();
    state = serde_json::from_value(saved).unwrap();
    decisions += 1;
  }
  panic!("match exceeded its finite scoring bound");
}

#[test]
fn rollout_uses_ordinary_moon_scoring() {
  let hands = Suit::ALL.map(|suit| Rank::ALL.map(|rank| CardId::new(suit, rank)).to_vec());
  let state = HeartsState::from_deal(hands.clone(), 3, [0; 4], RandomStreams::from_seed(1));
  let observation = state.observe(Seat::South);
  let result = search::rollout(
    &observation,
    &SampleDeal {
      hands,
      rollout_stream: RandomStream::from_seed(1),
    },
    Intention::PlayCard {
      seat: Seat::South,
      card: CardId::TWO_OF_CLUBS,
    },
    || {},
  );
  assert_eq!(result.moon, Some(Seat::South));
  assert_eq!(result.points, [0, 26, 26, 26]);
}
