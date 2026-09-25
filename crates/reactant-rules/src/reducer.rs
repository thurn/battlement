use std::{convert::Infallible, marker::PhantomData};

use crate::{DisplayConnection, Game};

/// Prompt-free rules with typed validation and lazily published checkpoints.
pub trait GameReducer: Sized + Send + 'static {
  /// Owned state; cloning must not share mutable logical data.
  type State: Clone + Send + 'static;
  /// One external intention.
  type Action: Send + 'static;
  /// Semantic presentation data for one checkpoint.
  type Event: Send + 'static;
  /// A recoverable invalid intention, returned before starting a worker.
  type Rejection;

  /// Validates without mutation; called only against the accepted state.
  fn validate(state: &Self::State, action: &Self::Action) -> Result<(), Self::Rejection>;

  /// Executes a validated intention against private state.
  fn reduce(
    &mut self,
    state: &mut Self::State,
    action: Self::Action,
    output: &mut ReducerOutput<Self>,
  );
}

/// Adapts a reducer to the existing worker, publication and acceptance path.
pub struct ReducerGame<R: GameReducer>(PhantomData<R>);

/// Session-owned reducer and its bounded publication connection.
pub struct ReducerContext<R: GameReducer> {
  reducer: R,
  output: ReducerOutput<R>,
}

/// Builds snapshots and events only after reserving an existing publication slot.
pub struct ReducerOutput<R: GameReducer> {
  connection: DisplayConnection<ReducerGame<R>>,
}

impl<R: GameReducer> ReducerContext<R> {
  /// Connects a reducer without requiring a prompt enum or choice policy.
  pub fn new(reducer: R, connection: DisplayConnection<ReducerGame<R>>) -> Self {
    Self {
      reducer,
      output: ReducerOutput { connection },
    }
  }
}

impl<R: GameReducer> ReducerOutput<R> {
  /// Reserves capacity before cloning state or invoking the event builder.
  pub fn publish(&mut self, state: &R::State, event: impl FnOnce() -> R::Event) {
    self.connection.present(state, event);
  }
}

impl<R: GameReducer> Game for ReducerGame<R> {
  type State = R::State;
  type Action = R::Action;
  type StateAnimation = R::Event;
  type Prompt<'a> = Infallible;
  type Context = ReducerContext<R>;

  fn logical_clone(state: &Self::State) -> Self::State {
    state.clone()
  }

  fn is_legal_action(state: &Self::State, action: &Self::Action) -> bool {
    R::validate(state, action).is_ok()
  }

  fn execute(context: &mut Self::Context, state: &mut Self::State, action: Self::Action) {
    context.reducer.reduce(state, action, &mut context.output);
  }
}
