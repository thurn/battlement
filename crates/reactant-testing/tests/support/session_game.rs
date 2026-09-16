use std::{
  borrow::Cow,
  sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use reactant_rules::{
  ChoiceOwner, ChoicePolicy, DisplayConnection, ExecutionMode, Game, PromptData,
};

pub struct Counter;
#[derive(Clone)]
pub struct State {
  pub value: usize,
  pub probe: Arc<Probe>,
}
#[derive(Default)]
pub struct Probe {
  pub contexts: AtomicUsize,
  pub clones: AtomicUsize,
  pub validations: AtomicUsize,
  pub executions: AtomicUsize,
  pub drops: AtomicUsize,
  gate: Mutex<(bool, bool)>,
  changed: Condvar,
}
pub struct Context {
  execution: ExecutionMode<Counter, Policy>,
  turns: usize,
  probe: Arc<Probe>,
}
#[derive(Clone, Copy)]
pub enum Action {
  Add,
  Choice,
  Illegal,
  Fail,
  HoldThenFail,
}
pub enum Prompt<'a> {
  Number(Cow<'a, Number>),
}
#[derive(Clone)]
pub struct Number;
pub struct Policy;

impl Probe {
  pub fn state(self: &Arc<Self>) -> State {
    State {
      value: 0,
      probe: self.clone(),
    }
  }
  pub fn wait_held(&self) {
    let (gate, timeout) = self
      .changed
      .wait_timeout_while(self.gate.lock().unwrap(), Duration::from_secs(10), |g| !g.0)
      .unwrap();
    assert!(!timeout.timed_out() && gate.0);
  }
  pub fn release(&self) {
    self.gate.lock().unwrap().1 = true;
    self.changed.notify_all();
  }
}
impl Context {
  pub fn interactive(connection: DisplayConnection<Counter>, probe: Arc<Probe>) -> Self {
    probe.contexts.fetch_add(1, Ordering::SeqCst);
    Self {
      execution: ExecutionMode::Interactive {
        connection,
        policy: Policy,
      },
      turns: 0,
      probe,
    }
  }
  pub fn simulation(probe: Arc<Probe>) -> Self {
    Self {
      execution: ExecutionMode::Simulation { policy: Policy },
      turns: 0,
      probe,
    }
  }
}
impl Drop for Context {
  fn drop(&mut self) {
    self.probe.drops.fetch_add(1, Ordering::SeqCst);
  }
}
impl PromptData<Counter> for Number {
  type ResponseType = usize;
  fn options(&self) -> impl Iterator<Item = usize> {
    [2, 5, 9].into_iter()
  }
  fn is_valid_response(&self, response: &usize) -> bool {
    [2, 5, 9].contains(response)
  }
  fn as_prompt(&self) -> Prompt<'_> {
    Prompt::Number(Cow::Borrowed(self))
  }
  fn into_prompt(self) -> Prompt<'static> {
    Prompt::Number(Cow::Owned(self))
  }
}
impl ChoicePolicy<Counter> for Policy {
  fn owner(&self, _: &State, _: &Prompt<'_>) -> ChoiceOwner {
    ChoiceOwner::Human
  }
  fn choose(&mut self, _: &State, _: &Prompt<'_>) -> usize {
    1
  }
}
impl Game for Counter {
  type State = State;
  type Action = Action;
  type StateAnimation = ();
  type Prompt<'a> = Prompt<'a>;
  type Context = Context;
  fn logical_clone(state: &State) -> State {
    state.probe.clones.fetch_add(1, Ordering::SeqCst);
    state.clone()
  }
  fn is_legal_action(state: &State, action: &Action) -> bool {
    state.probe.validations.fetch_add(1, Ordering::SeqCst);
    !matches!(action, Action::Illegal)
  }
  fn execute(cx: &mut Context, state: &mut State, action: Action) {
    state.probe.executions.fetch_add(1, Ordering::SeqCst);
    cx.turns += 1;
    match action {
      Action::Add => state.value += cx.turns,
      Action::Choice => state.value += cx.execution.choose(state, Number),
      Action::Fail => {
        state.value = 999;
        panic!("deliberate worker failure");
      }
      Action::HoldThenFail => {
        let mut gate = state.probe.gate.lock().unwrap();
        gate.0 = true;
        state.probe.changed.notify_all();
        while !gate.1 {
          gate = state.probe.changed.wait(gate).unwrap();
        }
        drop(gate);
        panic!("abandoned worker failure");
      }
      Action::Illegal => panic!("illegal action reached rules"),
    }
  }
}
