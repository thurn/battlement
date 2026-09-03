use std::{cell::RefCell, panic, rc::Rc};

use battlement::{
  CameraState, CommandBody, GameObject, GameObjectKind, MotionEventBatch, MotionEventKind,
  MotionGeneration, MotionLayer, MotionLifecycleEvent, MotionSequence, MotionSlotId, ObjectId,
  PanelScaleMode, PanelSettings, ParentScene, PreparedAsset, Prop, Scene, SceneId, SessionId,
  Snapshot, UiDocument, UiDocumentState, UiVisualElementProperties,
};
use battlement_reactant::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  prelude::*,
  runtime::{Reactant, ReactantCommit},
};

struct IdleSpawner;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Route {
  First,
  Second,
}

struct Game {
  open: bool,
  route: Route,
  mode: PresenceMode,
  completed: usize,
  slot_cancelled: usize,
  slot_completed: usize,
}

#[derive(Clone)]
struct RetainedCard {
  label: &'static str,
  lifecycle: Rc<RefCell<Vec<&'static str>>>,
  manual: bool,
  presence: Rc<RefCell<Option<Presence>>>,
  setter: Rc<RefCell<Option<StateSetter<u32>>>>,
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl Component for RetainedCard {
  fn render(&self) -> impl Render {
    let (value, setter) = use_state(7_u32);
    let present = if self.manual {
      let presence = use_presence();
      self.presence.replace(Some(presence.clone()));
      presence.is_present()
    } else {
      use_is_present()
    };
    self.setter.replace(Some(setter));
    let lifecycle = Rc::clone(&self.lifecycle);
    use_effect(
      move || {
        lifecycle.borrow_mut().push("mount");
        move || lifecycle.borrow_mut().push("unmount")
      },
      (),
    );
    View::new()
      .name(self.label)
      .layout(Layout::Both)
      .animate(MotionStyle::new().opacity(1.0))
      .exit(
        MotionTarget::new(MotionStyle::new().opacity(0.0))
          .transition(Transition::tween().duration_secs(0.2))
          .on_complete(|game: &mut Game| game.slot_completed += 1)
          .on_cancel(|game: &mut Game| game.slot_cancelled += 1),
      )
      .child(Label::new(format!(
        "{}:{value}:{}",
        self.label,
        if present { "present" } else { "exiting" }
      )))
  }
}

#[test]
fn automatic_exit_retains_hooks_until_exact_generation_completion() {
  let mut fixture = Fixture::new(PresenceMode::Sync, false);
  let initial = fixture.start();
  let host_id = initial.children[0].object_id;
  let _ = fixture.poll().into_groups();
  assert_eq!(&*fixture.lifecycle.borrow(), &["mount"]);

  fixture.game.open = false;
  let (descriptor_id, generation, _) = exit_update(fixture.refresh());
  assert_eq!(descriptor_id, host_id);
  assert_eq!(&*fixture.lifecycle.borrow(), &["mount"]);

  let _ = fixture
    .complete(descriptor_id, MotionGeneration(generation.0 - 1))
    .into_groups();
  assert_eq!(fixture.game.completed, 0);
  fixture
    .setter
    .borrow()
    .as_ref()
    .expect("retained setter")
    .set(12);
  let update = fixture.poll();
  assert!(!update.is_empty());
  let _ = update.into_groups();
  assert_eq!(&*fixture.lifecycle.borrow(), &["mount"]);

  let removed = fixture.complete(descriptor_id, generation);
  assert!(contains_destroy(removed, host_id));
  assert_eq!(fixture.game.completed, 1);
  assert_eq!(fixture.game.slot_completed, 1);
  let _ = fixture.poll().into_groups();
  assert_eq!(&*fixture.lifecycle.borrow(), &["mount", "unmount"]);
  fixture.shutdown();
}

#[test]
fn manual_hold_reconnect_and_rapid_reopen_preserve_one_mount() {
  let mut fixture = Fixture::new(PresenceMode::Sync, true);
  let initial = fixture.start();
  let host_id = initial.children[0].object_id;
  let _ = fixture.poll().into_groups();
  fixture.game.open = false;
  let (descriptor_id, generation, _) = exit_update(fixture.refresh());

  let reconnect = fixture.start();
  let Prop::Set(reconnected) = &reconnect.children[0].element.visual_element().motion else {
    panic!("reconnect lost retained motion");
  };
  assert_eq!(reconnected.generation, generation);
  fixture.game.open = true;
  let reopened = fixture.refresh();
  assert!(!contains_destroy(reopened, host_id));
  let _ = fixture
    .terminal(descriptor_id, generation, MotionEventKind::Cancelled)
    .into_groups();
  assert_eq!(fixture.game.completed, 0);
  assert_eq!(fixture.game.slot_completed, 0);
  assert_eq!(fixture.game.slot_cancelled, 1);
  assert_eq!(&*fixture.lifecycle.borrow(), &["mount"]);

  fixture.game.open = false;
  let (descriptor_id, second_generation, _) = exit_update(fixture.refresh());
  let second_reconnect = fixture.start();
  let Prop::Set(reconnected) = &second_reconnect.children[0].element.visual_element().motion else {
    panic!("second reconnect lost retained motion");
  };
  assert_eq!(reconnected.generation, second_generation);
  fixture.sequence = 0;
  let _ = fixture
    .complete(descriptor_id, second_generation)
    .into_groups();
  assert_eq!(fixture.game.slot_completed, 1);
  fixture
    .presence
    .borrow()
    .as_ref()
    .expect("exiting presence handle")
    .safe_to_remove();
  let removed = fixture.poll();
  assert!(contains_destroy(removed, host_id));
  assert_eq!(fixture.game.completed, 1);
  let _ = fixture.poll().into_groups();
  assert_eq!(&*fixture.lifecycle.borrow(), &["mount", "unmount"]);
  fixture.shutdown();
}

#[test]
fn exit_without_automatic_tracks_completes_on_the_next_boundary() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |game: &Game| {
    AnimatePresence::new()
      .on_exit_complete(|game: &mut Game| game.completed += 1)
      .child(
        game
          .open
          .then(|| Node::new(View::new().name("immediate").key("immediate"))),
      )
  });
  let mut game = Game {
    open: true,
    route: Route::First,
    mode: PresenceMode::Sync,
    completed: 0,
    slot_cancelled: 0,
    slot_completed: 0,
  };
  let _ = reactant
    .begin_session(&mut game)
    .unwrap()
    .into_parts(snapshot(&document));
  game.open = false;
  let removed = reactant.refresh(&mut game).unwrap();
  assert!(
    removed
      .into_groups()
      .into_iter()
      .flatten()
      .any(|body| matches!(body, CommandBody::VisualElementDestroy(_)))
  );
  assert_eq!(game.completed, 0);
  let _ = reactant.poll(&mut game).unwrap().into_groups();
  assert_eq!(game.completed, 1);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn wait_defers_the_next_key_and_pop_layout_marks_the_exiting_projection() {
  let mut fixture = Fixture::new(PresenceMode::Wait, false);
  let initial = fixture.start();
  assert_eq!(
    initial.children[0].element.visual_element().name,
    Prop::Set("first".to_owned())
  );
  let _ = fixture.poll().into_groups();

  fixture.game.route = Route::Second;
  let (descriptor_id, generation, _) = exit_update(fixture.refresh());
  assert_eq!(&*fixture.lifecycle.borrow(), &["mount"]);
  let entered = fixture.terminal(descriptor_id, generation, MotionEventKind::Cancelled);
  assert_eq!(fixture.game.slot_cancelled, 1);
  assert!(
    entered
      .into_groups()
      .into_iter()
      .flatten()
      .any(|body| matches!(body, CommandBody::VisualElementCreate(_)))
  );
  fixture.shutdown();

  let mut pop = Fixture::new(PresenceMode::PopLayout, false);
  let _ = pop.start();
  let _ = pop.poll().into_groups();
  pop.game.open = false;
  let (descriptor_id, generation, pop_layout) = exit_update(pop.refresh());
  assert!(pop_layout);
  let _ = pop.complete(descriptor_id, generation).into_groups();
  pop.shutdown();
}

