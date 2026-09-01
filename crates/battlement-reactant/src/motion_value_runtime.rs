use std::{
  cell::RefCell,
  rc::{Rc, Weak},
};

use battlement::{
  CommandBody, MotionPlaybackEvent, MotionPlaybackOutcome, MotionValueSample, ObjectId,
};

thread_local! {
  static CURRENT_RUNTIME: RefCell<Option<RuntimeContext>> = const { RefCell::new(None) };
}

pub(crate) struct MotionValueRuntime {
  commands: Vec<CommandBody>,
  subscriptions: Vec<Subscription>,
  playbacks: Vec<PlaybackSubscription>,
}

struct Subscription {
  id: ObjectId,
  invoke: Box<dyn Fn(&MotionValueSample) -> bool>,
}

struct PlaybackSubscription {
  id: ObjectId,
  generation: u32,
  invoke: Box<dyn Fn(MotionPlaybackOutcome) -> bool>,
}

pub(crate) struct PlaybackInvocation {
  outcome: MotionPlaybackOutcome,
  invoke: Box<dyn Fn(MotionPlaybackOutcome) -> bool>,
}

#[derive(Clone)]
struct RuntimeContext {
  runtime_id: u64,
  runtime: Weak<RefCell<MotionValueRuntime>>,
}

pub(crate) struct RuntimeGuard(Option<RuntimeContext>);

impl MotionValueRuntime {
  pub(crate) fn new(_runtime_id: u64) -> Rc<RefCell<Self>> {
    Rc::new(RefCell::new(Self {
      commands: Vec::new(),
      subscriptions: Vec::new(),
      playbacks: Vec::new(),
    }))
  }

  pub(crate) fn queued_commands(&self) -> usize {
    self.commands.len()
  }

  pub(crate) fn truncate_commands(&mut self, length: usize) {
    self.commands.truncate(length);
  }

  pub(crate) fn command_groups(&self, length: usize) -> Vec<Vec<CommandBody>> {
    self.commands[..length]
      .iter()
      .cloned()
      .map(|body| vec![body])
      .collect()
  }

  pub(crate) fn consume_commands(&mut self, length: usize) {
    self.commands.drain(..length);
  }

  pub(crate) fn clear(&mut self) {
    self.commands.clear();
    self.subscriptions.clear();
    self.playbacks.clear();
  }

  pub(crate) fn register_subscription(
    &mut self,
    id: ObjectId,
    invoke: impl Fn(&MotionValueSample) -> bool + 'static,
  ) {
    assert!(
      !self.subscriptions.iter().any(|value| value.id == id),
      "motion-value subscription identity is duplicated"
    );
    self.subscriptions.push(Subscription {
      id,
      invoke: Box::new(invoke),
    });
  }

  pub(crate) fn apply_samples(&mut self, samples: &[MotionValueSample]) -> bool {
    let mut invoked = false;
    self.subscriptions.retain(|subscription| {
      let matching = samples
        .iter()
        .find(|sample| sample.subscription_id == subscription.id);
      match matching {
        Some(sample) => {
          invoked |= (subscription.invoke)(sample);
          true
        }
        None => true,
      }
    });
    invoked
  }

  pub(crate) fn register_playback(
    &mut self,
    id: ObjectId,
    generation: u32,
    invoke: impl Fn(MotionPlaybackOutcome) -> bool + 'static,
  ) {
    assert!(
      !self
        .playbacks
        .iter()
        .any(|value| value.id == id && value.generation == generation),
      "motion playback identity is duplicated"
    );
    self.playbacks.push(PlaybackSubscription {
      id,
      generation,
      invoke: Box::new(invoke),
    });
  }

  pub(crate) fn take_playback_events(
    &mut self,
    events: &[MotionPlaybackEvent],
  ) -> Vec<PlaybackInvocation> {
    let mut invocations = Vec::new();
    let mut retained = Vec::new();
    for subscription in self.playbacks.drain(..) {
      let matching = events.iter().find(|event| {
        event.playback_id == subscription.id && event.generation == subscription.generation
      });
      if let Some(event) = matching {
        invocations.push(PlaybackInvocation {
          outcome: event.outcome,
          invoke: subscription.invoke,
        });
      } else {
        retained.push(subscription);
      }
    }
    self.playbacks = retained;
    invocations
  }
}

impl PlaybackInvocation {
  pub(crate) fn invoke(self) -> bool {
    (self.invoke)(self.outcome)
  }
}

impl Drop for RuntimeGuard {
  fn drop(&mut self) {
    CURRENT_RUNTIME.with(|current| current.replace(self.0.take()));
  }
}

pub(crate) fn enter_runtime(
  runtime_id: u64,
  runtime: &Rc<RefCell<MotionValueRuntime>>,
) -> RuntimeGuard {
  RuntimeGuard(CURRENT_RUNTIME.with(|current| {
    current.replace(Some(RuntimeContext {
      runtime_id,
      runtime: Rc::downgrade(runtime),
    }))
  }))
}

pub(crate) fn current_runtime() -> (u64, Weak<RefCell<MotionValueRuntime>>) {
  CURRENT_RUNTIME.with(|current| {
    let current = current.borrow();
    let current = current
      .as_ref()
      .expect("motion-value hooks require a Reactant runtime context");
    (current.runtime_id, current.runtime.clone())
  })
}

pub(crate) fn queue(
  runtime_id: u64,
  runtime: &Weak<RefCell<MotionValueRuntime>>,
  body: CommandBody,
) {
  assert!(
    !crate::context::rendering(),
    "motion-value commands are forbidden during render"
  );
  CURRENT_RUNTIME.with(|current| {
    if let Some(current) = current.borrow().as_ref() {
      assert_eq!(
        current.runtime_id, runtime_id,
        "motion-value commands cannot cross Reactant runtimes"
      );
    }
  });
  if let Some(runtime) = runtime.upgrade() {
    runtime.borrow_mut().commands.push(body);
  }
}
