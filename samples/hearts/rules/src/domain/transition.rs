use crate::domain::{
  CardId, Event, HeartsState, Intention, Phase, PlayedCard, PresentationSink, Rejection, Seat, Suit,
};
use crate::domain::{choices, deal, scoring};

/// Computes a tentative next state while preserving the caller's accepted state.
pub fn reduce(
  state: &HeartsState,
  intention: Intention,
  sink: &mut impl PresentationSink,
) -> Result<HeartsState, Rejection> {
  let mut next = state.clone();
  apply(&mut next, intention, sink)?;
  Ok(next)
}

/// Validates an external intention without cloning state or producing events.
pub fn validate(state: &HeartsState, intention: &Intention) -> Result<(), Rejection> {
  match intention {
    Intention::SubmitPass { seat, cards } => {
      if state.phase != Phase::Passing {
        return Err(Rejection::WrongPhase);
      }
      if state.passes[seat.index()].is_some() {
        return Err(Rejection::AlreadyPassed);
      }
      choices::validate_pass(state.hand(*seat), state.pass_direction(), cards)
    }
    Intention::PlayCard { seat, card } => state
      .play_context(*seat)
      .ok_or(Rejection::WrongPhase)?
      .validate(*card),
    Intention::NextHand => {
      if state.phase == Phase::HandOver {
        Ok(())
      } else {
        Err(Rejection::WrongPhase)
      }
    }
  }
}

/// Applies one validated intention; rejected intentions leave state and sink untouched.
pub fn apply(
  state: &mut HeartsState,
  intention: Intention,
  sink: &mut impl PresentationSink,
) -> Result<(), Rejection> {
  self::validate(state, &intention)?;
  match intention {
    Intention::SubmitPass { seat, cards } => submit_pass(state, seat, &cards, sink),
    Intention::PlayCard { seat, card } => play_card(state, seat, card, sink),
    Intention::NextHand => {
      let hands = deal::shuffled(&mut state.random.deck);
      *state = HeartsState::from_deal(
        hands,
        state
          .hand_index
          .checked_add(1)
          .expect("hand index exhausted"),
        state.totals,
        state.random.clone(),
      );
      sink.checkpoint(state, Event::Deal);
    }
  }
  Ok(())
}

fn submit_pass(
  state: &mut HeartsState,
  seat: Seat,
  cards: &[CardId],
  sink: &mut impl PresentationSink,
) {
  state.passes[seat.index()] = Some(cards.try_into().unwrap());
  if state.passes.iter().any(Option::is_none) {
    sink.checkpoint(state, Event::PassSubmitted { seat });
    return;
  }
  for sender in Seat::ALL {
    let cards = state.passes[sender.index()].unwrap();
    state.hands[sender.index()].retain(|card| !cards.contains(card));
  }
  for sender in Seat::ALL {
    let recipient = state.pass_direction().recipient(sender).unwrap();
    state.hands[recipient.index()].extend(state.passes[sender.index()].unwrap());
  }
  for hand in &mut state.hands {
    hand.sort_unstable();
  }
  state.leader = Seat::ALL
    .into_iter()
    .find(|&seat| state.hand(seat).contains(&CardId::TWO_OF_CLUBS))
    .unwrap();
  state.phase = Phase::Playing { turn: state.leader };
  sink.checkpoint(state, Event::PassExchanged);
}

fn play_card(state: &mut HeartsState, seat: Seat, card: CardId, sink: &mut impl PresentationSink) {
  if let Some(lead) = state.trick.first()
    && lead.card.suit != card.suit
  {
    state.voids[seat.index()][lead.card.suit.index()] = true;
  }
  state.hands[seat.index()].retain(|held| *held != card);
  let played = PlayedCard { seat, card };
  state.trick.push(played);
  state.history.push(played);
  let broke_hearts = card.suit == Suit::Hearts && !state.hearts_broken;
  state.hearts_broken |= broke_hearts;
  state.phase = Phase::Playing {
    turn: seat.clockwise(1),
  };
  sink.checkpoint(
    state,
    Event::CardPlayed {
      played,
      broke_hearts,
    },
  );
  if state.trick.len() == 4 {
    collect_trick(state, sink);
  }
}

fn collect_trick(state: &mut HeartsState, sink: &mut impl PresentationSink) {
  let cards: [PlayedCard; 4] = state.trick.as_slice().try_into().unwrap();
  let led_suit = cards[0].card.suit;
  let winner = cards
    .iter()
    .filter(|played| played.card.suit == led_suit)
    .max_by_key(|played| played.card.rank)
    .unwrap()
    .seat;
  state.captured[winner.index()].extend(cards.map(|played| played.card));
  state.trick.clear();
  state.leader = winner;
  state.phase = Phase::Playing { turn: winner };
  sink.checkpoint(state, Event::TrickCollected { winner, cards });
  if state.history.len() == 52 {
    let result = scoring::score(&state.captured);
    state.result = Some(result);
    for seat in Seat::ALL {
      state.totals[seat.index()] += result.points[seat.index()];
    }
    state.phase = Phase::HandOver;
    sink.checkpoint(state, Event::HandScored { result });
    if let Some(winners) = scoring::winners(state.totals) {
      state.phase = Phase::MatchOver { winners };
      sink.checkpoint(state, Event::MatchEnded { winners });
    }
  }
}
