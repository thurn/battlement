use battlement_reactant::{executor::Spawner, runtime::Reactant};

pub fn reactant<G>(spawner: impl Spawner) -> Reactant<G> {
  Reactant::new(spawner)
}
