use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  mem,
  rc::{Rc, Weak},
};

use battlement::{
  Command, CommandBody, MotionPlaybackCommand, MotionPlaybackEvent, MotionPlaybackOutcome,
  MotionSelector, MotionSequenceLabelEvent, MotionValuePlaybackOperation, MotionValueSample,
  ObjectId, Prop,
};

use crate::{
  element_ref::ElementRefRuntime, host_node::HostNode, native_identity_lease::NativeIdentityLease,
};

thread_local! {
  static CURRENT_RUNTIME: RefCell<Option<RuntimeContext>> = const { RefCell::new(None) };
}

pub(crate) struct MotionValueRuntime {
  commands: Vec<(Option<u64>, Command)>,
  consumed_owners: HashMap<ObjectId, u64>,
  subscriptions: Vec<Subscription>,
  playbacks: Vec<PlaybackSubscription>,
  scope_targets: HashMap<ObjectId, Vec<ScopeTarget>>,
  element_refs: Weak<RefCell<ElementRefRuntime>>,
}

struct Subscription {
  id: ObjectId,
  invoke: Box<dyn Fn(&MotionValueSample) -> bool>,
}

struct PlaybackSubscription {
  id: ObjectId,
  generation: u32,
  scope: Option<u64>,
  invoke: Box<dyn Fn(MotionPlaybackOutcome) -> bool>,
  label: Rc<dyn Fn(&str) -> bool>,
  _native_identities: Vec<Rc<NativeIdentityLease>>,
}

#[derive(Clone)]
struct ScopeTarget {
  object_id: ObjectId,
  parent_id: Option<ObjectId>,
  name: Option<String>,
  root: bool,
}

pub(crate) struct PlaybackInvocation {
  outcome: MotionPlaybackOutcome,
  invoke: Box<dyn Fn(MotionPlaybackOutcome) -> bool>,
  released_native_identity: bool,
}

pub(crate) struct LabelInvocation {
  label: String,
  invoke: Rc<dyn Fn(&str) -> bool>,
}

#[derive(Clone)]
struct RuntimeContext {
  runtime_id: u64,
  runtime: Weak<RefCell<MotionValueRuntime>>,
}

pub(crate) struct RuntimeGuard(Option<RuntimeContext>);

impl MotionValueRuntime {
  pub(crate) fn new(
    _runtime_id: u64,
    element_refs: &Rc<RefCell<ElementRefRuntime>>,
  ) -> Rc<RefCell<Self>> {
    Rc::new(RefCell::new(Self {
      commands: Vec::new(),
      consumed_owners: HashMap::new(),
      subscriptions: Vec::new(),
      playbacks: Vec::new(),
      scope_targets: HashMap::new(),
      element_refs: Rc::downgrade(element_refs),
    }))
  }

  pub(crate) fn queued_commands(&self) -> usize {
    self.commands.len()
  }

  pub(crate) fn truncate_commands(&mut self, length: usize) {
    self.commands.truncate(length);
  }

  pub(crate) fn command_groups(&self, length: usize) -> Vec<Vec<Command>> {
    self.commands[..length]
      .iter()
      .map(|(_, command)| vec![command.clone()])
      .collect()
  }

  pub(crate) fn commands_after(&self, offset: usize) -> Vec<Command> {
    self.commands[offset..]
      .iter()
      .map(|(_, command)| command.clone())
      .collect()
  }

  pub(crate) fn consume_commands(&mut self, length: usize) {
    for (scope, command) in self.commands.drain(..length) {
      if let (Some(scope), Some(target)) = (scope, crate::work_scope::target(&command.body)) {
        self.consumed_owners.insert(target, scope);
      }
    }
  }

  pub(crate) fn take_owners(&mut self) -> HashMap<ObjectId, u64> {
    mem::take(&mut self.consumed_owners)
  }

  pub(crate) fn clear(&mut self) {
    self.commands.clear();
    self.consumed_owners.clear();
    self.subscriptions.clear();
    self.playbacks.clear();
    self.scope_targets.clear();
  }

  pub(crate) fn clear_playbacks(&mut self) {
    let cancellations = self
      .playbacks
      .iter_mut()
      .map(|subscription| {
        subscription.invoke = Box::new(|_| false);
        subscription.label = Rc::new(|_| false);
        (
          subscription.scope,
          Command::new_v4(CommandBody::MotionValuePlayback(
            MotionValuePlaybackOperation {
              playback_id: subscription.id,
              generation: subscription.generation,
              command: MotionPlaybackCommand::Cancel,
            },
          )),
        )
      })
      .collect::<Vec<_>>();
    for cancellation in cancellations {
      self.commands.push((cancellation.0, cancellation.1));
    }
  }

