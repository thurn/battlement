//! First linked-registration fixture dependency.

use reactant_core::asset_generator;

asset_generator::generate! {
  @background DEPENDENCY_A {
    @canvas 20px 10px;
    background: linear-gradient(red, blue);
  }
}

/// Returns the generated fixture asset.
pub fn asset() -> reactant_core::asset_generator::BackgroundAsset {
  DEPENDENCY_A
}
