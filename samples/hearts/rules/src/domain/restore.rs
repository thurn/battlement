use crate::domain::{
  HeartsState, IgnorePresentation, Intention, PassDirection, Phase, Seat, cards, transition,
};

// Each hand adds at least 26 points; every pre-hand total is below 100.
const MAX_HAND_INDEX: u32 = (4 * 99) / 26;

impl HeartsState {
  /// Checks a saved accepted state by reconstructing its hand through the rules.
  pub fn validate_saved(&self) -> Result<(), &'static str> {
    if self.history.len() > 52 || self.hand_index > MAX_HAND_INDEX {
      return Err("saved hand exceeds game bounds");
    }
    let mut hands = self.hands.clone();
    for played in &self.history {
      hands[played.seat.index()].push(played.card);
    }
    if hands.iter().any(|hand| hand.len() != 13) {
      return Err("saved hand has inconsistent card counts");
    }
    let mut all: Vec<_> = hands.iter().flatten().copied().collect();
    all.sort_unstable();
    if all != cards::deck() {
      return Err("saved hand must own every card exactly once");
    }
    let mut totals = self.totals;
    if let Some(result) = self.result {
      for (total, points) in totals.iter_mut().zip(result.points) {
        *total = total
          .checked_sub(points)
          .ok_or("saved scores are inconsistent")?;
      }
    }
    if totals.iter().any(|&total| total >= 100) {
      return Err("saved hand started after the match ended");
    }
    if self.phase != Phase::Passing && self.pass_direction() != PassDirection::Hold {
      for sender in Seat::ALL {
        let pass = self.passes[sender.index()].ok_or("saved exchange is incomplete")?;
        let recipient = self.pass_direction().recipient(sender).unwrap();
        let hand = &mut hands[recipient.index()];
        let distinct = pass[0] != pass[1] && pass[0] != pass[2] && pass[1] != pass[2];
        if !distinct || pass.iter().any(|card| !hand.contains(card)) {
          return Err("saved pass does not belong to its recipient");
        }
        hand.retain(|card| !pass.contains(card));
      }
      for sender in Seat::ALL {
        hands[sender.index()].extend(self.passes[sender.index()].unwrap());
      }
    }
    let mut rebuilt = Self::from_deal(hands, self.hand_index, totals, self.random.clone());
    for seat in Seat::ALL {
      if let Some(cards) = self.passes[seat.index()] {
        transition::apply(
          &mut rebuilt,
          Intention::SubmitPass {
            seat,
            cards: cards.to_vec(),
          },
          &mut IgnorePresentation,
        )
        .map_err(|_| "saved pass is illegal")?;
      }
    }
    for played in &self.history {
      transition::apply(
        &mut rebuilt,
        Intention::PlayCard {
          seat: played.seat,
          card: played.card,
        },
        &mut IgnorePresentation,
      )
      .map_err(|_| "saved play history is illegal")?;
    }
    if rebuilt != *self {
      return Err("saved match disagrees with its legal history");
    }
    Ok(())
  }
}
