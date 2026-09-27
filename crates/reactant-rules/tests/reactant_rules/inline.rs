//! Runner tests are deliberately narrower than game behavior tests. They verify
//! FIFO, acceptance, and choice semantics at the rules boundary; a game should
//! normally exercise these through a rendered host instead of copying these probes.
use reactant_rules::{
  ChoiceOwner, ChoicePolicy, ExecutionMode, Game, PromptData, RulesContext, RulesWorker,
};
use std::{
  borrow::Cow,
  panic::{self, AssertUnwindSafe},
  time::Duration,
};
struct Counter;
struct Context {
  execution: ExecutionMode<Counter, Policy>,
}
struct Policy;
#[derive(Clone)]
struct Number;
impl PromptData<Counter> for Number {
  type ResponseType = usize;
  fn options(&self) -> impl Iterator<Item = usize> {
    [7, 11].into_iter()
  }
  fn is_valid_response(&self, response: &usize) -> bool {
    [7, 11].contains(response)
  }
  fn as_prompt(&self) -> Cow<'_, Number> {
    Cow::Borrowed(self)
  }
  fn into_prompt(self) -> Cow<'static, Number> {
    Cow::Owned(self)
  }
}
impl ChoicePolicy<Counter> for Policy {
  fn owner(&self, _: &usize, _: &Cow<'_, Number>) -> ChoiceOwner {
    ChoiceOwner::Human
  }
  fn choose(&mut self, _: &usize, _: &Cow<'_, Number>) -> usize {
    0
  }
}
impl Game for Counter {
  type State = usize;
  type Action = usize;
  type StateAnimation = usize;
  type Prompt<'a> = Cow<'a, Number>;
  type Context = Context;
  fn logical_clone(state: &usize) -> usize {
    *state
  }
  fn is_legal_action(_: &usize, count: &usize) -> bool {
    *count > 0
  }
  fn execute(cx: &mut Context, state: &mut usize, count: usize) {
    for value in 1..=count {
      *state = value;
      cx.execution.present(state, || value);
    }
    *state += cx.execution.choose(state, Number);
  }
}
fn context() -> RulesContext<Counter> {
  RulesContext::new(|connection| Context {
    execution: ExecutionMode::Interactive {
      connection,
      policy: Policy,
    },
  })
}

#[test]
fn inline_publications_exceed_worker_capacity_without_blocking_or_reordering() {
  // More than 32 checkpoints would deadlock a bounded FIFO on the same stack.
  // Normal-return state remains separate until the consumer accepts its completion.
  let runner = RulesWorker::inline().answer_with::<Counter>(|_| 1);
  let mut context = context();
  let run = context.start(&runner, &0, 80);
  assert!(!run.publication_observation().waiting_for_capacity);
  for value in 1..=80 {
    let checkpoint = run.take_checkpoint().unwrap();
    assert_eq!(*checkpoint.state(), value);
    assert_eq!(checkpoint.animation(), Some(&value));
  }
  let final_output = run.take_checkpoint().unwrap().into_parts();
  assert_eq!(final_output.state, 91);
  assert_eq!(context.accept(final_output.completion.unwrap()), 91);
  assert!(run.take_checkpoint().is_none());
  // Inline work must not fabricate worker lifecycle notifications.
  assert!(!run.observation().started);
  assert!(!run.observation().stopped);
}

#[test]
fn missing_and_invalid_scripted_answers_fail_without_accepting_state() {
  // Human choices may not silently fall back to a policy or create an unresolved
  // prompt. Failure abandons the entire run, including its earlier publications.
  for runner in [
    RulesWorker::inline(),
    RulesWorker::inline().answer_with::<Counter>(|_| 8),
  ] {
    let run = context().start(&runner, &0, 1);
    assert!(run.observation().failure.is_some());
    assert!(run.take_checkpoint().is_none());
  }
}

#[test]
fn abandoned_inline_work_cannot_leak_into_a_replacement() {
  // Cancellation after synchronous execution still matters: output may not have
  // reached the host yet. Dropping it must discard old completion and choice state.
  let runner = RulesWorker::inline().answer_with::<Counter>(|_| 0);
  let old = context().start(&runner, &0, 4);
  old.stop();
  assert!(old.take_checkpoint().is_none());
  let replacement = context().start(&runner, &0, 1);
  assert_eq!(replacement.take_checkpoint().unwrap().animation(), Some(&1));
  assert_eq!(*replacement.take_checkpoint().unwrap().state(), 8);
}

#[test]
fn inline_waits_are_developer_errors() {
  // A future accidental wait fails immediately instead of making a fast test flaky.
  let run = context().start(&RulesWorker::inline().answer_with::<Counter>(|_| 0), &0, 1);
  assert!(
    panic::catch_unwind(AssertUnwindSafe(
      || run.wait_for_worker_started(Duration::ZERO)
    ))
    .is_err()
  );
  assert!(
    panic::catch_unwind(AssertUnwindSafe(
      || run.wait_for_publication(Duration::ZERO, |_| true)
    ))
    .is_err()
  );
}
