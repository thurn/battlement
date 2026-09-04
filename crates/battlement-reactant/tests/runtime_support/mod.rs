use battlement_reactant::{executor::Spawner, runtime::Reactant};
use trox::Bundle;

pub fn source_bundle() -> Bundle {
  Bundle::from_canonical_json(include_str!(
    "../../../../samples/reactant/localization/en-US.trox.json"
  ))
  .expect("valid embedded test source bundle")
}

pub fn reactant<G>(spawner: impl Spawner) -> Reactant<G> {
  let mut reactant = Reactant::new(spawner);
  reactant.set_source_bundle(source_bundle());
  reactant
}
