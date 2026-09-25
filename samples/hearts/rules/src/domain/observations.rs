use serde::Serialize;

use crate::domain::{
  CardId, HandResult, HeartsState, PassDirection, Phase, PlayedCard, Rejection, Seat,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PublicTable {
  pub phase: Phase,
  pub leader: Option<Seat>,
  pub trick: Vec<PlayedCard>,
  pub history: Vec<PlayedCard>,
  pub hand_counts: [usize; 4],
  pub totals: [u16; 4],
  pub result: Option<HandResult>,
  pub hand_index: u32,
  pub pass_direction: PassDirection,
  pub hearts_broken: bool,
  pub voids: [[bool; 4]; 4],
}

/// Cards this observer passed which have not since appeared in public play.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct KnownPass {
  pub recipient: Seat,
  pub cards: Vec<CardId>,
}

/// Owned values containing only information available to the acting seat.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AiObservation {
  pub seat: Seat,
  pub hand: Vec<CardId>,
  pub table: PublicTable,
  pub pending_pass: Option<[CardId; 3]>,
  pub known_pass: Option<KnownPass>,
  pub legal_plays: Vec<CardId>,
}

impl HeartsState {
  pub fn public_table(&self) -> PublicTable {
    PublicTable {
      phase: self.phase,
      leader: if self.phase == Phase::Passing {
        None
      } else {
        Some(self.leader)
      },
      trick: self.trick.clone(),
      history: self.history.clone(),
      hand_counts: std::array::from_fn(|seat| self.hands[seat].len()),
      totals: self.totals,
      result: self.result,
      hand_index: self.hand_index,
      pass_direction: self.pass_direction(),
      hearts_broken: self.hearts_broken,
      voids: self.voids,
    }
  }

  pub fn observe(&self, seat: Seat) -> AiObservation {
    let pending_pass = if self.phase == Phase::Passing {
      self.passes[seat.index()]
    } else {
      None
    };
    let known_pass = if self.phase != Phase::Passing {
      self.passes[seat.index()].map(|cards| KnownPass {
        recipient: self.pass_direction().recipient(seat).unwrap(),
        cards: cards
          .into_iter()
          .filter(|card| !self.history.iter().any(|played| played.card == *card))
          .collect(),
      })
    } else {
      None
    };
    AiObservation {
      seat,
      hand: self.hand(seat).to_vec(),
      table: self.public_table(),
      pending_pass,
      known_pass,
      legal_plays: self
        .play_context(seat)
        .map_or_else(Vec::new, |context| context.legal_plays()),
    }
  }

  pub fn card_rejection(&self, seat: Seat, card: CardId) -> Option<Rejection> {
    match self.play_context(seat) {
      Some(context) => context.validate(card).err(),
      None => Some(Rejection::WrongPhase),
    }
  }
}
