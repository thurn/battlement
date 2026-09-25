use std::collections::BTreeSet;

use battlement_hearts_rules::domain::cards;
use battlement_hearts_rules::domain::deal;
use battlement_hearts_rules::domain::{
  CardId, PassDirection, RandomStream, RandomStreams, Rank, Seat, Suit,
};

#[test]
fn seeded_deals_conserve_the_deck_and_stably_order_each_hand() {
  let expected: BTreeSet<_> = cards::deck().into_iter().collect();
  assert_eq!(expected.len(), 52);
  for seed in 0..64 {
    let hands = deal::shuffled(&mut RandomStream::from_seed(seed));
    assert!(hands.iter().all(|hand| hand.len() == 13));
    assert!(
      hands
        .iter()
        .all(|hand| hand.windows(2).all(|pair| pair[0] < pair[1]))
    );
    assert_eq!(
      hands.into_iter().flatten().collect::<BTreeSet<_>>(),
      expected
    );
  }
  assert_eq!(cards::deck().first(), Some(&CardId::TWO_OF_CLUBS));
  assert_eq!(
    cards::deck().last(),
    Some(&CardId::new(Suit::Hearts, Rank::Ace))
  );
}

#[test]
fn fixed_seed_pins_the_deal_and_successive_random_continuation() {
  let mut rng = RandomStream::from_seed(42);
  let hands = deal::shuffled(&mut rng);
  assert_eq!(
    hands[Seat::South.index()],
    [
      CardId::new(Suit::Clubs, Rank::Three),
      CardId::new(Suit::Clubs, Rank::Four),
      CardId::new(Suit::Clubs, Rank::Eight),
      CardId::new(Suit::Clubs, Rank::Nine),
      CardId::new(Suit::Clubs, Rank::King),
      CardId::new(Suit::Diamonds, Rank::Six),
      CardId::new(Suit::Diamonds, Rank::King),
      CardId::new(Suit::Spades, Rank::Four),
      CardId::new(Suit::Spades, Rank::Five),
      CardId::new(Suit::Spades, Rank::Queen),
      CardId::new(Suit::Spades, Rank::King),
      CardId::new(Suit::Spades, Rank::Ace),
      CardId::new(Suit::Hearts, Rank::Two),
    ]
  );
  assert_eq!(
    serde_json::to_string(&rng).unwrap(),
    r#"{"state":120259934483646729}"#
  );
  assert_eq!(hands, deal::shuffled(&mut RandomStream::from_seed(42)));
}

#[test]
fn serialized_streams_resume_each_seat_and_dealing_independently() {
  let mut streams = RandomStreams::from_seed(123);
  let first = deal::shuffled(&mut streams.deck);
  for seat in Seat::ALL {
    for _ in 0..seat.index() + 1 {
      streams.ai[seat.index()].below(97);
    }
  }
  let mut restored: RandomStreams =
    serde_json::from_str(&serde_json::to_string(&streams).unwrap()).unwrap();
  assert_eq!(restored, streams);
  let next = deal::shuffled(&mut streams.deck);
  assert_ne!(first, next);
  assert_eq!(next, deal::shuffled(&mut restored.deck));
  for seat in Seat::ALL {
    let index = seat.index();
    assert_eq!(
      streams.ai[index].below(1_000_000),
      restored.ai[index].below(1_000_000)
    );
  }
  assert_eq!(streams, restored);
  for _ in 0..100 {
    streams.ai[Seat::South.index()].below(101);
  }
  assert_eq!(
    deal::shuffled(&mut streams.deck),
    deal::shuffled(&mut restored.deck)
  );
  assert_eq!(
    streams.ai[Seat::West.index()],
    restored.ai[Seat::West.index()]
  );
  assert_ne!(
    streams.ai[Seat::South.index()],
    restored.ai[Seat::South.index()]
  );
}

#[test]
fn pass_cycle_has_clockwise_recipients_and_a_hold_hand() {
  let cycle = [
    PassDirection::Left,
    PassDirection::Right,
    PassDirection::Across,
    PassDirection::Hold,
  ];
  for index in 0..12 {
    assert_eq!(PassDirection::for_hand(index), cycle[index as usize % 4]);
  }
  for (sender, left, right, across) in [
    (Seat::South, Seat::West, Seat::East, Seat::North),
    (Seat::West, Seat::North, Seat::South, Seat::East),
    (Seat::North, Seat::East, Seat::West, Seat::South),
    (Seat::East, Seat::South, Seat::North, Seat::West),
  ] {
    assert_eq!(PassDirection::Left.recipient(sender), Some(left));
    assert_eq!(PassDirection::Right.recipient(sender), Some(right));
    assert_eq!(PassDirection::Across.recipient(sender), Some(across));
    assert_eq!(PassDirection::Hold.recipient(sender), None);
  }
}
