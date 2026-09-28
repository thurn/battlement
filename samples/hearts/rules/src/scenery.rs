//! Forest-clearing composition around the card table.

use battlement::{MaterialAddress, MaterialAssignment, PrefabAddress, Vector3};
use reactant::world;

use Anchor::{Center, Left, Right};

use crate::assets::hearts::{forest::*, materials};

/// Horizontal reference for a placement, so edge scenery follows the viewport.
#[derive(Clone, Copy)]
enum Anchor {
  Left,
  Center,
  Right,
}

/// Model, anchor, x offset from the anchor, y, z, uniform scale and yaw in degrees.
type Placement = (PrefabAddress, Anchor, f64, f64, f64, f64, f64);

/// Depth below which interior scenery would sit under the raised larger-text hand.
const RAISED_HAND_TOP: f64 = -0.5;

/// Flattened ground ellipse: center x/z and diameters.
type Patch = (f64, f64, f64, f64);

const PROPS: &[Placement] = &[
  (HILL_8X8X2_COLOR1, Left, 1.58, 0.0, 6.25, 0.56, 0.0),
  (BUSH_3_C_COLOR1, Left, 0.48, 0.0, 3.66, 0.42, 0.0),
  (ROCK_1_B_COLOR1, Left, 2.22, 0.0, 2.65, 0.88, 30.0),
  (ROCK_1_E_COLOR1, Left, 1.82, 0.0, 2.47, 0.88, 70.0),
  (TREE_7_C_COLOR1, Left, 1.43, 0.0, 0.73, 0.442, 160.0),
  (TREE_5_B_COLOR1, Left, 0.24, 0.0, -1.1, 0.518, 40.0),
  (ROCK_1_B_COLOR1, Left, 1.43, 0.0, -0.91, 0.825, 110.0),
  (ROCK_1_D_COLOR1, Left, 1.79, 0.0, -1.17, 0.715, 10.0),
  (BUSH_3_C_COLOR1, Left, 0.48, 0.0, -2.29, 0.42, 90.0),
  (TREE_3_C_COLOR1, Left, 1.27, 0.0, -4.11, 0.476, 120.0),
  (BUSH_3_B_COLOR1, Left, 2.22, 0.0, -6.22, 0.42, 0.0),
  (HILL_4X4X4_COLOR1, Left, -0.24, 0.0, -5.3, 0.45, 0.0),
  (GRASS_2_C_COLOR1, Left, 0.63, 0.0, -4.21, 0.7, 30.0),
  (GRASS_1_D_COLOR1, Left, 0.95, 0.0, -0.91, 0.7, 200.0),
  (GRASS_2_B_COLOR1, Left, 4.12, 0.0, 4.57, 0.7, 80.0),
  (HILL_8X8X4_COLOR1, Right, -1.19, 0.0, 5.58, 0.5, 0.0),
  (HILL_4X4X4_COLOR1, Right, -3.8, 0.0, 6.22, 0.42, 0.0),
  (TREE_7_A_COLOR1, Right, -0.63, 3.2, 6.12, 0.255, 80.0),
  (TREE_3_C_COLOR1, Right, -4.43, 2.69, 5.76, 0.289, 10.0),
  (BUSH_3_C_COLOR1, Right, -1.82, 0.0, 0.55, 0.42, 40.0),
  (TREE_7_C_COLOR1, Right, -0.48, 0.0, -1.1, 0.442, 150.0),
  (HILL_4X4X4_COLOR1, Right, -0.55, 0.0, -3.66, 0.45, 0.0),
  (TREE_7_C_COLOR1, Right, -0.55, 2.88, -3.57, 0.289, 120.0),
  (TREE_5_B_COLOR1, Right, -0.32, 0.2, -5.12, 0.567, 10.0),
  (TREE_5_B_COLOR1, Right, -0.63, 0.0, -6.49, 0.542, 200.0),
  (BUSH_3_B_COLOR1, Right, -2.06, 0.0, -5.39, 0.42, 0.0),
  (GRASS_2_A_COLOR1, Right, -1.5, 0.0, -5.3, 0.7, 90.0),
  (ROCK_1_E_COLOR1, Center, -4.18, 0.0, -2.08, 0.88, 40.0),
  (ROCK_1_D_COLOR1, Center, -3.74, 0.0, -2.3, 0.935, 0.0),
  (BUSH_3_B_COLOR1, Center, -4.67, 0.0, -2.56, 0.3, 0.0),
  (GRASS_1_B_COLOR1, Center, -3.4, 0.0, -2.01, 0.7, 30.0),
  (BUSH_3_B_COLOR1, Center, -4.51, 0.0, -2.56, 0.32, 0.0),
  (GRASS_2_B_COLOR1, Center, -4.43, 0.0, -1.01, 0.7, 0.0),
  (ROCK_1_E_COLOR1, Center, 3.77, 0.0, 2.71, 0.374, 160.0),
  (ROCK_2_C_COLOR1, Center, 4.05, 0.0, 2.52, 0.352, 30.0),
  (BUSH_3_B_COLOR1, Center, 3.25, 0.0, -1.46, 0.4, 0.0),
  (GRASS_1_C_COLOR1, Center, 3.4, 0.0, 2.38, 0.7, 0.0),
  (ROCK_2_B_COLOR1, Center, 4.01, 0.0, -2.21, 0.88, 0.0),
  (BUSH_3_B_COLOR1, Center, 4.91, 0.0, -2.56, 0.3, 0.0),
  (GRASS_2_B_COLOR1, Center, 3.64, 0.0, -2.01, 0.7, 60.0),
  (ROCK_1_E_COLOR1, Center, 4.42, 0.0, -2.47, 0.792, 90.0),
  (GRASS_2_C_COLOR1, Center, 3.72, 0.0, -1.74, 0.7, 0.0),
  (BUSH_3_B_COLOR1, Center, -6.21, 0.0, 4.9, 0.4, 0.0),
  (GRASS_1_B_COLOR1, Center, -3.96, 0.0, 3.57, 0.7, 50.0),
  (BUSH_3_C_COLOR1, Left, 1.9, 0.0, 1.1, 0.42, 0.0),
  (BUSH_1_C_COLOR1, Left, 0.13, 0.0, -0.37, 0.5, 30.0),
  (TREE_3_C_COLOR1, Left, 0.4, 0.0, 2.65, 0.34, 60.0),
  (TREE_3_C_COLOR1, Left, 1.5, 0.0, -6.31, 0.34, 250.0),
  (BUSH_3_C_COLOR1, Left, 1.82, 0.0, -6.67, 0.42, 0.0),
  (TREE_7_A_COLOR1, Left, 3.72, 0.0, 5.85, 0.306, 90.0),
  (BUSH_3_C_COLOR1, Left, 4.75, 0.0, 5.58, 0.42, 0.0),
  (BUSH_3_C_COLOR1, Right, -0.16, 0.0, 1.37, 0.42, 0.0),
  (TREE_7_A_COLOR1, Right, -3.17, 0.0, 5.3, 0.306, 0.0),
  (BUSH_3_C_COLOR1, Right, -5.07, 0.0, 5.58, 0.42, 0.0),
  (BUSH_1_C_COLOR1, Right, -0.24, 0.0, -5.48, 0.7, 0.0),
  (TREE_3_C_COLOR1, Right, -0.87, 0.0, -6.86, 0.34, 100.0),
  (GRASS_2_B_COLOR1, Center, -1.98, 0.0, 3.11, 0.7, 70.0),
  (GRASS_2_C_COLOR1, Center, 4.51, 0.0, -0.64, 0.7, 50.0),
  (BUSH_3_C_COLOR1, Left, 0.32, 2.88, -4.94, 0.3, 0.0),
  (GRASS_1_B_COLOR1, Left, 0.48, 2.88, -4.57, 0.7, 0.0),
  (GRASS_2_A_COLOR1, Left, 0.16, 2.88, -5.48, 0.7, 40.0),
  (GRASS_1_C_COLOR1, Right, -1.03, 2.88, -3.11, 0.7, 0.0),
  (GRASS_2_B_COLOR1, Left, 0.32, 1.79, 5.03, 0.7, 0.0),
  (GRASS_1_D_COLOR1, Left, 3.25, 1.79, 5.34, 0.7, 70.0),
  (TREE_5_B_COLOR1, Right, -2.53, 0.0, 4.39, 0.468, 80.0),
  (ROCK_2_B_COLOR1, Left, 0.79, 0.0, 2.01, 0.77, 20.0),
  (ROCK_1_E_COLOR1, Right, -0.63, 0.0, 0.55, 0.77, 50.0),
  (HILL_8X8X4_COLOR1, Right, -0.4, 0.0, 3.29, 0.3, 0.0),
  (GRASS_2_A_COLOR1, Right, -0.48, 1.92, 3.84, 0.7, 0.0),
  (TREE_5_B_COLOR1, Right, -1.58, 0.0, 2.01, 0.518, 0.0),
  (ROCK_1_E_COLOR1, Right, -2.06, 0.0, 1.19, 0.66, 30.0),
  (ROCK_2_B_COLOR1, Right, -1.71, 0.0, 1.01, 0.605, 0.0),
  (GRASS_1_C_COLOR1, Right, -1.27, 0.0, 0.73, 0.7, 0.0),
  (GRASS_1_B_COLOR1, Right, -1.03, 2.56, 2.83, 0.7, 0.0),
];

