use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::domain::{CardId, PassDirection, PlayedCard, Seat, Suit};

/// Borrowed authoritative facts needed to validate one player's intention.
#[derive(Clone, Copy, Debug)]
pub struct PlayContext<'a> {
  pub hand: &'a [CardId],
  pub seat: Seat,
  pub turn: Seat,
  pub trick: &'a [PlayedCard],
  pub first_trick: bool,
  pub hearts_broken: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Rejection {
  NotYourTurn,
  CardNotHeld,
  MustOpenTwoClubs,
  MustFollowSuit,
  HeartsNotBroken,
  FirstTrickPenalty,
  NoPassing,
  PassCount,
  DuplicatePassCard,
  WrongPhase,
  AlreadyPassed,
}

impl Display for Rejection {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
    formatter.write_str(match self {
      Self::NotYourTurn => "Wait for your turn.",
      Self::CardNotHeld => "Choose a card in your hand.",
      Self::MustOpenTwoClubs => "Open the first trick with the two of clubs.",
      Self::MustFollowSuit => "Follow the led suit while you have it.",
      Self::HeartsNotBroken => "Hearts cannot lead until broken, unless you hold only hearts.",
      Self::FirstTrickPenalty => "Play a non-penalty card on the first trick if you can.",
      Self::NoPassing => "Keep your cards on this hand; there is no pass.",
      Self::PassCount => "Choose exactly three cards to pass.",
      Self::DuplicatePassCard => "Choose three different cards to pass.",
      Self::WrongPhase => "That action is not available in this phase.",
      Self::AlreadyPassed => "Your pass is already submitted.",
    })
  }
}

impl std::error::Error for Rejection {}

impl PlayContext<'_> {
  pub fn legal_plays(&self) -> Vec<CardId> {
    self
      .hand
      .iter()
      .copied()
      .filter(|&card| self.validate(card).is_ok())
      .collect()
  }

  /// Rejects an external intention without mutating any rules or random state.
  pub fn validate(&self, card: CardId) -> Result<(), Rejection> {
    assert!(
      self.trick.len() < 4,
      "collect the completed trick before requesting a play"
    );
    if self.seat != self.turn {
      return Err(Rejection::NotYourTurn);
    }
    if !self.hand.contains(&card) {
      return Err(Rejection::CardNotHeld);
    }
    if self.first_trick && self.trick.is_empty() {
      return if card == CardId::TWO_OF_CLUBS {
        Ok(())
      } else {
        Err(Rejection::MustOpenTwoClubs)
      };
    }
    if !self.follows_suit(card) {
      return Err(Rejection::MustFollowSuit);
    }
    if self.trick.is_empty() && !self.hearts_broken {
      let only_hearts = self.hand.iter().all(|held| held.suit == Suit::Hearts);
      if card.suit == Suit::Hearts && !only_hearts {
        return Err(Rejection::HeartsNotBroken);
      }
    }
    if self.first_trick && card.penalty_points() > 0 {
      let non_penalty = self
        .hand
        .iter()
        .any(|&held| held.penalty_points() == 0 && self.follows_suit(held));
      if non_penalty {
        return Err(Rejection::FirstTrickPenalty);
      }
    }
    Ok(())
  }

  fn follows_suit(&self, card: CardId) -> bool {
    let Some(lead) = self.trick.first() else {
      return true;
    };
    card.suit == lead.card.suit || !self.hand.iter().any(|held| held.suit == lead.card.suit)
  }
}

pub fn pass_choices(hand: &[CardId], direction: PassDirection) -> &[CardId] {
  if direction == PassDirection::Hold {
    &[]
  } else {
    hand
  }
}

/// Checks three references against the unchanged hand without transferring ownership.
pub fn validate_pass(
  hand: &[CardId],
  direction: PassDirection,
  cards: &[CardId],
) -> Result<(), Rejection> {
  if direction == PassDirection::Hold {
    return Err(Rejection::NoPassing);
  }
  if cards.len() != 3 {
    return Err(Rejection::PassCount);
  }
  for (index, card) in cards.iter().enumerate() {
    if cards[..index].contains(card) {
      return Err(Rejection::DuplicatePassCard);
    }
    if !hand.contains(card) {
      return Err(Rejection::CardNotHeld);
    }
  }
  Ok(())
}
