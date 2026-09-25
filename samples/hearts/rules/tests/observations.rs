use battlement_hearts_rules::domain::{
  CardId, Checkpoint, HeartsState, IgnorePresentation, Intention, Phase, RandomStreams, Seat,
};
use battlement_hearts_rules::domain::{cards, transition};
use battlement_hearts_rules::projection::Projection;

#[test]
fn private_opponent_cards_randomness_and_pass_submissions_do_not_enter_observations() {
  let original = HeartsState::new(27);
  let mut hands = Seat::ALL.map(|seat| original.hand(seat).to_vec());
  let left = hands[1][0];
  let right = hands[2][0];
  hands[1][0] = right;
  hands[2][0] = left;
  let mut different = HeartsState::from_deal(hands, 0, [0; 4], RandomStreams::from_seed(999));
  assert_eq!(
    original.observe(Seat::South),
    different.observe(Seat::South)
  );
  let before = different.observe(Seat::South);
  let projection = Projection::default();
  let human_before = projection.view(&different, Seat::South);
  let pass = different.hand(Seat::West)[..3].to_vec();
  transition::apply(
    &mut different,
    Intention::SubmitPass {
      seat: Seat::West,
      cards: pass,
    },
    &mut IgnorePresentation,
  )
  .unwrap();
  assert_eq!(different.observe(Seat::South), before);
  assert_eq!(projection.view(&different, Seat::South), human_before);
  assert!(different.observe(Seat::West).pending_pass.is_some());
  assert_eq!(human_before.table.leader, None);
  for seat in [Seat::West, Seat::North, Seat::East] {
    assert_eq!(human_before.hands[seat.index()].len(), 13);
    assert!(
      human_before.hands[seat.index()]
        .iter()
        .all(|card| card.face.is_none())
    );
    assert!(human_before.hands[seat.index()].iter().all(|card| {
      projection
        .resolve_owned(&different, Seat::South, card.token)
        .is_none()
    }));
  }
  assert_eq!(
    human_before.hands[0]
      .iter()
      .map(|card| card.face.unwrap())
      .collect::<Vec<_>>(),
    original.hand(Seat::South)
  );
}

#[test]
fn an_unseen_two_of_clubs_owner_is_private_until_passing_finishes() {
  let mut hands = [vec![], vec![], vec![], vec![]];
  for (index, card) in cards::deck().into_iter().enumerate() {
    hands[index % 4].push(card);
  }
  let original = HeartsState::from_deal(hands.clone(), 0, [0; 4], RandomStreams::from_seed(1));
  let other = hands[1][0];
  hands[1][0] = CardId::TWO_OF_CLUBS;
  hands[0][0] = other;
  let swapped = HeartsState::from_deal(hands, 0, [0; 4], RandomStreams::from_seed(1));
  assert_eq!(original.observe(Seat::East), swapped.observe(Seat::East));
}

#[test]
fn only_known_outgoing_passes_are_exposed_and_public_plays_remove_hidden_ownership() {
  let mut state = HeartsState::new(12);
  let outgoing = state.hand(Seat::South)[..3].to_vec();
  for seat in Seat::ALL {
    let pass = state.hand(seat)[..3].to_vec();
    transition::apply(
      &mut state,
      Intention::SubmitPass { seat, cards: pass },
      &mut IgnorePresentation,
    )
    .unwrap();
  }
  let observed = state.observe(Seat::South);
  assert_eq!(observed.pending_pass, None);
  let known = observed.known_pass.unwrap();
  assert_eq!(known.recipient, Seat::West);
  assert_eq!(known.cards, outgoing);
  let mut played_known = false;
  while let Phase::Playing { turn } = state.phase() {
    let card = state.observe(turn).legal_plays[0];
    transition::apply(
      &mut state,
      Intention::PlayCard { seat: turn, card },
      &mut IgnorePresentation,
    )
    .unwrap();
    if outgoing.contains(&card) {
      assert!(
        !state
          .observe(Seat::South)
          .known_pass
          .unwrap()
          .cards
          .contains(&card)
      );
      played_known = true;
      break;
    }
  }
  assert!(played_known);
}

#[test]
fn opaque_identity_survives_reveal_and_capture_but_is_fresh_after_restore() {
  let initial = HeartsState::new(9);
  let hands = Seat::ALL.map(|seat| initial.hand(seat).to_vec());
  let mut state = HeartsState::from_deal(hands, 3, [0; 4], initial.random().clone());
  let projection = Projection::default();
  let Phase::Playing { turn: opener } = state.phase() else {
    panic!()
  };
  let own = projection.view(&state, opener);
  let token = own.hands[opener.index()]
    .iter()
    .find(|card| card.face == Some(CardId::TWO_OF_CLUBS))
    .unwrap()
    .token;
  let hidden = projection.view(&state, opener.clockwise(1));
  assert!(
    hidden.hands[opener.index()]
      .iter()
      .any(|card| card.token == token && card.face.is_none())
  );
  assert_eq!(
    projection.resolve_owned(&state, opener, token),
    Some(CardId::TWO_OF_CLUBS)
  );
  let random = state.random().clone();
  let mut frames = Vec::<Checkpoint>::new();
  transition::apply(
    &mut state,
    Intention::PlayCard {
      seat: opener,
      card: CardId::TWO_OF_CLUBS,
    },
    &mut frames,
  )
  .unwrap();
  let visible = projection.view(&state, opener.clockwise(1));
  assert_eq!(visible.trick[0].1.token, token);
  assert_eq!(visible.trick[0].1.face, Some(CardId::TWO_OF_CLUBS));
  for _ in 0..3 {
    let Phase::Playing { turn } = state.phase() else {
      panic!()
    };
    let card = state.observe(turn).legal_plays[0];
    transition::apply(
      &mut state,
      Intention::PlayCard { seat: turn, card },
      &mut frames,
    )
    .unwrap();
  }
  let visible = projection.view(&state, opener);
  assert!(
    visible
      .captured
      .iter()
      .flatten()
      .any(|card| card.token == token)
  );
  let complete = frames
    .iter()
    .find(|frame| frame.state.trick().len() == 4)
    .unwrap();
  assert!(
    projection
      .view(&complete.state, opener)
      .legal_plays
      .is_empty()
  );
  let restored: HeartsState =
    serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
  let fresh = Projection::default().view(&restored, opener);
  assert_eq!(fresh.table, visible.table);
  assert!(
    fresh
      .captured
      .iter()
      .flatten()
      .all(|card| card.token != token)
  );
  assert_eq!(state.random(), &random);
}
