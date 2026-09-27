use std::cmp::Reverse;

use crate::domain::{AiObservation, CardId, RandomStream, Rank, Suit};

/// Keeps eight dangerous three-card combinations, with canonical tie ordering.
pub fn passes(observation: &AiObservation) -> Vec<Vec<CardId>> {
  let mut hand = observation.hand.clone();
  hand.sort_unstable();
  let mut choices = Vec::new();
  for a in 0..hand.len() {
    for b in a + 1..hand.len() {
      for c in b + 1..hand.len() {
        choices.push([hand[a], hand[b], hand[c]]);
      }
    }
  }
  choices.sort_by_key(|cards| {
    (
      Reverse(cards.iter().map(|card| self::danger(*card)).sum::<u16>()),
      *cards,
    )
  });
  choices.truncate(8);
  choices.into_iter().map(|cards| cards.to_vec()).collect()
}

/// Uses only this actor's legal cards and public trick; randomness breaks equal scores.
pub fn play(observation: &AiObservation, stream: &mut RandomStream) -> CardId {
  let mut choices = observation.legal_plays.clone();
  choices.sort_unstable();
  let led = observation
    .table
    .trick
    .first()
    .map(|played| played.card.suit);
  let highest = led.and_then(|suit| {
    observation
      .table
      .trick
      .iter()
      .filter(|played| played.card.suit == suit)
      .map(|played| played.card.rank)
      .max()
  });
  let follows = led.is_some_and(|suit| choices.iter().any(|card| card.suit == suit));
  let can_lose = follows && highest.is_some_and(|rank| choices.iter().any(|card| card.rank < rank));
  let score = |card: CardId| -> i32 {
    if led.is_some() && !follows {
      -i32::from(self::danger(card))
    } else if can_lose {
      if card.rank < highest.unwrap() {
        -i32::from(card.rank as u8)
      } else {
        1000
      }
    } else {
      i32::from(self::danger(card))
    }
  };
  let best = choices
    .iter()
    .map(|card| score(*card))
    .min()
    .expect("actor has legal plays");
  choices.retain(|card| score(*card) == best);
  choices[stream.below(choices.len() as u64) as usize]
}

fn danger(card: CardId) -> u16 {
  let rank = u16::from(card.rank as u8);
  match (card.suit, card.rank) {
    (Suit::Spades, Rank::Queen) => 100,
    (Suit::Spades, Rank::King | Rank::Ace) => 60 + rank,
    (Suit::Hearts, _) => 30 + rank,
    _ => rank,
  }
}