// Portrait foliage along the side edges; `Clearance` drops whatever would cover
// the seats, hands or labels at the current resolution.
const PORTRAIT_PROPS: &[Placement] = &[
  (BUSH_3_C_COLOR1, Left, 0.09, 0.0, 1.9, 0.42, 0.0),
  (TREE_3_C_COLOR1, Left, 0.2, 0.0, 0.67, 0.26, 40.0),
  (BUSH_1_C_COLOR1, Left, 0.07, 0.0, -0.62, 0.5, 0.0),
  (TREE_5_B_COLOR1, Left, 0.16, 0.0, -1.95, 0.26, 90.0),
  (BUSH_3_B_COLOR1, Left, 0.27, 0.0, -2.73, 0.35, 0.0),
  (ROCK_1_E_COLOR1, Left, 0.53, 0.0, 0.0, 0.35, 20.0),
  (TREE_5_B_COLOR1, Right, -0.11, 0.0, 0.82, 0.26, 10.0),
  (BUSH_3_C_COLOR1, Right, -0.13, 0.0, 0.41, 0.42, 0.0),
  (TREE_3_C_COLOR1, Right, -0.07, 0.0, -0.93, 0.26, 200.0),
  (BUSH_1_C_COLOR1, Right, -0.12, 0.0, -2.21, 0.5, 0.0),
  (ROCK_1_B_COLOR1, Right, -0.46, 0.0, 1.03, 0.4, 0.0),
  (GRASS_2_B_COLOR1, Right, -0.53, 0.0, -0.21, 0.35, 0.0),
  (TREE_7_A_COLOR1, Left, 0.53, 0.0, 6.27, 0.24, 0.0),
  (BUSH_3_C_COLOR1, Left, 1.51, 0.0, 6.38, 0.35, 0.0),
  (TREE_3_C_COLOR1, Right, -0.27, 0.0, 6.43, 0.24, 0.0),
];