  pub(crate) fn install_scope_targets(&mut self, roots: &[&[HostNode]]) {
    self.scope_targets.clear();
    for roots in roots {
      self.collect_scope_roots(roots);
    }
  }

  pub(crate) fn resolve_scope_targets(
    &self,
    scope_id: ObjectId,
    selectors: &[MotionSelector],
  ) -> Vec<ObjectId> {
    let Some(targets) = self.scope_targets.get(&scope_id) else {
      return Vec::new();
    };
    let root = targets
      .iter()
      .find(|target| target.root)
      .map(|target| target.object_id);
    let mut selected = HashSet::new();
    for selector in selectors {
      for target in targets {
        let matches = match selector {
          MotionSelector::Element(_) => false,
          MotionSelector::Name(name) => target.name.as_ref() == Some(name),
          MotionSelector::ScopeRoot => target.root,
          MotionSelector::Children => target.parent_id == root,
          MotionSelector::Descendants => !target.root,
        };
        if matches {
          selected.insert(target.object_id);
        }
      }
    }
    selected.into_iter().collect()
  }

  pub(crate) fn retain_scope_targets(
    &self,
    scope_id: ObjectId,
    selectors: &[MotionSelector],
  ) -> Vec<Rc<NativeIdentityLease>> {
    let targets = self.resolve_scope_targets(scope_id, selectors);
    let element_refs = self
      .element_refs
      .upgrade()
      .expect("motion scope element-ref runtime is unavailable");
    let element_refs = element_refs.borrow();
    targets
      .into_iter()
      .map(|object_id| element_refs.retain_native_identity(object_id))
      .collect()
  }

  fn collect_scope_roots(&mut self, hosts: &[HostNode]) {
    for host in hosts {
      if let Prop::Set(descriptor) = host.motion()
        && descriptor.scope_root
        && let Some(scope_id) = descriptor.scope_id
      {
        self.collect_scope_subtree(scope_id, host, None, true);
      }
      self.collect_scope_roots(&host.children);
    }
  }

  fn collect_scope_subtree(
    &mut self,
    scope_id: ObjectId,
    host: &HostNode,
    parent_id: Option<ObjectId>,
    root: bool,
  ) {
    let name = host
      .motion_descriptor()
      .and_then(|value| value.motion_name.clone());
    self
      .scope_targets
      .entry(scope_id)
      .or_default()
      .push(ScopeTarget {
        object_id: host.object_id,
        parent_id,
        name,
        root,
      });
    for child in &host.children {
      self.collect_scope_subtree(scope_id, child, Some(host.object_id), false);
    }
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
    scope: Option<u64>,
    invoke: impl Fn(MotionPlaybackOutcome) -> bool + 'static,
    label: impl Fn(&str) -> bool + 'static,
    native_identities: Vec<Rc<NativeIdentityLease>>,
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
      scope,
      invoke: Box::new(invoke),
      label: Rc::new(label),
      _native_identities: native_identities,
    });
  }

  pub(crate) fn take_label_events(
    &mut self,
    events: &[MotionSequenceLabelEvent],
  ) -> Vec<LabelInvocation> {
    let mut invocations = Vec::new();
    for event in events {
      let Some(index) = self.playbacks.iter().position(|subscription| {
        subscription.id == event.playback_id && subscription.generation == event.generation
      }) else {
        continue;
      };
      invocations.push(LabelInvocation {
        label: event.label.clone(),
        invoke: Rc::clone(&self.playbacks[index].label),
      });
    }
    invocations
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
          released_native_identity: !subscription._native_identities.is_empty(),
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
    (self.invoke)(self.outcome) || self.released_native_identity
  }
}

impl LabelInvocation {
  pub(crate) fn invoke(self) -> bool {
    (self.invoke)(&self.label)
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
  scope: Option<u64>,
  body: CommandBody,
) {
  queue_command(
    runtime_id,
    runtime,
    scope,
    Command::new_v4(body).nonblocking(),
  );
}

pub(crate) fn queue_blocking(
  runtime_id: u64,
  runtime: &Weak<RefCell<MotionValueRuntime>>,
  scope: Option<u64>,
  body: CommandBody,
) {
  queue_command(runtime_id, runtime, scope, Command::new_v4(body));
}

fn queue_command(
  runtime_id: u64,
  runtime: &Weak<RefCell<MotionValueRuntime>>,
  scope: Option<u64>,
  command: Command,
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
    runtime.borrow_mut().commands.push((scope, command));
  }
}
