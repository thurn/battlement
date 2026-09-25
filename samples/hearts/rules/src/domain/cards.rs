use serde::{Deserialize, Serialize};

/// Stable domain order, independent of assets or presentation identifiers.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Suit {
  Clubs,
  Diamonds,
  Spades,
  Hearts,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Rank {
  Two,
  Three,
  Four,
  Five,
  Six,
  Seven,
  Eight,
  Nine,
  Ten,
  Jack,
  Queen,
  King,
  Ace,
}

/// A logical card identity; never a public token for a hidden card.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CardId {
  pub suit: Suit,
  pub rank: Rank,
}

/// Clockwise seating order, starting with the human player.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Seat {
  South,
  West,
  North,
  East,
}

/// Actual card ownership; pending passes and history only reference cards.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Zone {
  Hand(Seat),
  Trick,
  Captured(Seat),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PassDirection {
  Left,
  Right,
  Across,
  Hold,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlayedCard {
  pub seat: Seat,
  pub card: CardId,
}

impl Suit {
  pub const ALL: [Self; 4] = [Self::Clubs, Self::Diamonds, Self::Spades, Self::Hearts];

  pub const fn index(self) -> usize {
    match self {
      Self::Clubs => 0,
      Self::Diamonds => 1,
      Self::Spades => 2,
      Self::Hearts => 3,
    }
  }
}

impl Rank {
  pub const ALL: [Self; 13] = [
    Self::Two,
    Self::Three,
    Self::Four,
    Self::Five,
    Self::Six,
    Self::Seven,
    Self::Eight,
    Self::Nine,
    Self::Ten,
    Self::Jack,
    Self::Queen,
    Self::King,
    Self::Ace,
  ];
}

impl CardId {
  pub const TWO_OF_CLUBS: Self = Self {
    suit: Suit::Clubs,
    rank: Rank::Two,
  };
  pub const QUEEN_OF_SPADES: Self = Self {
    suit: Suit::Spades,
    rank: Rank::Queen,
  };

  pub const fn new(suit: Suit, rank: Rank) -> Self {
    Self { suit, rank }
  }

  pub fn penalty_points(self) -> u8 {
    match self {
      Self {
        suit: Suit::Hearts, ..
      } => 1,
      Self::QUEEN_OF_SPADES => 13,
      _ => 0,
    }
  }
}

impl Seat {
  pub const ALL: [Self; 4] = [Self::South, Self::West, Self::North, Self::East];

  pub const fn index(self) -> usize {
    match self {
      Self::South => 0,
      Self::West => 1,
      Self::North => 2,
      Self::East => 3,
    }
  }

  pub fn clockwise(self, places: usize) -> Self {
    Self::ALL[(self.index() + places % 4) % 4]
  }
}

impl PassDirection {
  /// The first hand is index zero.
  pub fn for_hand(hand_index: u32) -> Self {
    [Self::Left, Self::Right, Self::Across, Self::Hold][(hand_index % 4) as usize]
  }

  pub fn recipient(self, sender: Seat) -> Option<Seat> {
    match self {
      Self::Left => Some(sender.clockwise(1)),
      Self::Right => Some(sender.clockwise(3)),
      Self::Across => Some(sender.clockwise(2)),
      Self::Hold => None,
    }
  }
}

pub fn deck() -> Vec<CardId> {
  Suit::ALL
    .into_iter()
    .flat_map(|suit| Rank::ALL.map(|rank| CardId::new(suit, rank)))
    .collect()
}
