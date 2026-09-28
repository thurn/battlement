use reactant::hooks;

use crate::{
  layout::{self, Layout},
  projection::{CardToken, VisibleCard},
};

#[derive(Clone, Copy, Default, PartialEq)]
struct Position {
  offset: f64,
  focused: Option<CardToken>,
}

#[derive(Clone, PartialEq)]
pub(crate) struct HandPan {
  pub layout: Layout,
  pub width: f64,
  pub spacing: f64,
  pub offset: f64,
  pub maximum: f64,
  cards: Vec<CardToken>,
  set_position: hooks::StateSetter<Position>,
}

pub(crate) fn use_hand_pan(cards: &[VisibleCard]) -> HandPan {
  let layout = layout::use_layout();
  let width = layout::hand_card_width(layout.large);
  let spacing = if layout.large { 50.0 } else { 42.0 };
  let available = (layout.safe.width - 36.0).max(width);
  let maximum = (width + spacing * cards.len().saturating_sub(1) as f64 - available).max(0.0);
  let (position, set_position) = hooks::use_state(Position::default());
  let cards: Vec<_> = cards.iter().map(|card| card.token).collect();
  let mut pan = HandPan {
    layout,
    width,
    spacing,
    offset: position.offset.clamp(0.0, maximum),
    maximum,
    cards,
    set_position: set_position.clone(),
  };
  let focused = position.focused.filter(|token| pan.cards.contains(token));
  if let Some(token) = focused {
    pan.offset = pan.revealed(token);
  }
  let clean = Position {
    offset: pan.offset,
    focused,
  };
  hooks::use_effect(
    move || {
      if position != clean {
        set_position.set(clean);
      }
    },
    clean,
  );
  pan
}

impl HandPan {
  pub fn enabled(&self) -> bool {
    self.layout.portrait && self.maximum > 0.0
  }

  pub fn pan_to(&self, offset: f64) {
    self.set_position.set(Position {
      offset: offset.clamp(0.0, self.maximum),
      focused: None,
    });
  }

  pub fn focus(&self, token: CardToken) {
    self.set_position.set(Position {
      offset: self.revealed(token),
      focused: Some(token),
    });
  }

  pub fn world_center(&self) -> f64 {
    let center = self.layout.safe.x + self.layout.safe.width / 2.0;
    let center = center - f64::from(self.layout.viewport.size.width) / 2.0;
    (center + self.maximum / 2.0 - self.offset) * self.layout.units_per_pixel()
  }

  fn revealed(&self, token: CardToken) -> f64 {
    let Some(index) = self.cards.iter().position(|card| *card == token) else {
      return self.offset;
    };
    let left = index as f64 * self.spacing;
    let right = left + self.width;
    let available = (self.layout.safe.width - 36.0).max(self.width);
    self
      .offset
      .min(left)
      .max(right - available)
      .clamp(0.0, self.maximum)
  }
}