struct Fixture {
  document: UiDocument,
  game: Game,
  reactant: Reactant<Game>,
  lifecycle: Rc<RefCell<Vec<&'static str>>>,
  presence: Rc<RefCell<Option<Presence>>>,
  setter: Rc<RefCell<Option<StateSetter<u32>>>>,
  sequence: u64,
}

impl Fixture {
  fn new(mode: PresenceMode, manual: bool) -> Self {
    let document = document();
    let lifecycle = Rc::new(RefCell::new(Vec::new()));
    let presence = Rc::new(RefCell::new(None));
    let setter = Rc::new(RefCell::new(None));
    let mut reactant = Reactant::new(IdleSpawner);
    let view_lifecycle = Rc::clone(&lifecycle);
    let view_presence = Rc::clone(&presence);
    let view_setter = Rc::clone(&setter);
    reactant.register_root(document.clone(), move |game: &Game| {
      let label = match game.route {
        Route::First => "first",
        Route::Second => "second",
      };
      AnimatePresence::new()
        .initial(false)
        .mode(game.mode)
        .on_exit_complete(|game: &mut Game| game.completed += 1)
        .child(game.open.then(|| {
          Node::new(
            RetainedCard {
              label,
              lifecycle: Rc::clone(&view_lifecycle),
              manual,
              presence: Rc::clone(&view_presence),
              setter: Rc::clone(&view_setter),
            }
            .key(label),
          )
        }))
    });
    Self {
      document,
      game: Game {
        open: true,
        route: Route::First,
        mode,
        completed: 0,
        slot_cancelled: 0,
        slot_completed: 0,
      },
      reactant,
      lifecycle,
      presence,
      setter,
      sequence: 0,
    }
  }