const MEADOW: &[Patch] = &[
  (0.0, 0.4, 15.0, 10.6),
  (-6.8, 4.6, 6.0, 3.6),
  (6.8, 4.8, 6.0, 3.6),
  (-6.4, -3.4, 5.5, 4.6),
  (6.4, -3.0, 5.0, 4.6),
  (0.0, -5.4, 11.0, 3.4),
];

const PATH: &[Patch] = &[
  (-0.35, 6.3, 1.05, 1.8),
  (-0.3, 5.0, 0.95, 1.9),
  (-0.2, 3.7, 1.05, 1.9),
  (-0.1, 2.5, 1.15, 1.7),
  (0.0, 0.9, 2.2, 1.4),
  (-0.2, 0.3, 1.6, 1.0),
  (-2.4, 0.9, 2.3, 0.9),
  (-3.7, 1.1, 2.0, 0.8),
  (-4.9, 1.0, 1.8, 0.75),
  (2.4, 0.7, 2.3, 0.9),
  (3.7, 0.9, 2.0, 0.8),
  (4.9, 0.8, 1.8, 0.75),
  (-6.1, 1.0, 1.5, 0.6),
  (6.1, 0.8, 1.5, 0.6),
  (0.1, -1.3, 1.15, 1.6),
  (0.0, -2.6, 1.05, 1.6),
];

const SUNLIT: &[Patch] = &[(0.0, 0.8, 7.0, 4.6), (-0.6, -2.0, 3.6, 2.0)];

const DAPPLE: &[Patch] = &[
  (1.2, 1.6, 1.4, 0.9),
  (-3.3, -1.1, 1.8, 1.0),
  (3.6, -0.4, 1.6, 1.1),
  (-1.6, 3.2, 1.3, 0.8),
  (2.2, 3.6, 1.5, 0.9),
  (-2.2, -2.6, 1.6, 0.8),
  (2.5, -2.4, 1.4, 0.9),
];

