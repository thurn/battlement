use reactant_rules::{GameReducer, ReducerOutput};

use crate::domain::{Event, HeartsState, Intention, PresentationSink, Rejection, transition};

/// Runs the pure Hearts transition function through bounded reducer publications.
pub struct HeartsReducer;

struct Output<'a>(&'a mut ReducerOutput<HeartsReducer>);

impl GameReducer for HeartsReducer {
  type State = HeartsState;
  type Action = Intention;
  type Event = Event;
  type Rejection = Rejection;

  fn validate(state: &HeartsState, action: &Intention) -> Result<(), Rejection> {
    transition::validate(state, action)
  }

  fn reduce(
    &mut self,
    state: &mut HeartsState,
    action: Intention,
    output: &mut ReducerOutput<Self>,
  ) {
    transition::apply(state, action, &mut Output(output)).expect("validated Hearts intention");
  }
}

impl PresentationSink for Output<'_> {
  fn checkpoint(&mut self, state: &HeartsState, event: Event) {
    self.0.publish(state, || event);
  }
}
