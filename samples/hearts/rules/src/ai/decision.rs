use crate::domain::{AiObservation, Intention, RandomStream};

/// A detached result whose input and random state are checked at final acceptance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decision {
  pub observation: AiObservation,
  pub previous_stream: RandomStream,
  pub next_stream: RandomStream,
  pub intention: Intention,
}
