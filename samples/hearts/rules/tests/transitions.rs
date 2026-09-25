use std::collections::BTreeSet;

use battlement_hearts_rules::domain::{
  CardId, Checkpoint, Event, Hands, HeartsState, IgnorePresentation, Intention, PassDirection,
  Phase, PresentationSink, RandomStreams, Rank, Rejection, Seat, Suit, Zone,
};
use battlement_hearts_rules::domain::{cards, scoring, transition};

#[test]
fn all_pass_cycles_exchange_atomically_and_pending_cards_still_belong_to_hands() {
  for (hand_index, direction) in [
    (0, PassDirection::Left),
    (1, PassDirection::Right),
    (2, PassDirection::Across),
  ] {
    let hands = suit_hands();
    let mut state = HeartsState::from_deal(
      hands.clone(),
      hand_index,
      [0; 4],
      RandomStreams::from_seed(1),
    );
    let mut frames = Vec::<Checkpoint>::new();
    for seat in Seat::ALL {
      transition::apply(
        &mut state,
        Intention::SubmitPass {
          seat,
          cards: hands[seat.index()][..3].to_vec(),
        },
        &mut frames,
      )
      .unwrap();
      if seat != Seat::East {
        for owner in Seat::ALL {
          assert_eq!(state.hand(owner), hands[owner.index()]);
        }
        assert_eq!(state.zone(hands[seat.index()][0]), Zone::Hand(seat));
      }
      assert_conserved(&state);
      if seat == Seat::South {
        let restored: HeartsState =
          serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert_eq!(restored, state);
        state = restored;
      }
    }
    assert_eq!(frames.last().unwrap().event, Event::PassExchanged);
    for sender in Seat::ALL {
      let recipient = direction.recipient(sender).unwrap();
      for card in &hands[sender.index()][..3] {
        assert_eq!(state.zone(*card), Zone::Hand(recipient));
      }
    }
    let opener = direction.recipient(Seat::South).unwrap();
    assert_eq!(state.phase(), Phase::Playing { turn: opener });
    assert_eq!(state.observe(opener).legal_plays, [CardId::TWO_OF_CLUBS]);
  }
  let mut hold = HeartsState::from_deal(suit_hands(), 3, [0; 4], RandomStreams::from_seed(1));
  assert_eq!(hold.phase(), Phase::Playing { turn: Seat::South });
  assert_eq!(
    transition::apply(
      &mut hold,
      Intention::SubmitPass {
        seat: Seat::South,
        cards: vec![]
      },
      &mut IgnorePresentation
    ),
    Err(Rejection::WrongPhase)
  );
}

#[test]
fn rejected_intentions_leave_all_state_and_presentation_unchanged() {
  let mut state = HeartsState::new(7);
  let mut frames = Vec::<Checkpoint>::new();
  let missing = *state.hand(Seat::West).first().unwrap();
  for command in [
    Intention::PlayCard {
      seat: Seat::South,
      card: state.hand(Seat::South)[0],
    },
    Intention::NextHand,
    Intention::SubmitPass {
      seat: Seat::South,
      cards: vec![missing; 3],
    },
    Intention::SubmitPass {
      seat: Seat::South,
      cards: state.hand(Seat::South)[..2].to_vec(),
    },
  ] {
    let before = state.clone();
    assert!(transition::apply(&mut state, command, &mut frames).is_err());
    assert_eq!(state, before);
    assert!(frames.is_empty());
  }
  let pass = Intention::SubmitPass {
    seat: Seat::South,
    cards: state.hand(Seat::South)[..3].to_vec(),
  };
  transition::apply(&mut state, pass.clone(), &mut frames).unwrap();
  let before = state.clone();
  let previous_frames = frames.clone();
  assert_eq!(
    transition::apply(&mut state, pass, &mut frames),
    Err(Rejection::AlreadyPassed)
  );
  assert_eq!(state, before);
  assert_eq!(frames, previous_frames);
}

#[test]
fn pure_reduction_preserves_accepted_state_until_the_caller_commits() {
  let accepted = HeartsState::from_deal(suit_hands(), 3, [0; 4], RandomStreams::from_seed(2));
  let before = accepted.clone();
  let mut frames = Vec::<Checkpoint>::new();
  let next = transition::reduce(
    &accepted,
    Intention::PlayCard {
      seat: Seat::South,
      card: CardId::TWO_OF_CLUBS,
    },
    &mut frames,
  )
  .unwrap();
  assert_eq!(accepted, before);
  assert_eq!(accepted.zone(CardId::TWO_OF_CLUBS), Zone::Hand(Seat::South));
  assert_eq!(next.zone(CardId::TWO_OF_CLUBS), Zone::Trick);
  assert_eq!(frames.last().unwrap().state, next);
}

