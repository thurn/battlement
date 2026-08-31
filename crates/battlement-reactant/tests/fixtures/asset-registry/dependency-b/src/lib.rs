//! Second linked-registration fixture dependency.

use battlement_reactant::asset_generator;

asset_generator::generate! {
  @nine-slice DEPENDENCY_B {
    @canvas 24px 12px;
    @slices 2px 3px 2px 3px;
    border: 1px dashed red;
  }
}

/// Returns the generated fixture asset.
pub fn asset() -> battlement_reactant::asset_generator::NineSliceAsset {
  DEPENDENCY_B
}
