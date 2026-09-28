use battlement::Rect;
use reactant::{
  app_context::{self, Viewport},
  hooks,
};

use crate::{domain::Seat, scene, settings::Preferences};

/// Card height per unit of card width.
const CARD_ASPECT: f64 = 6.000022 / 4.31462;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Layout {
  pub viewport: Viewport,
  pub safe: Rect,
  pub portrait: bool,
  pub large: bool,
  pub footer: f64,
}

/// Width in pixels of a card in the portrait South hand.
pub(crate) fn hand_card_width(large: bool) -> f64 {
  if large { 112.0 } else { 92.0 }
}

/// Card width and spacing in pixels of a portrait opponent fan.
pub(crate) fn opponent_fan(seat: Seat) -> (f64, f64) {
  if seat == Seat::North {
    (48.0, 12.0)
  } else {
    (44.0, 6.0)
  }
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
    self.safe.y + self.safe.height - footer - 28.0 - self::hand_card_width(large) * CARD_ASPECT
  }

  /// Top-center of a portrait seat label.
  pub fn label_origin(self, seat: Seat, large: bool) -> (f64, f64) {
    if seat == Seat::South {
      (
        self.safe.x + self.safe.width / 2.0,
        self.hand_top(large) - 40.0,
      )
    } else {
      let (x, y) = self.opponent_center(seat, large);
      (x, y - if seat == Seat::North { 88.0 } else { 98.0 })
    }
  }

  /// Screen rectangles `[left, top, right, bottom]` covered by the portrait
  /// hands and seat labels.
  pub fn portrait_obstacles(self) -> Vec<[f64; 4]> {
    let top = self.hand_top(self.large);
    // The South hand pans across the full width above the footer; focus and
    // selection together lift its cards by up to 0.4 of their width on screen.
    let size = self.viewport.size;
    let hand = [
      0.0,
      top - self::hand_card_width(self.large) * 0.4,
      f64::from(size.width),
      f64::from(size.height),
    ];
    // A full thirteen-card opponent fan, turned sideways for West and East.
    let opponents = [Seat::North, Seat::West, Seat::East].map(|seat| {
      let (x, y) = self.opponent_center(seat, self.large);
      let (width, spacing) = self::opponent_fan(seat);
      let (along, across) = (width + 12.0 * spacing, width * CARD_ASPECT);
      let (half_width, half_height) = if seat == Seat::North {
        (along / 2.0, across / 2.0)
      } else {
        (across / 2.0, along / 2.0)
      };
      [
        x - half_width,
        y - half_height,
        x + half_width,
        y + half_height,
      ]
    });
    let labels = Seat::ALL.map(|seat| {
      let (x, y) = self.label_origin(seat, self.large);
      [x - 48.0, y, x + 48.0, y + 32.0]
    });
    std::iter::once(hand)
      .chain(opponents)
      .chain(labels)
      .collect()
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
    2.0 * scene::CAMERA_SIZE / f64::from(self.viewport.size.height)
  }

  /// Ground-plane `(x, z)` seen at screen pixel `(x, y)`.
  pub fn ground_point(self, x: f64, y: f64) -> (f64, f64) {
    let size = self.viewport.size;
    let units = self.units_per_pixel();
    (
      (x - f64::from(size.width) / 2.0) * units,
      (f64::from(size.height) / 2.0 - y) * units / scene::CAMERA_TILT.to_radians().sin(),
    )
  }
}

/// Whether the landscape header uses its compact two-by-two button grid.
pub(crate) fn compact_header(layout: Layout, larger_text: bool) -> bool {
  !layout.portrait && !larger_text && layout.safe.width >= 900.0
}
