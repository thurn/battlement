//! Landscape forest-clearing composition around the card table.

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
  (HILL_8X8X4_COLOR1, Left, 1.11, -0.4, 5.76, 0.45, 0.0),
  (TREE_1_C_COLOR1, Left, 1.9, 1.4, 5.76, 0.36, 20.0),
  (TREE_3_B_COLOR1, Left, 3.4, 0.0, 4.84, 0.36, 140.0),
  (BUSH_3_C_COLOR1, Left, 0.48, 0.0, 3.66, 0.6, 0.0),
  (BUSH_3_B_COLOR1, Left, 1.27, 0.0, 3.38, 0.5, 50.0),
  (ROCK_4_C_COLOR1, Left, 2.22, 0.0, 2.65, 0.64, 30.0),
  (ROCK_1_E_COLOR1, Left, 1.82, 0.0, 2.47, 0.64, 70.0),
  (TREE_1_C_COLOR1, Left, 1.27, 0.0, 0.73, 0.42, 200.0),
  (TREE_4_C_COLOR1, Left, 0.24, 0.0, -1.1, 0.42, 40.0),
  (ROCK_4_C_COLOR1, Left, 1.43, 0.0, -2.01, 0.72, 110.0),
  (ROCK_1_D_COLOR1, Left, 1.98, 0.0, -2.38, 0.64, 10.0),
  (BUSH_3_C_COLOR1, Left, 0.48, 0.0, -2.29, 0.7, 90.0),
  (TREE_3_C_COLOR1, Left, 1.5, 0.0, -4.02, 0.4, 300.0),
  (BUSH_3_C_COLOR1, Left, 0.79, 0.0, -5.67, 0.7, 20.0),
  (BUSH_3_B_COLOR1, Left, 2.22, 0.0, -6.22, 0.6, 0.0),
  (HILL_4X4X4_COLOR1, Left, -0.32, -0.4, -6.31, 0.45, 0.0),
  (GRASS_2_C_COLOR1, Left, 0.63, 0.0, -4.21, 0.7, 30.0),
  (GRASS_1_D_COLOR1, Left, 0.95, 0.0, -0.91, 0.7, 200.0),
  (GRASS_2_B_COLOR1, Left, 4.12, 0.0, 4.57, 0.7, 80.0),
  (HILL_8X8X4_COLOR1, Right, -1.19, -0.4, 5.58, 0.5, 0.0),
  (HILL_4X4X4_COLOR1, Right, -3.8, -0.4, 6.22, 0.42, 0.0),
  (TREE_1_C_COLOR1, Right, -1.03, 1.6, 5.21, 0.36, 80.0),
  (TREE_3_B_COLOR1, Right, -4.43, 1.3, 5.76, 0.34, 10.0),
  (TREE_4_C_COLOR1, Right, -1.03, 0.0, 2.1, 0.44, 300.0),
  (ROCK_4_C_COLOR1, Right, -2.3, 0.0, 3.38, 0.64, 200.0),
  (ROCK_1_E_COLOR1, Right, -1.9, 0.0, 3.2, 0.64, 20.0),
  (BUSH_3_B_COLOR1, Right, -1.66, 0.0, 4.3, 0.5, 0.0),
  (BUSH_3_C_COLOR1, Right, -3.33, 0.0, 1.74, 0.6, 40.0),
  (BUSH_1_C_COLOR1, Right, -3.88, 0.0, 2.47, 0.9, 0.0),
  (TREE_1_C_COLOR1, Right, -0.55, 0.0, -0.27, 0.42, 150.0),
  (HILL_4X4X4_COLOR1, Right, 0.16, -0.4, -3.2, 0.45, 0.0),
  (TREE_4_C_COLOR1, Right, -1.19, 0.0, -4.11, 0.42, 120.0),
  (TREE_4_B_COLOR1, Right, -0.87, 0.2, -4.57, 0.5, 10.0),
  (TREE_4_C_COLOR1, Right, -0.63, 0.0, -6.49, 0.44, 200.0),
  (ROCK_1_D_COLOR1, Right, -1.43, 0.0, -2.93, 0.64, 60.0),
  (ROCK_4_B_COLOR1, Right, -1.03, 0.0, -3.11, 0.64, 0.0),
  (BUSH_3_C_COLOR1, Right, -0.16, 0.0, -1.46, 0.6, 0.0),
  (BUSH_3_B_COLOR1, Right, -2.06, 0.0, -5.39, 0.6, 0.0),
  (GRASS_2_A_COLOR1, Right, -1.5, 0.0, -5.3, 0.7, 90.0),
  (ROCK_1_E_COLOR1, Center, -4.35, 0.0, -1.92, 0.48, 40.0),
  (ROCK_2_C_COLOR1, Center, -3.96, 0.0, -2.16, 0.56, 0.0),
  (BUSH_3_B_COLOR1, Center, -4.83, 0.0, -2.65, 0.45, 0.0),
  (GRASS_2_B_COLOR1, Center, -4.43, 0.0, -1.01, 0.7, 0.0),
  (ROCK_1_E_COLOR1, Center, 3.96, 0.0, 3.66, 0.48, 160.0),
  (ROCK_2_C_COLOR1, Center, 4.28, 0.0, 3.47, 0.48, 30.0),
  (BUSH_3_B_COLOR1, Center, 3.48, 0.0, 2.01, 0.45, 0.0),
  (GRASS_1_C_COLOR1, Center, 3.4, 0.0, 2.38, 0.7, 0.0),
  (ROCK_1_D_COLOR1, Center, 4.28, 0.0, -2.01, 0.48, 0.0),
  (ROCK_2_C_COLOR1, Center, 4.67, 0.0, -2.19, 0.4, 90.0),
  (GRASS_2_C_COLOR1, Center, 3.72, 0.0, -1.74, 0.7, 0.0),
  (BUSH_3_B_COLOR1, Center, -4.91, 0.0, 4.84, 0.45, 0.0),
  (GRASS_1_B_COLOR1, Center, -3.96, 0.0, 3.57, 0.7, 50.0),
  (BUSH_3_C_COLOR1, Left, 1.9, 0.0, 1.1, 0.7, 0.0),
  (BUSH_1_C_COLOR1, Left, 0.32, 0.0, 0.46, 1.0, 30.0),
  (TREE_3_B_COLOR1, Left, 0.4, 0.0, 2.65, 0.4, 60.0),
  (BUSH_3_B_COLOR1, Left, 1.82, 0.0, -0.46, 0.55, 0.0),
  (BUSH_1_C_COLOR1, Left, 2.22, 0.0, -3.38, 1.0, 0.0),
  (TREE_3_B_COLOR1, Left, 0.4, 0.0, -5.3, 0.4, 250.0),
  (BUSH_3_C_COLOR1, Left, 1.82, 0.0, -6.67, 0.7, 0.0),
  (TREE_1_B_COLOR1, Left, 3.72, 0.0, 5.85, 0.36, 90.0),
  (BUSH_3_C_COLOR1, Left, 4.75, 0.0, 5.58, 0.6, 0.0),
  (BUSH_1_C_COLOR1, Right, -1.66, 0.0, 1.1, 1.0, 0.0),
  (BUSH_3_C_COLOR1, Right, -0.16, 0.0, 1.37, 0.7, 0.0),
  (TREE_3_B_COLOR1, Right, -0.4, 0.0, -2.01, 0.4, 30.0),
  (BUSH_3_B_COLOR1, Right, -1.5, 0.0, -0.64, 0.55, 0.0),
  (TREE_1_B_COLOR1, Right, -3.17, 0.0, 5.3, 0.36, 0.0),
  (BUSH_3_C_COLOR1, Right, -5.07, 0.0, 5.58, 0.6, 0.0),
  (BUSH_1_C_COLOR1, Right, -0.24, 0.0, -5.48, 1.0, 0.0),
  (TREE_3_B_COLOR1, Right, -0.87, 0.0, -6.86, 0.4, 100.0),
  (GRASS_2_B_COLOR1, Center, -1.98, 0.0, 3.11, 0.7, 70.0),
  (GRASS_2_C_COLOR1, Center, 4.51, 0.0, -0.64, 0.7, 50.0),
];