#[test]
fn fourth_card_is_presented_before_collection_and_moon_scoring() {
  let mut state = HeartsState::from_deal(suit_hands(), 3, [0; 4], RandomStreams::from_seed(1));
  let mut frames = Vec::<Checkpoint>::new();
  finish_hand(&mut state, &mut frames);
  let completed: Vec<_> = frames
    .iter()
    .filter(|frame| frame.state.trick().len() == 4)
    .collect();
  assert_eq!(completed.len(), 13);
  for frame in &frames {
    assert_conserved(&frame.state);
  }
  let last = frames.len() - 1;
  assert!(matches!(frames[last].event, Event::HandScored { .. }));
  assert!(matches!(
    frames[last - 1].event,
    Event::TrickCollected {
      winner: Seat::South,
      ..
    }
  ));
  assert_eq!(frames[last - 2].state.trick().len(), 4);
  assert!(
    Seat::ALL
      .into_iter()
      .all(|seat| frames[last - 2].state.observe(seat).legal_plays.is_empty())
  );
  let table = state.public_table();
  assert_eq!(table.result.unwrap().moon, Some(Seat::South));
  assert_eq!(table.result.unwrap().points, [0, 26, 26, 26]);
  assert_eq!(table.totals, [0, 26, 26, 26]);
  assert_eq!(state.captured(Seat::South).len(), 52);
  assert_eq!(state.phase(), Phase::HandOver);
  let result = state.public_table();
  let random = state.random().clone();
  assert_eq!(state.public_table(), result);
  transition::apply(&mut state, Intention::NextHand, &mut frames).unwrap();
  assert_eq!(state.public_table().hand_index, 4);
  assert_eq!(state.public_table().totals, result.totals);
  assert_eq!(state.public_table().result, None);
  assert_eq!(state.random().ai, random.ai);
  assert_ne!(state.random().deck, random.deck);
  assert_conserved(&state);
}

#[test]
fn reaching_one_hundred_ends_after_scoring_and_lowest_ties_share_the_win() {
  let mut state =
    HeartsState::from_deal(suit_hands(), 3, [26, 0, 74, 0], RandomStreams::from_seed(2));
  let mut frames = Vec::<Checkpoint>::new();
  finish_hand(&mut state, &mut frames);
  assert_eq!(state.public_table().totals, [26, 26, 100, 26]);
  let winners = [true, true, false, true];
  assert_eq!(state.phase(), Phase::MatchOver { winners });
  assert_eq!(frames.last().unwrap().event, Event::MatchEnded { winners });
  assert!(matches!(
    frames[frames.len() - 2].event,
    Event::HandScored { .. }
  ));
  let before = state.clone();
  assert_eq!(
    transition::apply(&mut state, Intention::NextHand, &mut frames),
    Err(Rejection::WrongPhase)
  );
  assert_eq!(state, before);
}

#[test]
fn ordinary_penalties_add_without_a_moon_and_match_end_uses_total_scores() {
  let mut captured: Hands = std::array::from_fn(|_| Vec::new());
  for card in cards::deck() {
    let owner = if card == CardId::QUEEN_OF_SPADES {
      1
    } else if card.suit == Suit::Hearts {
      if card.rank < Rank::Seven { 0 } else { 2 }
    } else {
      3
    };
    captured[owner].push(card);
  }
  let result = scoring::score(&captured);
  assert_eq!(result.points, [5, 13, 8, 0]);
  assert_eq!(result.moon, None);
  assert_eq!(scoring::winners([99, 99, 99, 99]), None);
  assert_eq!(
    scoring::winners([100, 22, 6, 22]),
    Some([false, false, true, false])
  );
}

