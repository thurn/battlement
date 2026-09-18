use crate::world_layout::{
  ArcLayout, FanLayout, FlexDirection, FlexLayout, GridLayout, LayoutAlignment, PileLayout,
  WorldLayout,
};

/// Taffy Flexbox world layout.
pub type Flex = WorldLayout<FlexLayout>;
/// Taffy Grid world layout.
pub type Grid = WorldLayout<GridLayout>;
/// Pure fan world layout.
pub type Fan = WorldLayout<FanLayout>;
/// Pure pile world layout.
pub type Pile = WorldLayout<PileLayout>;
/// Pure arc world layout.
pub type Arc = WorldLayout<ArcLayout>;

macro_rules! constructor {
  ($algorithm:ty) => {
    impl WorldLayout<$algorithm> {
      /// Creates a world layout with default parameters.
      #[must_use]
      pub fn new() -> Self {
        Self::default()
      }

      /// Replaces the pure algorithm parameters.
      #[must_use]
      pub fn algorithm(mut self, algorithm: $algorithm) -> Self {
        self.algorithm = algorithm;
        self
      }
    }
  };
}

constructor!(FlexLayout);
constructor!(GridLayout);
constructor!(FanLayout);
constructor!(PileLayout);
constructor!(ArcLayout);

impl Flex {
  /// Sets horizontal or vertical flow.
  #[must_use]
  pub fn direction(mut self, direction: FlexDirection) -> Self {
    self.algorithm.direction = direction;
    self
  }

  /// Sets the world-unit gap between adjacent children.
  #[must_use]
  pub fn gap(mut self, gap: f64) -> Self {
    self.algorithm.gap = gap;
    self
  }

  /// Sets main-axis distribution.
  #[must_use]
  pub fn justify(mut self, justify: LayoutAlignment) -> Self {
    self.algorithm.justify = justify;
    self
  }

  /// Sets cross-axis alignment.
  #[must_use]
  pub fn align(mut self, align: LayoutAlignment) -> Self {
    self.algorithm.align = align;
    self
  }
}

impl Grid {
  /// Sets the positive number of equal-width columns.
  #[must_use]
  pub fn columns(mut self, columns: u16) -> Self {
    self.algorithm.columns = columns;
    self
  }

  /// Sets horizontal and vertical world-unit gaps.
  #[must_use]
  pub fn gaps(mut self, column_gap: f64, row_gap: f64) -> Self {
    self.algorithm.column_gap = column_gap;
    self.algorithm.row_gap = row_gap;
    self
  }

  /// Sets alignment within grid cells.
  #[must_use]
  pub fn align(mut self, align: LayoutAlignment) -> Self {
    self.algorithm.align = align;
    self
  }
}

impl Fan {
  /// Sets total horizontal spread and center rise.
  #[must_use]
  pub fn curve(mut self, spread: f64, rise: f64) -> Self {
    self.algorithm.spread = spread;
    self.algorithm.rise = rise;
    self
  }

  /// Sets total in-plane angle spread in radians.
  #[must_use]
  pub fn angle(mut self, angle: f64) -> Self {
    self.algorithm.angle = angle;
    self
  }
}

impl Pile {
  /// Sets the per-child world-unit offset.
  #[must_use]
  pub fn step(mut self, x: f64, y: f64) -> Self {
    self.algorithm.step_x = x;
    self.algorithm.step_y = y;
    self
  }
}

impl Arc {
  /// Sets the first and last angles in radians.
  #[must_use]
  pub fn angles(mut self, start: f64, end: f64) -> Self {
    self.algorithm.start_angle = start;
    self.algorithm.end_angle = end;
    self
  }

  /// Sets the horizontal and vertical radii.
  #[must_use]
  pub fn radii(mut self, x: f64, y: f64) -> Self {
    self.algorithm.radius_x = x;
    self.algorithm.radius_y = y;
    self
  }
}