const MEADOW: &[Patch] = &[
  (0.0, 0.6, 15.0, 11.5),
  (-6.8, 4.6, 6.0, 3.6),
  (6.8, 4.8, 6.0, 3.6),
  (-6.4, -3.4, 5.5, 4.6),
  (6.4, -3.0, 5.0, 4.6),
  (0.0, -5.4, 11.0, 3.4),
];

const PATH: &[Patch] = &[
  (-0.35, 6.3, 0.8, 1.8),
  (-0.3, 5.0, 0.7, 1.9),
  (-0.2, 3.7, 0.8, 1.9),
  (-0.1, 2.5, 0.9, 1.7),
  (0.0, 0.9, 2.6, 1.7),
  (-0.2, 0.3, 1.8, 1.2),
  (-2.3, 0.9, 1.9, 0.7),
  (-3.6, 1.1, 1.6, 0.6),
  (-4.8, 1.0, 1.4, 0.55),
  (2.3, 0.7, 1.9, 0.7),
  (3.6, 0.9, 1.6, 0.6),
  (4.8, 0.8, 1.4, 0.55),
  (0.1, -1.3, 0.9, 1.6),
  (0.0, -2.6, 0.8, 1.6),
];

const SUNLIT: &[Patch] = &[
  (0.0, 0.8, 7.5, 5.0),
  (-2.6, 3.4, 3.2, 2.0),
  (2.8, 3.1, 3.4, 2.0),
  (-0.6, -2.0, 4.0, 2.2),
  (-4.0, 5.2, 3.0, 1.6),
];

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
  (-4.6, 5.0, 0.55),
  (-3.9, 3.9, 0.45),
  (-0.9, 5.6, 0.4),
  (0.4, 4.4, 0.45),
  (-2.6, 0.5, 0.4),
  (-0.3, -1.05, 0.5),
  (0.2, -1.35, 0.35),
  (2.7, -1.05, 0.45),
  (3.4, 1.4, 0.4),
];

