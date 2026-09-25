use fastrand::Rng;
use serde::{Deserialize, Serialize};

use crate::domain::cards;
use crate::domain::{CardId, Seat};

pub type Hands = [Vec<CardId>; 4];

/// Resumable fastrand 2.3.0 state; all bounded draws use u64 on every platform.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RandomStream {
  state: u64,
}

/// Independent deck and per-seat decision streams, excluding presentation RNG.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RandomStreams {
  pub deck: RandomStream,
  pub ai: [RandomStream; 4],
}

impl RandomStream {
  pub fn from_seed(seed: u64) -> Self {
    Self { state: seed }
  }

  pub fn below(&mut self, upper: u64) -> u64 {
    assert!(upper > 0, "random bound must be positive");
    let mut rng = Rng::with_seed(self.state);
    let value = rng.u64(0..upper);
    self.state = rng.get_seed();
    value
  }
}

impl RandomStreams {
  pub fn from_seed(seed: u64) -> Self {
    let mut rng = Rng::with_seed(seed);
    Self {
      deck: RandomStream::from_seed(rng.u64(..)),
      ai: std::array::from_fn(|_| RandomStream::from_seed(rng.u64(..))),
    }
  }
}

/// Fisher–Yates followed by clockwise dealing; hands use stable card order.
pub fn shuffled(rng: &mut RandomStream) -> Hands {
  let mut deck = cards::deck();
  for last in (1..deck.len()).rev() {
    let selected = rng.below((last + 1) as u64) as usize;
    deck.swap(last, selected);
  }
  let mut hands: Hands = std::array::from_fn(|_| Vec::with_capacity(13));
  for (index, card) in deck.into_iter().enumerate() {
    hands[index % Seat::ALL.len()].push(card);
  }
  for hand in &mut hands {
    hand.sort_unstable();
  }
  hands
}
