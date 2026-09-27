use std::borrow::Cow;

use reactant_rules::{
  ChoiceOwner, ChoicePolicy, DisplayConnection, ExecutionMode, Game, PromptData,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CounterState {
  value: u8,
}

struct CounterGame;

struct CounterContext {
  execution: ExecutionMode<CounterGame, CounterPolicy>,
}

#[derive(Clone)]
enum CounterPrompt<'a> {
  Number(Cow<'a, NumberPrompt>),
  Pair(Cow<'a, PairPrompt>),
}

#[derive(Clone, Copy)]
struct NumberPrompt {
  choices: [u8; 3],
}

impl PromptData<CounterGame> for NumberPrompt {
  type ResponseType = u8;

  fn options(&self) -> impl Iterator<Item = Self::ResponseType> {
    self.choices.into_iter()
  }

  fn is_valid_response(&self, response: &Self::ResponseType) -> bool {
    self.choices.contains(response)
  }

  fn as_prompt(&self) -> CounterPrompt<'_> {
    CounterPrompt::Number(Cow::Borrowed(self))
  }

  fn into_prompt(self) -> CounterPrompt<'static> {
    CounterPrompt::Number(Cow::Owned(self))
  }
}

#[derive(Clone, Copy)]
struct PairPrompt {
  choices: [[u8; 2]; 2],
}

impl PromptData<CounterGame> for PairPrompt {
  type ResponseType = [u8; 2];

  fn options(&self) -> impl Iterator<Item = Self::ResponseType> {
    self.choices.into_iter()
  }

  fn is_valid_response(&self, response: &Self::ResponseType) -> bool {
    self.choices.contains(response)
  }

  fn as_prompt(&self) -> CounterPrompt<'_> {
    CounterPrompt::Pair(Cow::Borrowed(self))
  }

  fn into_prompt(self) -> CounterPrompt<'static> {
    CounterPrompt::Pair(Cow::Owned(self))
  }
}

#[derive(Default)]
struct CounterPolicy {
  calls: usize,
}

impl ChoicePolicy<CounterGame> for CounterPolicy {
  fn owner(&self, _state: &CounterState, _prompt: &CounterPrompt<'_>) -> ChoiceOwner {
    panic!("simulation must not inspect live choice ownership");
  }

  fn choose(&mut self, _state: &CounterState, prompt: &CounterPrompt<'_>) -> usize {
    self.calls += 1;
    match prompt {
      CounterPrompt::Number(prompt) => {
        assert_eq!(prompt.choices, [2, 5, 9]);
        1
      }
      CounterPrompt::Pair(prompt) => {
        assert_eq!(prompt.choices, [[1, 3], [4, 7]]);
        0
      }
    }
  }
}

impl Game for CounterGame {
  type State = CounterState;
  type Action = ();
  type StateAnimation = ();
  type Prompt<'a> = CounterPrompt<'a>;
  type Context = CounterContext;

  fn logical_clone(_state: &Self::State) -> Self::State {
    panic!("simulation must not clone a display snapshot");
  }

  fn is_legal_action(_state: &Self::State, _action: &Self::Action) -> bool {
    true
  }

  fn execute(context: &mut Self::Context, state: &mut Self::State, _action: Self::Action) {
    nested_rule(context, state);
  }
}

fn nested_rule(context: &mut CounterContext, state: &mut CounterState) {
  let number = context
    .execution
    .choose(state, NumberPrompt { choices: [2, 5, 9] });
  let pair = context.execution.choose(
    state,
    PairPrompt {
      choices: [[1, 3], [4, 7]],
    },
  );
  state.value = number + pair[0] + pair[1];
}

#[test]
fn nested_rules_return_distinct_typed_responses_in_stable_order() {
  let mut context = CounterContext {
    execution: ExecutionMode::Simulation {
      policy: CounterPolicy::default(),
    },
  };
  let mut state = CounterState { value: 0 };

  CounterGame::execute(&mut context, &mut state, ());

  assert_eq!(state.value, 9);
}

#[test]
fn simulation_present_skips_snapshot_and_animation_work() {
  let mut execution = ExecutionMode::<CounterGame, CounterPolicy>::Simulation {
    policy: CounterPolicy::default(),
  };
  let state = CounterState { value: 4 };

  execution.present(&state, || panic!("simulation must not build animations"));
}

fn interactive_shape(
  connection: DisplayConnection<CounterGame>,
) -> ExecutionMode<CounterGame, CounterPolicy> {
  ExecutionMode::Interactive {
    connection,
    policy: CounterPolicy::default(),
  }
}

#[test]
fn one_game_type_checks_for_both_execution_modes() {
  let _interactive_constructor = interactive_shape;
  let _simulation = ExecutionMode::<CounterGame, CounterPolicy>::Simulation {
    policy: CounterPolicy::default(),
  };
}

struct ChoiceFreeGame;

struct ChoiceFreeContext {
  execution: ExecutionMode<ChoiceFreeGame, ChoiceFreePolicy>,
}

struct ChoiceFreePolicy;

impl ChoicePolicy<ChoiceFreeGame> for ChoiceFreePolicy {
  fn owner(&self, _state: &u8, _prompt: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }

