//! Native and WebAssembly linked-registration fixture.

use battlement_reactant::asset_generator;

/// Returns the number of unique registrations linked from both dependencies.
#[unsafe(no_mangle)]
pub extern "C" fn registration_count() -> u32 {
  self::link_dependencies();
  u32::try_from(asset_generator::registrations().count()).expect("fixture registration overflow")
}

/// Returns an order-independent FNV-1a hash of the linked addresses.
#[unsafe(no_mangle)]
pub extern "C" fn registration_address_hash() -> u32 {
  self::link_dependencies();
  let mut addresses = asset_generator::registrations()
    .map(|value| value.address)
    .collect::<Vec<_>>();
  addresses.sort_unstable();
  addresses.into_iter().fold(2_166_136_261, |hash, address| {
    address
      .as_bytes()
      .iter()
      .fold(hash, |hash, byte| (hash ^ u32::from(*byte)).wrapping_mul(16_777_619))
  })
}

fn link_dependencies() {
  let _ = dependency_a::asset();
  let _ = dependency_b::asset();
}