const STONES: &[(f64, f64, f64)] = &[
  (-0.55, 6.1, 0.4),
  (-0.6, 4.7, 0.35),
  (0.1, 3.9, 0.45),
  (0.35, 2.75, 0.35),
  (-4.6, 5.0, 0.55),
  (-3.9, 3.9, 0.45),
  (-2.7, 0.8, 0.4),
  (-4.3, 0.75, 0.35),
  (-5.6, 1.1, 0.35),
  (-6.4, 0.85, 0.3),
  (2.9, 0.4, 0.4),
  (4.6, 0.7, 0.35),
  (3.1, 0.95, 0.3),
  (5.6, 0.9, 0.35),
  (6.4, 0.7, 0.3),
  (-0.3, -1.05, 0.5),
  (0.25, -1.4, 0.35),
  (-1.4, -1.6, 0.3),
  (1.5, -1.7, 0.3),
  (2.7, -1.05, 0.45),
];

const TUFTS: &[PrefabAddress] = &[
  GRASS_1_B_COLOR1,
  GRASS_2_A_COLOR1,
  GRASS_1_C_COLOR1,
  GRASS_2_B_COLOR1,
  GRASS_1_D_COLOR1,
  GRASS_2_C_COLOR1,
];

// Height multiplier that lets plateau cliff faces read under the steep camera.
const PLATEAU_LIFT: f64 = 1.6;
// Height multiplier that keeps tuft blades upright under the steep camera.
const TUFT_STRETCH: f64 = 1.8;
// Trees stand taller so trunks survive the steep camera's foreshortening.
const TREE_STRETCH: f64 = 1.35;

/// Ground rectangles `[left, right, near, far]` under portrait cards and labels,
/// which scenery keeps clear.
#[derive(Clone)]
pub(crate) struct Clearance(Vec<[f64; 4]>);

impl Clearance {
  pub(crate) fn new(rects: Vec<[f64; 4]>) -> Self {
    Self(rects)
  }

  // Whether scenery rooted within `reach` of `(x, z)`, whose silhouette rises
  // `rise` depth units up the screen, would cover a cleared rectangle.
  fn covers(&self, x: f64, z: f64, reach: f64, rise: f64) -> bool {
    self.0.iter().any(|&[left, right, near, far]| {
      (left - reach..right + reach).contains(&x) && (near - rise..far + reach).contains(&z)
    })
  }
}

// Deterministic pseudo-random value in `[0, 1)` for scenery scattering.
fn noise(index: usize, salt: f64) -> f64 {
  ((index as f64 * 12.9898 + salt * 78.233).sin() * 43758.5453)
    .fract()
    .abs()
}

// Whether a landscape ground point lies under a card fan, the trick, a pile or a seat label.
fn under_cards(x: f64, z: f64, half_width: f64) -> bool {
  let side = (x.abs() - half_width * 0.55).abs() < 1.9 && (-2.8..4.6).contains(&z);
  let north = x.abs() < 4.8 && z > 3.0;
  let center = x.abs() < 3.9 && (-2.3..3.1).contains(&z);
  let label = (x.abs() - 3.75).abs() < 1.0 && (z - 1.85).abs() < 0.5;
  let interior = center || label || z < -2.0;
  side || north || interior
}

