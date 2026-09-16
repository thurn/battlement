use std::{
  sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use reactant_rules::{ChoiceOwner, ChoicePolicy, DisplayConnection, ExecutionMode, Game};
use reactant_testing::PublicationDisplay;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HeldBuilder {
  None,
  Snapshot,
  Animation,
}

pub struct Probe {
  pub snapshots: AtomicUsize,
  pub animations: AtomicUsize,
  pub context_drops: AtomicUsize,
  hold: HeldBuilder,
  gate: Mutex<(bool, bool)>,
  changed: Condvar,
}

pub struct State {
  pub values: Vec<usize>,
  // Instrumentation is shared; logical game data above is always independent.
  pub probe: Arc<Probe>,
}

pub enum Action {
  Publish(usize),
  FailAfterPublication,
}

pub struct PublicationGame;
pub struct Policy;
pub struct Context {
  execution: ExecutionMode<PublicationGame, Policy>,
  probe: Arc<Probe>,
}

impl Probe {
  pub fn new(hold: HeldBuilder) -> Arc<Self> {
    Arc::new(Self {
      snapshots: AtomicUsize::new(0),
      animations: AtomicUsize::new(0),
      context_drops: AtomicUsize::new(0),
      hold,
      gate: Mutex::new((false, false)),
      changed: Condvar::new(),
    })
  }

  pub fn enter(&self, builder: HeldBuilder) {
    if self.hold != builder {
      return;
    }
    let mut gate = self.gate.lock().unwrap();
    gate.0 = true;
    self.changed.notify_all();
    while !gate.1 {
      gate = self.changed.wait(gate).unwrap();
    }
  }

  pub fn wait_for_builder(&self) {
    let gate = self.gate.lock().unwrap();
    let (gate, result) = self
      .changed
      .wait_timeout_while(gate, Duration::from_secs(5), |gate| !gate.0)
      .unwrap();
    assert!(!result.timed_out() && gate.0, "builder did not enter");
  }

  pub fn release(&self) {
    self.gate.lock().unwrap().1 = true;
    self.changed.notify_all();
  }
}

impl Drop for Context {
  fn drop(&mut self) {
    self.probe.context_drops.fetch_add(1, Ordering::SeqCst);
  }
}

impl Game for PublicationGame {
  type State = State;
  type Action = Action;
  type StateAnimation = usize;
  type Prompt<'a> = ();
  type Context = Context;

  fn logical_clone(state: &State) -> State {
    state.probe.snapshots.fetch_add(1, Ordering::SeqCst);
    if state.values[0] == 1 {
      state.probe.enter(HeldBuilder::Snapshot);
    }
    State {
      values: state.values.clone(),
      probe: Arc::clone(&state.probe),
    }
  }

  fn is_legal_action(_: &State, _: &Action) -> bool {
    true
  }

  fn execute(context: &mut Context, state: &mut State, action: Action) {
    let count = match action {
      Action::Publish(count) => count,
      Action::FailAfterPublication => 1,
    };
    for value in 1..=count {
      state.values[0] = value;
      context.execution.present(state, || {
        state.probe.animations.fetch_add(1, Ordering::SeqCst);
        if value == 1 {
          state.probe.enter(HeldBuilder::Animation);
        }
        value
      });
    }
    if matches!(action, Action::FailAfterPublication) {
      panic!("rules failed after publication");
    }
    state.values[0] = 999;
  }
}

impl ChoicePolicy<PublicationGame> for Policy {
  fn owner(&self, _: &State, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }
  fn choose(&mut self, _: &State, _: &()) -> usize {
    panic!("choice-free fixture")
  }
}

pub fn start(state: &State, count: usize) -> PublicationDisplay<PublicationGame> {
  self::start_action(state, Action::Publish(count))
}

pub fn start_action(state: &State, action: Action) -> PublicationDisplay<PublicationGame> {
  let probe = Arc::clone(&state.probe);
  PublicationDisplay::start(
    state,
    action,
    |connection: DisplayConnection<PublicationGame>| Context {
      execution: ExecutionMode::Interactive {
        connection,
        policy: Policy,
      },
      probe,
    },
  )
}
