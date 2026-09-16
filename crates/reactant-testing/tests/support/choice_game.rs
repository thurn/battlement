use std::{
  borrow::Cow,
  sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use reactant_rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game, PromptData};
use reactant_testing::PublicationDisplay;

pub struct Choices;
pub struct Policy {
  pub owner: ChoiceOwner,
  pub probe: Arc<Probe>,
}
pub struct Probe {
  pub clones: AtomicUsize,
  pub calls: AtomicUsize,
  pub hold_policy: bool,
  gate: Mutex<(bool, bool)>,
  changed: Condvar,
}
#[derive(Clone)]
pub struct State {
  pub answers: Vec<usize>,
  pub probe: Arc<Probe>,
}
pub enum Action {
  Human,
  Policies,
  Empty,
}
pub enum Prompt<'a> {
  Number(Cow<'a, Number>),
  Pair(Cow<'a, Pair>),
  Empty(Cow<'a, Empty>),
}
pub struct Number {
  pub choices: [usize; 3],
  pub probe: Arc<Probe>,
}
#[derive(Clone)]
pub struct Pair(pub [[usize; 2]; 2]);
#[derive(Clone)]
pub struct Empty;

impl Probe {
  pub fn new(hold_policy: bool) -> Arc<Self> {
    Arc::new(Self {
      clones: AtomicUsize::new(0),
      calls: AtomicUsize::new(0),
      hold_policy,
      gate: Mutex::new((false, false)),
      changed: Condvar::new(),
    })
  }
  pub fn wait_policy(&self) {
    let (gate, timeout) = self
      .changed
      .wait_timeout_while(self.gate.lock().unwrap(), Duration::from_secs(5), |g| !g.0)
      .unwrap();
    assert!(!timeout.timed_out() && gate.0);
  }
  pub fn release(&self) {
    self.gate.lock().unwrap().1 = true;
    self.changed.notify_all();
  }
}
impl Clone for Number {
  fn clone(&self) -> Self {
    self.probe.clones.fetch_add(1, Ordering::SeqCst);
    Self {
      choices: self.choices,
      probe: Arc::clone(&self.probe),
    }
  }
}
impl PromptData<Choices> for Number {
  type ResponseType = usize;
  fn options(&self) -> impl Iterator<Item = usize> {
    self.choices.into_iter()
  }
  fn is_valid_response(&self, response: &usize) -> bool {
    self.choices.contains(response)
  }
  fn as_prompt(&self) -> Prompt<'_> {
    Prompt::Number(Cow::Borrowed(self))
  }
  fn into_prompt(self) -> Prompt<'static> {
    Prompt::Number(Cow::Owned(self))
  }
}
impl PromptData<Choices> for Pair {
  type ResponseType = [usize; 2];
  fn options(&self) -> impl Iterator<Item = Self::ResponseType> {
    self.0.into_iter()
  }
  fn is_valid_response(&self, response: &Self::ResponseType) -> bool {
    self.0.contains(response)
  }
  fn as_prompt(&self) -> Prompt<'_> {
    Prompt::Pair(Cow::Borrowed(self))
  }
  fn into_prompt(self) -> Prompt<'static> {
    Prompt::Pair(Cow::Owned(self))
  }
}
impl PromptData<Choices> for Empty {
  type ResponseType = ();
  fn options(&self) -> impl Iterator<Item = ()> {
    [()].into_iter()
  }
  fn is_valid_response(&self, _: &()) -> bool {
    true
  }
  fn as_prompt(&self) -> Prompt<'_> {
    Prompt::Empty(Cow::Borrowed(self))
  }
  fn into_prompt(self) -> Prompt<'static> {
    Prompt::Empty(Cow::Owned(self))
  }
}
impl ChoicePolicy<Choices> for Policy {
  fn owner(&self, _: &State, _: &Prompt<'_>) -> ChoiceOwner {
    self.owner
  }
  fn choose(&mut self, _: &State, prompt: &Prompt<'_>) -> usize {
    assert!(matches!(prompt, Prompt::Number(Cow::Borrowed(_))));
    self.probe.calls.fetch_add(1, Ordering::SeqCst);
    if self.probe.hold_policy {
      let mut gate = self.probe.gate.lock().unwrap();
      gate.0 = true;
      self.probe.changed.notify_all();
      while !gate.1 {
        gate = self.probe.changed.wait(gate).unwrap();
      }
    }
    1
  }
}
impl Game for Choices {
  type State = State;
  type Action = Action;
  type StateAnimation = ();
  type Prompt<'a> = Prompt<'a>;
  type Context = ExecutionMode<Self, Policy>;
  fn logical_clone(state: &State) -> State {
    state.clone()
  }
  fn is_legal_action(_: &State, _: &Action) -> bool {
    true
  }
  fn execute(cx: &mut Self::Context, state: &mut State, action: Action) {
    match action {
      Action::Human => {
        let number = self::number(cx, state);
        state.answers.push(number);
        let pair = cx.choose(state, Pair([[2, 4], [6, 8]]));
        state.answers.extend(pair);
      }
      Action::Policies => {
        for _ in 0..5 {
          let number = self::number(cx, state);
          state.answers.push(number);
        }
      }
      Action::Empty => {
        cx.choose(state, Empty);
        state.answers.push(1);
      }
    }
  }
}
fn number(cx: &mut ExecutionMode<Choices, Policy>, state: &State) -> usize {
  cx.choose(
    state,
    Number {
      choices: [3, 7, 11],
      probe: Arc::clone(&state.probe),
    },
  )
}
pub fn start(
  probe: &Arc<Probe>,
  action: Action,
  owner: ChoiceOwner,
) -> PublicationDisplay<Choices> {
  let state = State {
    answers: Vec::new(),
    probe: Arc::clone(probe),
  };
  PublicationDisplay::start(&state, action, |connection| ExecutionMode::Interactive {
    connection,
    policy: Policy {
      owner,
      probe: Arc::clone(probe),
    },
  })
}