/// Ground treatment: irregular sunlit, shaded and dirt patches over the meadow,
/// with stepping stones and clustered grass tufts.
///
/// Portrait shows about half the landscape width at nearly twice the pixel
/// density, so it narrows the patterns and shrinks the stones to keep them in scale.
pub(crate) fn ground(
  half_width: f64,
  portrait: bool,
  raised_hand: bool,
  clearance: Clearance,
) -> impl reactant::prelude::Render {
  let (narrow, shallow) = if portrait { (0.5, 0.7) } else { (1.0, 1.0) };
  let disc = move |(x, z, width, depth): Patch, y: f64, material: MaterialAddress| {
    world::Cylinder::new()
      .position(Vector3::new(x * narrow, y, z))
      .scale(Vector3::new(width * narrow, 0.001, depth * shallow))
      .materials([MaterialAssignment::new(0, material)])
  };
  // Irregular blotches: each is the union of a few offset opaque discs.
  let blotches = move |patches: Vec<Patch>, salt: f64, y: f64, material: MaterialAddress| {
    patches
      .into_iter()
      .enumerate()
      .flat_map(|(index, (x, z, width, depth))| {
        (0..4).map(move |lobe| {
          let seed = index * 4 + lobe;
          let size = if lobe == 0 {
            1.0
          } else {
            0.45 + 0.3 * self::noise(seed, salt + 5.0)
          };
          let (dx, dz) = if lobe == 0 {
            (0.0, 0.0)
          } else {
            (
              (self::noise(seed, salt + 6.0) - 0.5) * width * 0.9,
              (self::noise(seed, salt + 7.0) - 0.5) * depth * 0.9,
            )
          };
          (x + dx, z + dz, width * size, depth * size)
        })
      })
      .map(|patch| disc(patch, y, material.clone()))
      .collect::<Vec<_>>()
  };
  // Mottles scatter across `reach` of the half width and the given depth band.
  let mottles = |count: usize,
                 salt: f64,
                 (smallest, largest): (f64, f64),
                 reach: f64,
                 (near, far): (f64, f64)| {
    (0..count)
      .map(|index| {
        let width = smallest + (largest - smallest) * self::noise(index, salt + 2.0);
        (
          (self::noise(index, salt) * 2.0 - 1.0) * half_width * reach / narrow,
          near + self::noise(index, salt + 1.0) * (far - near),
          width,
          width * (0.5 + 0.3 * self::noise(index, salt + 3.0)),
        )
      })
      .collect::<Vec<_>>()
  };
  // Tufts cluster in small groups at scattered sites and beside stepping stones.
  let sites = (0..70)
    .map(|index| {
      (
        (self::noise(index, 11.0) * 2.0 - 1.0) * (half_width - 0.8),
        self::noise(index, 12.0) * 11.4 - 5.4,
      )
    })
    .chain(STONES.iter().map(|&(x, z, _)| (x * narrow + 0.45, z - 0.2)));
  let tufts = sites
    .enumerate()
    .flat_map(|(site, (x, z))| {
      (0..2 + site % 2).map(move |blade| {
        let seed = site * 3 + blade;
        let angle = self::noise(seed, 15.0) * std::f64::consts::TAU;
        let reach = 0.15 + 0.25 * self::noise(seed, 16.0);
        (seed, x + angle.cos() * reach, z + angle.sin() * reach)
      })
    })
    .filter_map(|(seed, x, z)| {
      let blocked = if portrait {
        let center = (-2.2..5.2).contains(&z) && x.abs() < 1.2;
        let edge = x.abs() > half_width * 0.72;
        center || edge || clearance.covers(x, z, 0.15, 0.45)
      } else {
        let raised = raised_hand && z < RAISED_HAND_TOP;
        raised || self::under_cards(x, z, half_width)
      };
      let size = (0.65 + 0.4 * self::noise(seed, 13.0)) * shallow * shallow;
      let thinned = portrait && seed % 2 == 1;
      // Tall blades keep tufts upright under the steep camera.
      (!blocked && !thinned).then(|| {
        crate::scene::prop(
          TUFTS[seed % TUFTS.len()].clone(),
          x,
          0.0,
          z,
          size,
          self::noise(seed, 14.0) * 360.0,
        )
        .scale(Vector3::new(size, size * TUFT_STRETCH, size))
      })
    })
    .collect::<Vec<_>>();
  let paths = PATH
    .iter()
    .map(|&(x, z, width, depth)| (x, z, width * 0.96, depth))
    .collect::<Vec<_>>();
  // Each layer has its own parent: sibling placements under one parent are
  // applied in sequence, and a whole-scene relayout must stay within one batch.
  let layer = |children: Vec<world::Cylinder>| world::Group::new().child(children);
  (
    layer(
      MEADOW
        .iter()
        .map(|&patch| disc(patch, 0.004, materials::MEADOW))
        .collect(),
    ),
    layer(blotches(SUNLIT.to_vec(), 1.0, 0.005, materials::SUNLIT)),
    layer(blotches(
      mottles(
        if portrait { 12 } else { 22 },
        3.0,
        (0.8, 2.2),
        if portrait { 0.85 } else { 0.55 },
        (-2.0, 4.5),
      ),
      3.0,
      0.006,
      materials::SUNLIT,
    )),
    layer(blotches(DAPPLE.to_vec(), 5.0, 0.007, materials::SHADE)),
    layer(blotches(
      mottles(
        if portrait { 10 } else { 30 },
        7.0,
        (0.6, 1.9),
        0.9,
        (-5.8, 6.2),
      ),
      7.0,
      0.008,
      materials::SHADE,
    )),
    layer(blotches(paths, 9.0, 0.009, materials::SAND)),
    world::Group::new().child(
      STONES
        .iter()
        .enumerate()
        .filter(move |(_, stone)| !raised_hand || stone.1 > RAISED_HAND_TOP)
        .map(|(index, &(x, z, size))| {
          let size = size * 1.2 * shallow * shallow;
          world::Prefab::at(if index % 2 == 0 {
            ROCK_3_A_COLOR1
          } else {
            ROCK_3_B_COLOR1
          })
          .position(Vector3::new(x * narrow, 0.0, z))
          .scale(Vector3::new(size, size * 0.3, size))
          .rotation(crate::scene::yaw(index as f64 * 67.0))
        })
        .collect::<Vec<_>>(),
    ),
    world::Group::new().child(tufts),
  )
}

