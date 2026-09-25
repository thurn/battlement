use battlement_hearts_rules::domain::choices;
use battlement_hearts_rules::domain::{
  CardId, PassDirection, PlayContext, PlayedCard, Rank, Rejection, Seat, Suit,
};

#[test]
fn two_of_clubs_opens_and_following_suit_precedes_discarding() {
  let clubs = CardId::new(Suit::Clubs, Rank::Ace);
  let diamond = CardId::new(Suit::Diamonds, Rank::Two);
  let hand = [CardId::TWO_OF_CLUBS, clubs, diamond];
  let opening = PlayContext {
    hand: &hand,
    seat: Seat::South,
    turn: Seat::South,
    trick: &[],
    first_trick: true,
    hearts_broken: false,
  };
  assert_eq!(opening.legal_plays(), [CardId::TWO_OF_CLUBS]);
  assert_eq!(opening.validate(diamond), Err(Rejection::MustOpenTwoClubs));
  let following = PlayContext {
    hand: &[clubs, diamond],
    trick: &[PlayedCard {
      seat: Seat::East,
      card: CardId::TWO_OF_CLUBS,
    }],
    ..opening
  };
  assert_eq!(following.legal_plays(), [clubs]);
  assert_eq!(following.validate(diamond), Err(Rejection::MustFollowSuit));
  assert_eq!(following.validate(clubs), Ok(()));
}

#[test]
fn first_trick_penalties_are_allowed_only_when_forced() {
  let heart = CardId::new(Suit::Hearts, Rank::Ace);
  let diamond = CardId::new(Suit::Diamonds, Rank::Two);
  let queen = CardId::QUEEN_OF_SPADES;
  let context = PlayContext {
    hand: &[heart, queen, diamond],
    seat: Seat::West,
    turn: Seat::West,
    trick: &[PlayedCard {
      seat: Seat::South,
      card: CardId::TWO_OF_CLUBS,
    }],
    first_trick: true,
    hearts_broken: false,
  };
  assert_eq!(context.legal_plays(), [diamond]);
  for penalty in [heart, queen] {
    assert_eq!(context.validate(penalty), Err(Rejection::FirstTrickPenalty));
  }
  let forced = PlayContext {
    hand: &[heart, queen],
    ..context
  };
  assert_eq!(forced.legal_plays(), [heart, queen]);
  assert_eq!(
    PlayContext {
      hand: &[queen],
      ..context
    }
    .legal_plays(),
    [queen]
  );
  assert_eq!(
    PlayContext {
      hand: &[heart],
      ..context
    }
    .legal_plays(),
    [heart]
  );
  assert_eq!(
    PlayContext {
      first_trick: false,
      ..context
    }
    .legal_plays(),
    [heart, queen, diamond]
  );
}

#[test]
fn unbroken_hearts_allow_only_all_hearts_leads_but_never_restrict_queen_leads() {
  let heart = CardId::new(Suit::Hearts, Rank::Two);
  let higher_heart = CardId::new(Suit::Hearts, Rank::Ace);
  let queen = CardId::QUEEN_OF_SPADES;
  let context = PlayContext {
    hand: &[heart, queen],
    seat: Seat::North,
    turn: Seat::North,
    trick: &[],
    first_trick: false,
    hearts_broken: false,
  };
  assert_eq!(context.validate(heart), Err(Rejection::HeartsNotBroken));
  assert_eq!(context.legal_plays(), [queen]);
  assert_eq!(
    PlayContext {
      hearts_broken: true,
      ..context
    }
    .legal_plays(),
    [heart, queen]
  );
  assert_eq!(
    PlayContext {
      hand: &[heart, higher_heart],
      ..context
    }
    .legal_plays(),
    [heart, higher_heart]
  );
  let following = PlayContext {
    trick: &[PlayedCard {
      seat: Seat::West,
      card: queen,
    }],
    hand: &[heart, higher_heart],
    ..context
  };
  assert_eq!(following.legal_plays(), [heart, higher_heart]);
}

#[test]
fn illegal_external_choices_preserve_card_ownership() {
  let hand = vec![
    CardId::new(Suit::Hearts, Rank::King),
    CardId::QUEEN_OF_SPADES,
  ];
  let before = serde_json::to_string(&hand).unwrap();
  let context = PlayContext {
    hand: &hand,
    seat: Seat::East,
    turn: Seat::South,
    trick: &[],
    first_trick: false,
    hearts_broken: false,
  };
  assert_eq!(context.validate(hand[0]), Err(Rejection::NotYourTurn));
  assert!(context.legal_plays().is_empty());
  let current = PlayContext {
    turn: Seat::East,
    ..context
  };
  assert_eq!(
    current.validate(CardId::TWO_OF_CLUBS),
    Err(Rejection::CardNotHeld)
  );
  assert_eq!(current.validate(hand[0]), Err(Rejection::HeartsNotBroken));
  assert_eq!(serde_json::to_string(&hand).unwrap(), before);
  assert_eq!(
    Rejection::MustFollowSuit.to_string(),
    "Follow the led suit while you have it."
  );
}

#[test]
fn passing_requires_exactly_three_different_owned_cards_and_preserves_the_hand() {
  let hand = [
    CardId::TWO_OF_CLUBS,
    CardId::QUEEN_OF_SPADES,
    CardId::new(Suit::Hearts, Rank::Ace),
    CardId::new(Suit::Diamonds, Rank::Ten),
  ];
  let before = hand;
  for direction in [
    PassDirection::Left,
    PassDirection::Right,
    PassDirection::Across,
  ] {
    assert_eq!(choices::pass_choices(&hand, direction), hand);
    assert_eq!(choices::validate_pass(&hand, direction, &hand[..3]), Ok(()));
    assert_eq!(
      choices::validate_pass(&hand, direction, &hand[..2]),
      Err(Rejection::PassCount)
    );
    assert_eq!(
      choices::validate_pass(&hand, direction, &hand),
      Err(Rejection::PassCount)
    );
    assert_eq!(
      choices::validate_pass(&hand, direction, &[hand[0], hand[0], hand[1]]),
      Err(Rejection::DuplicatePassCard)
    );
    let missing = CardId::new(Suit::Diamonds, Rank::Ace);
    assert_eq!(
      choices::validate_pass(&hand, direction, &[hand[0], missing, hand[1]]),
      Err(Rejection::CardNotHeld)
    );
  }
  assert!(choices::pass_choices(&hand, PassDirection::Hold).is_empty());
  assert_eq!(
    choices::validate_pass(&hand, PassDirection::Hold, &hand[..3]),
    Err(Rejection::NoPassing)
  );
  assert_eq!(hand, before);
}
