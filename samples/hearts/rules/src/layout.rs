use battlement::Rect;
use reactant::{
  app_context::{self, Viewport},
  hooks,
};

use crate::{domain::Seat, settings::Preferences};

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Layout {
  pub viewport: Viewport,
  pub safe: Rect,
  pub portrait: bool,
  pub large: bool,
  pub footer: f64,
}

pub(crate) fn use_layout() -> Layout {
  let viewport = app_context::use_viewport();
  let large = hooks::use_context::<Preferences>().larger_text;
  let safe = viewport.safe_area;
  let portrait = safe.width < safe.height;
  let footer = if portrait {
    if large { 210.0 } else { 168.0 }
  } else if large {
    142.0
  } else {
    52.0
  };
  Layout {
    viewport,
    safe,
    portrait,
    large,
    footer,
  }
}

impl Layout {
  pub fn hand_top(self, large: bool) -> f64 {
    let footer = if large { 210.0 } else { 168.0 };
    self.safe.y + self.safe.height
      - footer
      - 28.0
      - if large { 112.0 } else { 92.0 } * (6.000022 / 4.31462)
  }

  pub fn opponent_center(self, seat: Seat, large: bool) -> (f64, f64) {
    match seat {
      Seat::North => (
        self.safe.x + self.safe.width / 2.0,
        (self.safe.y + 240.0).min(self.hand_top(large) - 125.0),
      ),
      Seat::West => (
        self.safe.x + 48.0,
        (self.safe.y + 335.0).min(self.hand_top(large) - 60.0),
      ),
      Seat::East => (
        self.safe.x + self.safe.width - 48.0,
        (self.safe.y + 335.0).min(self.hand_top(large) - 60.0),
      ),
      Seat::South => unreachable!("human fan has its own position"),
    }
  }

  pub fn right(self) -> f64 {
    f64::from(self.viewport.size.width) - self.safe.x - self.safe.width
  }

  pub fn units_per_pixel(self) -> f64 {
    11.4 / f64::from(self.viewport.size.height)
  }
}