  fn start(&mut self) -> UiDocument {
    self
      .reactant
      .begin_session(&mut self.game)
      .unwrap()
      .into_parts(snapshot(&self.document))
      .0
      .ui
      .into_iter()
      .find(|value| value.document_id == self.document.document_id)
      .unwrap()
  }

  fn refresh(&mut self) -> ReactantCommit {
    self.reactant.refresh(&mut self.game).unwrap()
  }

  fn poll(&mut self) -> ReactantCommit {
    self.reactant.poll(&mut self.game).unwrap()
  }

  fn complete(&mut self, descriptor_id: ObjectId, generation: MotionGeneration) -> ReactantCommit {
    self.terminal(descriptor_id, generation, MotionEventKind::Completed)
  }

  fn terminal(
    &mut self,
    descriptor_id: ObjectId,
    generation: MotionGeneration,
    kind: MotionEventKind,
  ) -> ReactantCommit {
    self.sequence += 1;
    self
      .reactant
      .motion_events(
        &mut self.game,
        MotionEventBatch {
          first_sequence: MotionSequence(self.sequence),
          last_sequence: MotionSequence(self.sequence),
          events: vec![MotionLifecycleEvent {
            sequence: MotionSequence(self.sequence),
            descriptor_id,
            slot: MotionSlotId(1),
            generation,
            elapsed_micros: 200_000,
            kind,
          }],
          samples: Vec::new(),
          value_samples: Vec::new(),
          playback_events: Vec::new(),
          gesture_events: Vec::new(),
        },
      )
      .unwrap()
  }

  fn shutdown(&mut self) {
    let _ = self.reactant.shutdown(&mut self.game).into_groups();
  }
}

fn exit_update(commit: ReactantCommit) -> (ObjectId, MotionGeneration, bool) {
  commit
    .into_groups()
    .into_iter()
    .flatten()
    .find_map(|body| {
      let CommandBody::VisualElementUpdate(update) = body else {
        return None;
      };
      let battlement::VisualElementUpdate::Properties { object_id, element } = *update else {
        return None;
      };
      let Prop::Set(descriptor) = element.visual_element().motion.clone() else {
        return None;
      };
      descriptor
        .slots
        .iter()
        .any(|slot| slot.layer == MotionLayer::Exit)
        .then_some((
          object_id,
          descriptor.generation,
          descriptor.layout.is_some_and(|layout| layout.pop_layout),
        ))
    })
    .expect("presence removal did not install an exit descriptor")
}

fn contains_destroy(commit: ReactantCommit, object_id: ObjectId) -> bool {
  commit.into_groups().into_iter().flatten().any(
    |body| matches!(body, CommandBody::VisualElementDestroy(value) if value.object_id == object_id),
  )
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(document: &UiDocument) -> Snapshot {
  let camera_id = ObjectId::new_v4();
  Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    vec![
      GameObject::new(camera_id, CameraState::new()),
      GameObject::new(
        document.document_id,
        GameObjectKind::UiDocument(UiDocumentState::new(document.root_id).panel_settings(
          PanelSettings::new().scale_mode(PanelScaleMode::ConstantLogicalPixelSize),
        )),
      )
      .parent_scene(ParentScene::Persistent),
    ],
    camera_id,
  )
}
