use crate::domain::{HandResult, HeartsState, PlayedCard, Seat};

/// Receives borrowed checkpoints; simulation need not clone display snapshots.
pub trait PresentationSink {
  fn checkpoint(&mut self, state: &HeartsState, event: Event);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
  Deal,
  PassSubmitted {
    seat: Seat,
  },
  PassExchanged,
  CardPlayed {
    played: PlayedCard,
    broke_hearts: bool,
  },
  TrickCollected {
    winner: Seat,
    cards: [PlayedCard; 4],
  },
  HandScored {
    result: HandResult,
  },
  MatchEnded {
    winners: [bool; 4],
  },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkpoint {
  pub state: HeartsState,
  pub event: Event,
}

pub struct IgnorePresentation;

impl PresentationSink for IgnorePresentation {
  fn checkpoint(&mut self, _: &HeartsState, _: Event) {}
}

impl PresentationSink for Vec<Checkpoint> {
  fn checkpoint(&mut self, state: &HeartsState, event: Event) {
    self.push(Checkpoint {
      state: state.clone(),
      event,
    });
  }
}
