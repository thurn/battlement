use serde::{Deserialize, Serialize};

use crate::domain::{Hands, Seat};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HandResult {
  pub points: [u16; 4],
  pub moon: Option<Seat>,
}

pub fn score(captured: &Hands) -> HandResult {
  let points: [u16; 4] = std::array::from_fn(|seat| {
    captured[seat]
      .iter()
      .map(|card| u16::from(card.penalty_points()))
      .sum()
  });
  assert_eq!(
    points.iter().sum::<u16>(),
    26,
    "a scored hand must contain all penalties"
  );
  let moon = Seat::ALL
    .into_iter()
    .find(|seat| points[seat.index()] == 26);
  HandResult {
    points: if let Some(shooter) = moon {
      std::array::from_fn(|seat| if seat == shooter.index() { 0 } else { 26 })
    } else {
      points
    },
    moon,
  }
}

pub fn winners(totals: [u16; 4]) -> Option<[bool; 4]> {
  if totals.iter().all(|&score| score < 100) {
    return None;
  }
  let lowest = *totals.iter().min().unwrap();
  Some(totals.map(|score| score == lowest))
}
