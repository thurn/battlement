use std::collections::BTreeMap;

use fastrand::Rng;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::cards;
use crate::domain::observations::KnownPass;
use crate::domain::{CardId, HeartsState, PublicTable, Rejection, Seat};

/// Opaque identity allocated independently of game and AI randomness.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CardToken(Uuid);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct VisibleCard {
  pub token: CardToken,
  pub face: Option<CardId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HumanView {
  pub seat: Seat,
  pub table: PublicTable,
  pub hands: [Vec<VisibleCard>; 4],
  pub trick: Vec<(Seat, VisibleCard)>,
  pub captured: [Vec<VisibleCard>; 4],
  pub legal_plays: Vec<CardToken>,
  pub rejections: Vec<(CardToken, Rejection)>,
  pub pending_pass: Option<[CardToken; 3]>,
  pub known_pass: Option<KnownPass>,
}

/// Private mapping for one mounted session; construct fresh after restore.
pub struct Projection {
  tokens: BTreeMap<CardId, CardToken>,
  order: BTreeMap<CardToken, usize>,
}

/// Optional reproducible visual ordering, independent of opaque IDs and game randomness.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct PresentationSeed(pub u64);

impl Default for Projection {
  fn default() -> Self {
    Self::with_order_seed(fastrand::u64(..))
  }
}

impl CardToken {
  pub fn id(self) -> Uuid {
    self.0
  }
}

impl Projection {
  /// Reproduces hidden-card positions without making their presentation IDs predictable.
  pub fn with_order_seed(seed: u64) -> Self {
    let tokens: BTreeMap<_, _> = cards::deck()
      .into_iter()
      .map(|card| (card, CardToken(Uuid::new_v4())))
      .collect();
    let mut ordered = cards::deck();
    Rng::with_seed(seed).shuffle(&mut ordered);
    let order = ordered
      .into_iter()
      .enumerate()
      .map(|(index, card)| (tokens[&card], index))
      .collect();
    Self { tokens, order }
  }

  pub fn view(&self, state: &HeartsState, seat: Seat) -> HumanView {
    let observation = state.observe(seat);
    let hands = std::array::from_fn(|index| {
      let owner = Seat::ALL[index];
      let mut cards: Vec<_> = state
        .hand(owner)
        .iter()
        .map(|&card| self.card(card, owner == seat))
        .collect();
      if owner != seat {
        cards.sort_unstable_by_key(|card| self.order[&card.token]);
      }
      cards
    });
    HumanView {
      seat,
      table: observation.table,
      hands,
      trick: state
        .trick()
        .iter()
        .map(|played| (played.seat, self.card(played.card, true)))
        .collect(),
      captured: std::array::from_fn(|index| {
        state
          .captured(Seat::ALL[index])
          .iter()
          .map(|&card| self.card(card, true))
          .collect()
      }),
      legal_plays: observation
        .legal_plays
        .into_iter()
        .map(|card| self.tokens[&card])
        .collect(),
      rejections: state
        .hand(seat)
        .iter()
        .filter_map(|&card| {
          state
            .card_rejection(seat, card)
            .map(|reason| (self.tokens[&card], reason))
        })
        .collect(),
      pending_pass: observation
        .pending_pass
        .map(|cards| cards.map(|card| self.tokens[&card])),
      known_pass: observation.known_pass,
    }
  }

  /// Hidden or absent cards cannot resolve into this seat's input intentions.
  pub fn resolve_owned(&self, state: &HeartsState, seat: Seat, token: CardToken) -> Option<CardId> {
    state
      .hand(seat)
      .iter()
      .copied()
      .find(|card| self.tokens[card] == token)
  }

  fn card(&self, card: CardId, visible: bool) -> VisibleCard {
    VisibleCard {
      token: self.tokens[&card],
      face: visible.then_some(card),
    }
  }
}