/// Trees, bushes, rocks, cliffs and grass framing the clearing.
pub(crate) fn forest(
  half_width: f64,
  portrait: bool,
  raised_hand: bool,
  clearance: &Clearance,
) -> Vec<world::Group> {
  let props = if portrait { PORTRAIT_PROPS } else { PROPS };
  props
    .iter()
    .filter(|(model, anchor, x, _, z, _, _)| {
      let origin = match anchor {
        Left => -half_width,
        Center => 0.0,
        Right => half_width,
      };
      let rise = if model.as_str().starts_with("hearts/forest/tree") {
        1.4
      } else {
        0.5
      };
      // The raised hand also clears the bottom corners, whose trees would shade its ends.
      let interior = *z < RAISED_HAND_TOP && (origin + x).abs() < half_width - 1.0;
      let raised = raised_hand && (interior || *z < -4.0);
      !raised && !clearance.covers(origin + x, *z, 0.35, rise)
    })
    .flat_map(|(model, anchor, x, y, z, scale, yaw)| {
      let origin = match anchor {
        Left => -half_width,
        Center => 0.0,
        Right => half_width,
      };
      let x = origin + x;
      let hill = self::hill_size(model);
      let lift = if hill.is_some() {
        PLATEAU_LIFT
      } else if model.as_str().starts_with("hearts/forest/grass") {
        TUFT_STRETCH
      } else if model.as_str().starts_with("hearts/forest/tree") {
        TREE_STRETCH
      } else {
        1.0
      };
      let prop = crate::scene::prop(model.clone(), x, *y, *z, *scale, *yaw).scale(Vector3::new(
        *scale,
        scale * lift,
        *scale,
      ));
      // Unity's plane is ten units square. The shared terrain tint that keeps cliff
      // faces grey leaves hill tops too bright, so a translucent glaze tones them down
      // while their scalloped rims keep their shape; it sorts before the cards.
      let cap = hill.map(|(width, depth, height)| {
        world::Group::new().sort_order(-1).child(
          world::Plane::new()
            .position(Vector3::new(x, height * scale * lift + 0.01, *z))
            .scale(Vector3::new(
              width * scale / 10.0,
              1.0,
              depth * scale / 10.0,
            ))
            .materials([MaterialAssignment::new(0, materials::PLATEAU)]),
        )
      });
      let sleeve = self::trunk(model).map(|(width, height)| {
        world::Group::new().child(
          world::Cylinder::new()
            .position(Vector3::new(x, *y + height * scale * lift / 2.0, *z))
            .scale(Vector3::new(
              width * scale,
              height * scale * lift / 2.0,
              width * scale,
            ))
            .materials([MaterialAssignment::new(0, materials::TRUNK)]),
        )
      });
      std::iter::once(prop).chain(cap).chain(sleeve)
    })
    .collect()
}

// Trunk width and height at unit scale for trees whose shared tint would dull
// their orange trunks; an orange sleeve restores the trunk color.
fn trunk(model: &PrefabAddress) -> Option<(f64, f64)> {
  let name = model.as_str().strip_prefix("hearts/forest/tree-")?;
  match name.chars().next()? {
    '3' => Some((1.1, 3.0)),
    '5' => Some((0.5, 1.0)),
    '7' => Some((1.2, 3.9)),
    _ => None,
  }
}

// Width, depth and height of a hill block, parsed from its `hill-WxDxH` address.
fn hill_size(model: &PrefabAddress) -> Option<(f64, f64, f64)> {
  let size = model.as_str().strip_prefix("hearts/forest/hill-")?;
  let mut dimensions = size.split('-').next()?.split('x').map(|value| {
    value
      .parse::<f64>()
      .expect("hill addresses encode numeric dimensions")
  });
  Some((dimensions.next()?, dimensions.next()?, dimensions.next()?))
}
