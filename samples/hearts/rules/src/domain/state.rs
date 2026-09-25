use serde::{Deserialize, Serialize};

use crate::domain::{
  CardId, HandResult, Hands, PassDirection, PlayContext, PlayedCard, RandomStreams, Seat, Zone,
};
use crate::domain::{cards, deal};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Phase {
  Passing,
  Playing { turn: Seat },
  HandOver,
  MatchOver { winners: [bool; 4] },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Intention {
  SubmitPass { seat: Seat, cards: Vec<CardId> },
  PlayCard { seat: Seat, card: CardId },
  NextHand,
}

/// Authoritative private domain state. Views, never this value, feed opponents.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HeartsState {
  pub(crate) hands: Hands,
  pub(crate) passes: [Option<[CardId; 3]>; 4],
  pub(crate) trick: Vec<PlayedCard>,
  pub(crate) captured: Hands,
  pub(crate) history: Vec<PlayedCard>,
  pub(crate) voids: [[bool; 4]; 4],
  pub(crate) leader: Seat,
  pub(crate) hearts_broken: bool,
  pub(crate) hand_index: u32,
  pub(crate) totals: [u16; 4],
  pub(crate) phase: Phase,
  pub(crate) result: Option<HandResult>,
  pub(crate) random: RandomStreams,
}

impl HeartsState {
  pub fn new(seed: u64) -> Self {
    let mut random = RandomStreams::from_seed(seed);
    Self::from_deal(deal::shuffled(&mut random.deck), 0, [0; 4], random)
  }

  /// Constructs a complete explicit deal; malformed internal fixtures panic.
  pub fn from_deal(
    mut hands: Hands,
    hand_index: u32,
    totals: [u16; 4],
    random: RandomStreams,
  ) -> Self {
    assert!(
      hands.iter().all(|hand| hand.len() == 13),
      "each hand must contain thirteen cards"
    );
    let mut all: Vec<_> = hands.iter().flatten().copied().collect();
    all.sort_unstable();
    assert_eq!(
      all,
      cards::deck(),
      "a deal must contain each card exactly once"
    );
    assert!(
      totals.iter().all(|&score| score < 100),
      "a finished match cannot deal another hand"
    );
    for hand in &mut hands {
      hand.sort_unstable();
    }
    let leader = Seat::ALL
      .into_iter()
      .find(|seat| hands[seat.index()].contains(&CardId::TWO_OF_CLUBS))
      .unwrap();
    Self {
      hands,
      passes: [None; 4],
      trick: Vec::with_capacity(4),
      captured: std::array::from_fn(|_| Vec::new()),
      history: Vec::with_capacity(52),
      voids: [[false; 4]; 4],
      leader,
      hearts_broken: false,
      hand_index,
      totals,
      phase: if PassDirection::for_hand(hand_index) == PassDirection::Hold {
        Phase::Playing { turn: leader }
      } else {
        Phase::Passing
      },
      result: None,
      random,
    }
  }

  pub fn phase(&self) -> Phase {
    self.phase
  }
  pub fn hand(&self, seat: Seat) -> &[CardId] {
    &self.hands[seat.index()]
  }
  pub fn trick(&self) -> &[PlayedCard] {
    &self.trick
  }
  pub fn captured(&self, seat: Seat) -> &[CardId] {
    &self.captured[seat.index()]
  }
  pub fn random(&self) -> &RandomStreams {
    &self.random
  }
  pub fn pass_direction(&self) -> PassDirection {
    PassDirection::for_hand(self.hand_index)
  }

  pub fn zone(&self, card: CardId) -> Zone {
    for seat in Seat::ALL {
      if self.hand(seat).contains(&card) {
        return Zone::Hand(seat);
      }
      if self.captured(seat).contains(&card) {
        return Zone::Captured(seat);
      }
    }
    assert!(
      self.trick.iter().any(|played| played.card == card),
      "card has no ownership zone"
    );
    Zone::Trick
  }

  /// Complete-trick presentation snapshots offer no further card action.
  pub fn play_context(&self, seat: Seat) -> Option<PlayContext<'_>> {
    let Phase::Playing { turn } = self.phase else {
      return None;
    };
    if self.trick.len() == 4 {
      return None;
    }
    Some(PlayContext {
      hand: self.hand(seat),
      seat,
      turn,
      trick: &self.trick,
      first_trick: self.history.len() < 4,
      hearts_broken: self.hearts_broken,
    })
  }
}
