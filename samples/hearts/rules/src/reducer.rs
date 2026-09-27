use reactant_rules::{GameReducer, ReducerOutput};

use crate::ai::decision::Decision;
use crate::domain::{Event, HeartsState, Intention, PresentationSink, Rejection, transition};

/// Runs the pure Hearts transition function through bounded reducer publications.
pub struct HeartsReducer;

#[derive(Clone, Debug)]
pub enum HeartsAction {
  Human(Intention),
  Computer(Box<Decision>),
}

struct Output<'a>(&'a mut ReducerOutput<HeartsReducer>);

impl GameReducer for HeartsReducer {
  type State = HeartsState;
  type Action = HeartsAction;
  type Event = Event;
  type Rejection = Rejection;

  fn validate(state: &HeartsState, action: &HeartsAction) -> Result<(), Rejection> {
    let intention = match action {
      HeartsAction::Human(intention) => intention,
      HeartsAction::Computer(decision) => {
        let seat = decision.observation.seat;
        if state.observe(seat) != decision.observation
          || state.random().ai[seat.index()] != decision.previous_stream
        {
          return Err(Rejection::StaleDecision);
        }
        let actor = match &decision.intention {
          Intention::SubmitPass { seat, .. } | Intention::PlayCard { seat, .. } => Some(*seat),
          Intention::NextHand => None,
        };
        if actor != Some(seat) {
          return Err(Rejection::NotYourTurn);
        }
        &decision.intention
      }
    };
    transition::validate(state, intention)
  }

  fn reduce(
    &mut self,
    state: &mut HeartsState,
    action: HeartsAction,
    output: &mut ReducerOutput<Self>,
  ) {
    let intention = match action {
      HeartsAction::Human(intention) => intention,
      HeartsAction::Computer(decision) => {
        state.random.ai[decision.observation.seat.index()] = decision.next_stream;
        decision.intention
      }
    };
    transition::apply(state, intention, &mut Output(output)).expect("validated Hearts intention");
  }
}

impl PresentationSink for Output<'_> {
  fn checkpoint(&mut self, state: &HeartsState, event: Event) {
    self.0.publish(state, || event);
  }
}