#[test]
fn queen_does_not_break_hearts_and_follow_suit_voids_are_public() {
  let mut hands: Hands = [
    vec![CardId::TWO_OF_CLUBS],
    vec![CardId::new(Suit::Clubs, Rank::Three)],
    vec![CardId::new(Suit::Clubs, Rank::Four)],
    vec![],
  ];
  hands[0].extend(
    Rank::ALL[..12]
      .iter()
      .map(|&rank| CardId::new(Suit::Diamonds, rank)),
  );
  hands[1].extend(
    Rank::ALL[..12]
      .iter()
      .map(|&rank| CardId::new(Suit::Spades, rank)),
  );
  hands[2].extend(
    Rank::ALL[..12]
      .iter()
      .map(|&rank| CardId::new(Suit::Hearts, rank)),
  );
  hands[3] = cards::deck()
    .into_iter()
    .filter(|card| !hands[..3].iter().flatten().any(|held| held == card))
    .collect();
  let mut state = HeartsState::from_deal(hands, 3, [0; 4], RandomStreams::from_seed(9));
  let mut frames = Vec::<Checkpoint>::new();
  for (seat, card) in [
    (Seat::South, CardId::TWO_OF_CLUBS),
    (Seat::West, CardId::new(Suit::Clubs, Rank::Three)),
    (Seat::North, CardId::new(Suit::Clubs, Rank::Four)),
    (Seat::East, CardId::new(Suit::Clubs, Rank::Ace)),
    (Seat::East, CardId::new(Suit::Spades, Rank::Ace)),
    (Seat::South, CardId::new(Suit::Diamonds, Rank::Two)),
    (Seat::West, CardId::QUEEN_OF_SPADES),
  ] {
    transition::apply(&mut state, Intention::PlayCard { seat, card }, &mut frames).unwrap();
  }
  assert!(!state.public_table().hearts_broken);
  assert!(state.public_table().voids[Seat::South.index()][Suit::Spades.index()]);
  assert!(matches!(
    frames.last().unwrap().event,
    Event::CardPlayed {
      broke_hearts: false,
      ..
    }
  ));
  transition::apply(
    &mut state,
    Intention::PlayCard {
      seat: Seat::North,
      card: CardId::new(Suit::Hearts, Rank::Two),
    },
    &mut frames,
  )
  .unwrap();
  assert!(state.public_table().hearts_broken);
  assert_eq!(state.phase(), Phase::Playing { turn: Seat::East });
  assert!(frames.iter().any(|frame| matches!(
    frame.event,
    Event::CardPlayed {
      broke_hearts: true,
      ..
    }
  )));
}

#[test]
fn live_and_snapshot_free_simulation_resume_identical_seeded_matches() {
  for seed in 0..8 {
    let mut live = HeartsState::new(seed);
    let mut simulated = live.clone();
    let mut frames = Vec::<Checkpoint>::new();
    let mut commands = 0;
    while let Some(command) = next_command(&live) {
      transition::apply(&mut live, command.clone(), &mut frames).unwrap();
      transition::apply(&mut simulated, command, &mut IgnorePresentation).unwrap();
      assert_eq!(live, simulated);
      assert_conserved(&live);
      commands += 1;
      if commands % 17 == 0 {
        simulated = serde_json::from_str(&serde_json::to_string(&simulated).unwrap()).unwrap();
      }
      frames.clear();
      assert!(commands < 1000, "match failed to finish");
    }
    assert!(matches!(live.phase(), Phase::MatchOver { .. }));
    assert!(live.public_table().hand_index > 0);
  }
}

fn suit_hands() -> Hands {
  Suit::ALL.map(|suit| Rank::ALL.map(|rank| CardId::new(suit, rank)).to_vec())
}

fn assert_conserved(state: &HeartsState) {
  let mut owned = Vec::new();
  for seat in Seat::ALL {
    owned.extend(state.hand(seat));
    owned.extend(state.captured(seat));
  }
  owned.extend(state.trick().iter().map(|played| &played.card));
  assert_eq!(owned.len(), 52);
  assert_eq!(
    owned.into_iter().copied().collect::<BTreeSet<_>>(),
    cards::deck().into_iter().collect()
  );
}

fn next_command(state: &HeartsState) -> Option<Intention> {
  match state.phase() {
    Phase::Passing => {
      let seat = Seat::ALL
        .into_iter()
        .find(|&seat| state.observe(seat).pending_pass.is_none())
        .unwrap();
      Some(Intention::SubmitPass {
        seat,
        cards: state.hand(seat)[..3].to_vec(),
      })
    }
    Phase::Playing { turn } => Some(Intention::PlayCard {
      seat: turn,
      card: state.observe(turn).legal_plays[0],
    }),
    Phase::HandOver => Some(Intention::NextHand),
    Phase::MatchOver { .. } => None,
  }
}

fn finish_hand(state: &mut HeartsState, sink: &mut impl PresentationSink) {
  while matches!(state.phase(), Phase::Passing | Phase::Playing { .. }) {
    transition::apply(state, next_command(state).unwrap(), sink).unwrap();
  }
}