  fn choose(&mut self, _state: &u8, _prompt: &()) -> usize {
    0
  }
}

impl Game for ChoiceFreeGame {
  type State = u8;
  type Action = u8;
  type StateAnimation = ();
  type Prompt<'a> = ();
  type Context = ChoiceFreeContext;

  fn logical_clone(state: &Self::State) -> Self::State {
    *state
  }

  fn is_legal_action(_state: &Self::State, _action: &Self::Action) -> bool {
    true
  }

  fn execute(context: &mut Self::Context, state: &mut Self::State, action: Self::Action) {
    context.execution.present(state, || ());
    *state += action;
  }
}

#[test]
fn choice_free_game_runs_with_unit_prompt() {
  let mut context = ChoiceFreeContext {
    execution: ExecutionMode::Simulation {
      policy: ChoiceFreePolicy,
    },
  };
  let mut state = 2;

  ChoiceFreeGame::execute(&mut context, &mut state, 3);

  assert_eq!(state, 5);
}

struct WordGame;

#[derive(Clone)]
enum WordPrompt<'a> {
  Word(Cow<'a, WordChoices>),
}

#[derive(Clone)]
struct WordChoices {
  choices: Vec<String>,
}

impl PromptData<WordGame> for WordChoices {
  type ResponseType = String;

  fn options(&self) -> impl Iterator<Item = Self::ResponseType> {
    self.choices.iter().cloned()
  }

  fn is_valid_response(&self, response: &Self::ResponseType) -> bool {
    self.choices.contains(response)
  }

  fn as_prompt(&self) -> WordPrompt<'_> {
    WordPrompt::Word(Cow::Borrowed(self))
  }

  fn into_prompt(self) -> WordPrompt<'static> {
    WordPrompt::Word(Cow::Owned(self))
  }
}

struct WordPolicy;

impl ChoicePolicy<WordGame> for WordPolicy {
  fn owner(&self, _state: &(), _prompt: &WordPrompt<'_>) -> ChoiceOwner {
    ChoiceOwner::Human
  }

  fn choose(&mut self, _state: &(), prompt: &WordPrompt<'_>) -> usize {
    match prompt {
      WordPrompt::Word(prompt) => prompt
        .choices
        .iter()
        .position(|choice| choice == "beta")
        .unwrap(),
    }
  }
}

impl Game for WordGame {
  type State = ();
  type Action = ();
  type StateAnimation = ();
  type Prompt<'a> = WordPrompt<'a>;
  type Context = ExecutionMode<Self, WordPolicy>;

  fn logical_clone(_state: &Self::State) -> Self::State {}

  fn is_legal_action(_state: &Self::State, _action: &Self::Action) -> bool {
    true
  }

  fn execute(context: &mut Self::Context, state: &mut Self::State, _action: Self::Action) {
    assert_eq!(
      choose_from(
        context,
        state,
        WordChoices {
          choices: vec!["alpha".into(), "beta".into()],
        }
      ),
      "beta"
    );
  }
}

fn choose_from<G, C, P>(
  execution: &mut ExecutionMode<G, C>,
  state: &G::State,
  prompt: P,
) -> P::ResponseType
where
  G: Game,
  C: ChoicePolicy<G>,
  P: PromptData<G>,
{
  execution.choose(state, prompt)
}

#[test]
fn a_second_game_uses_the_same_generic_prompt_path() {
  let mut context = ExecutionMode::Simulation { policy: WordPolicy };

  WordGame::execute(&mut context, &mut (), ());
}

struct IndexPolicy(usize);

impl ChoicePolicy<CounterGame> for IndexPolicy {
  fn owner(&self, _state: &CounterState, _prompt: &CounterPrompt<'_>) -> ChoiceOwner {
    ChoiceOwner::Policy
  }

  fn choose(&mut self, _state: &CounterState, _prompt: &CounterPrompt<'_>) -> usize {
    self.0
  }
}

#[test]
#[should_panic(expected = "policy selected an invalid option index")]
fn out_of_range_policy_index_panics() {
  let mut execution = ExecutionMode::Simulation {
    policy: IndexPolicy(3),
  };

  execution.choose(
    &CounterState { value: 0 },
    NumberPrompt { choices: [2, 5, 9] },
  );
}

#[derive(Clone)]
struct InvalidPrompt;

impl PromptData<CounterGame> for InvalidPrompt {
  type ResponseType = u8;

  fn options(&self) -> impl Iterator<Item = Self::ResponseType> {
    [7].into_iter()
  }

  fn is_valid_response(&self, _response: &Self::ResponseType) -> bool {
    false
  }

  fn as_prompt(&self) -> CounterPrompt<'_> {
    CounterPrompt::Number(Cow::Owned(NumberPrompt { choices: [7; 3] }))
  }

  fn into_prompt(self) -> CounterPrompt<'static> {
    CounterPrompt::Number(Cow::Owned(NumberPrompt { choices: [7; 3] }))
  }
}

#[test]
#[should_panic(expected = "policy selected an invalid response")]
fn invalid_selected_response_panics() {
  let mut execution = ExecutionMode::Simulation {
    policy: IndexPolicy(0),
  };

  execution.choose(&CounterState { value: 0 }, InvalidPrompt);
}