/// Ground treatment: lighter meadow over the base grass, dirt paths and flat stones.
///
/// Portrait shows about half the landscape width at nearly twice the pixel
/// density, so it narrows the patterns and shrinks the stones to keep them in scale.
pub(crate) fn ground(portrait: bool, raised_hand: bool) -> impl reactant::prelude::Render {
  let (narrow, shallow) = if portrait { (0.5, 0.7) } else { (1.0, 1.0) };
  let layer = move |patches: &'static [Patch], y: f64, material: MaterialAddress| {
    patches
      .iter()
      .map(move |&(x, z, width, depth)| {
        world::Cylinder::new()
          .position(Vector3::new(x * narrow, y, z))
          .scale(Vector3::new(width * narrow, 0.002, depth * shallow))
          .materials([MaterialAssignment::new(0, material.clone())])
      })
      .collect::<Vec<_>>()
  };
  (
    layer(MEADOW, 0.004, materials::MEADOW),
    layer(SUNLIT, 0.005, materials::SUNLIT),
    layer(DAPPLE, 0.006, materials::CLEARING),
    layer(PATH, 0.008, materials::SAND),
    STONES
      .iter()
      .enumerate()
      .filter(move |(_, stone)| !raised_hand || stone.1 > RAISED_HAND_TOP)
      .map(|(index, &(x, z, size))| {
        let size = size * 1.3 * shallow;
        world::Prefab::at(if index % 2 == 0 {
          ROCK_3_A_COLOR1
        } else {
          ROCK_3_B_COLOR1
        })
        .position(Vector3::new(x * narrow, 0.0, z))
        .scale(Vector3::new(size, size * 0.3, size))
        .rotation(super::scene::yaw(index as f64 * 67.0))
      })
      .collect::<Vec<_>>(),
  )
}

/// Trees, bushes, rocks, cliffs and grass framing the clearing.
pub(crate) fn forest(half_width: f64, raised_hand: bool) -> Vec<world::Group> {
  PROPS
    .iter()
    .filter_map(|(model, anchor, x, y, z, scale, yaw)| {
      let origin = match anchor {
        Left => -half_width,
        Center => 0.0,
        Right => half_width,
      };
      let x = origin + x;
      let under_hand = *z < RAISED_HAND_TOP && x.abs() < half_width - 1.0;
      (!raised_hand || !under_hand)
        .then(|| super::scene::prop(model.clone(), x, *y, *z, *scale, *yaw))
    })
    .collect()
}
