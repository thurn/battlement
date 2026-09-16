use std::{
  alloc::{GlobalAlloc, Layout, System},
  borrow::Cow,
  cell::Cell,
  hint,
};

use reactant_rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game, PromptData};

struct CountingAllocator;

thread_local! {
  static TRACKING: Cell<bool> = const { Cell::new(false) };
  static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for CountingAllocator {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    TRACKING.with(|tracking| {
      if tracking.get() {
        ALLOCATIONS.with(|allocations| allocations.set(allocations.get() + 1));
      }
    });
    unsafe { System.alloc(layout) }
  }

  unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
    unsafe { System.dealloc(pointer, layout) }
  }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn allocations_during<T>(operation: impl FnOnce() -> T) -> (T, usize) {
  ALLOCATIONS.with(|allocations| allocations.set(0));
  TRACKING.with(|tracking| tracking.set(true));
  let value = operation();
  TRACKING.with(|tracking| tracking.set(false));
  let allocations = ALLOCATIONS.with(Cell::get);
  (value, allocations)
}

struct AllocationGame;

#[derive(Clone)]
enum AllocationPrompt<'a> {
  Fixed(Cow<'a, FixedPrompt>),
}

#[derive(Clone, Copy)]
struct FixedPrompt {
  choices: [usize; 3],
}

impl PromptData<AllocationGame> for FixedPrompt {
  type ResponseType = usize;

  fn options(&self) -> impl Iterator<Item = Self::ResponseType> {
    self.choices.into_iter()
  }

  fn is_valid_response(&self, response: &Self::ResponseType) -> bool {
    self.choices.contains(response)
  }

  fn as_prompt(&self) -> AllocationPrompt<'_> {
    AllocationPrompt::Fixed(Cow::Borrowed(self))
  }

  fn into_prompt(self) -> AllocationPrompt<'static> {
    AllocationPrompt::Fixed(Cow::Owned(self))
  }
}

struct FixedPolicy;

impl ChoicePolicy<AllocationGame> for FixedPolicy {
  fn owner(&self, _state: &(), _prompt: &AllocationPrompt<'_>) -> ChoiceOwner {
    ChoiceOwner::Policy
  }

  fn choose(&mut self, _state: &(), prompt: &AllocationPrompt<'_>) -> usize {
    match prompt {
      AllocationPrompt::Fixed(prompt) => {
        hint::black_box(prompt.choices);
        2
      }
    }
  }
}

impl Game for AllocationGame {
  type State = ();
  type Action = ();
  type StateAnimation = ();
  type Prompt<'a> = AllocationPrompt<'a>;
  type Context = ExecutionMode<Self, FixedPolicy>;

  fn logical_clone(_state: &Self::State) -> Self::State {}

  fn is_legal_action(_state: &Self::State, _action: &Self::Action) -> bool {
    true
  }

  fn execute(_context: &mut Self::Context, _state: &mut Self::State, _action: Self::Action) {}
}

#[test]
fn public_simulation_primitives_add_no_allocations() {
  let mut execution = ExecutionMode::Simulation {
    policy: FixedPolicy,
  };
  let prompt = FixedPrompt {
    choices: [4, 8, 15],
  };
  let (_, present_allocations) = allocations_during(|| execution.present(&(), || ()));
  let (selected, choice_allocations) = allocations_during(|| execution.choose(&(), prompt));

  assert_eq!(selected, 15);
  assert_eq!(present_allocations, 0);
  assert_eq!(choice_allocations, 0);
}

#[test]
fn owned_prompt_construction_is_measured_separately() {
  let (choices, prompt_allocations) = allocations_during(|| vec![4, 8, 15]);

  assert_eq!(choices, [4, 8, 15]);
  assert!(prompt_allocations > 0);
}

#[test]
fn policy_search_allocations_are_not_primitive_overhead() {
  let (_, rollout_allocations) = allocations_during(|| {
    let candidates = vec![4, 8, 15];
    hint::black_box(candidates.into_iter().max())
  });

  assert!(rollout_allocations > 0);
}
