use std::{
  cell::{Cell, RefCell},
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{
  CameraState, CommandBody, GameObject, GameObjectKind, ObjectId, PanelScaleMode, PanelSettings,
  ParentScene, PreparedAsset, Scene, SceneId, SessionId, Snapshot, UiDocument, UiDocumentState,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  component::Component,
  executor::{BoxFuture, SpawnedTask, Spawner},
  hooks::{use_effect, use_effect_always, use_state},
  render::Render,
  runtime::{Reactant, ReactantCommit},
};

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

struct ReadyEffect {
  host_committed: Rc<Cell<bool>>,
  observed_commit: Rc<Cell<bool>>,
}

impl Component for ReadyEffect {
  fn render(&self) -> impl Render {
    let (ready, set_ready) = use_state(false);
    let host_committed = Rc::clone(&self.host_committed);
    let observed_commit = Rc::clone(&self.observed_commit);
    use_effect(
      move || {
        observed_commit.set(host_committed.get());
        set_ready.set(true);
      },
      (),
    );
    battlement_reactant::host::Label::new(if ready { "ready" } else { "waiting" })
  }
}

struct FrequencyEffects {
  log: Rc<RefCell<Vec<&'static str>>>,
}

impl Component for FrequencyEffects {
  fn render(&self) -> impl Render {
    let mount_log = Rc::clone(&self.log);
    use_effect(move || mount_log.borrow_mut().push("mount-only"), ());
    let always_log = Rc::clone(&self.log);
    use_effect_always(move || always_log.borrow_mut().push("always"));
    battlement_reactant::host::Label::new("frequency")
  }
}

struct OrderedParent {
  dependency: u8,
  show_child: bool,
  log: Rc<RefCell<Vec<String>>>,
}

struct OrderedChild {
  dependency: u8,
  log: Rc<RefCell<Vec<String>>>,
}

impl Component for OrderedParent {
  fn render(&self) -> impl Render {
    let dependency = self.dependency;
    let setup_log = Rc::clone(&self.log);
    use_effect(
      move || {
        setup_log
          .borrow_mut()
          .push(format!("parent setup {dependency}"));
        let cleanup_log = Rc::clone(&setup_log);
        move || {
          cleanup_log
            .borrow_mut()
            .push(format!("parent cleanup {dependency}"));
        }
      },
      dependency,
    );
    (
      self.show_child.then(|| OrderedChild {
        dependency,
        log: Rc::clone(&self.log),
      }),
      battlement_reactant::host::Label::new("parent"),
    )
  }
}

impl Component for OrderedChild {
  fn render(&self) -> impl Render {
    let dependency = self.dependency;
    let setup_log = Rc::clone(&self.log);
    use_effect(
      move || {
        setup_log
          .borrow_mut()
          .push(format!("child setup {dependency}"));
        let cleanup_log = Rc::clone(&setup_log);
        move || {
          cleanup_log
            .borrow_mut()
            .push(format!("child cleanup {dependency}"));
        }
      },
      dependency,
    );
    battlement_reactant::host::Label::new("child")
  }
}

struct PanicEffect;

struct RetriedEffect {
  setups: Rc<Cell<usize>>,
}

impl Component for PanicEffect {
  fn render(&self) -> impl Render {
    use_effect(
      || -> () {
        panic!("effect failed");
      },
      (),
    );
    battlement_reactant::host::Label::new("committed")
  }
}

impl Component for RetriedEffect {
  fn render(&self) -> impl Render {
    let (retried, set_retried) = use_state(false);
    if !retried {
      set_retried.set(true);
    }
    let setups = Rc::clone(&self.setups);
    use_effect(move || setups.set(setups.get() + 1), ());
    battlement_reactant::host::Label::new("retried")
  }
}

#[derive(Default)]
struct OrderGame {
  dependency: u8,
  show_child: bool,
  show_parent: bool,
}

#[test]
fn effect_state_joins_the_next_render_after_the_host_commit() {
  let document = self::document();
  let host_committed = Rc::new(Cell::new(false));
  let observed_commit = Rc::new(Cell::new(false));
  let mut reactant = Reactant::new(IdleSpawner);
  let view_host_committed = Rc::clone(&host_committed);
  let view_observed_commit = Rc::clone(&observed_commit);
  reactant.register_root(document.clone(), move |_: &()| ReadyEffect {
    host_committed: Rc::clone(&view_host_committed),
    observed_commit: Rc::clone(&view_observed_commit),
  });
  let initial = self::begin(&mut reactant, &mut (), &document);
  let label = initial.ui[0].children[0].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).expect("initial UI is valid");
  assert_eq!(world.element(label).unwrap().text(), Some("waiting"));
  assert!(!observed_commit.get(), "session conversion defers effects");

  host_committed.set(true);
  self::apply(
    &mut world,
    reactant.poll(&mut ()).expect("effect poll renders"),
  );

  assert!(
    observed_commit.get(),
    "setup observes the applied host commit"
  );
  assert_eq!(world.element(label).unwrap().text(), Some("ready"));
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn unit_dependencies_mount_once_while_always_runs_after_each_commit() {
  let document = self::document();
  let log = Rc::new(RefCell::new(Vec::new()));
  let view_log = Rc::clone(&log);
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), move |_: &()| FrequencyEffects {
    log: Rc::clone(&view_log),
  });
  let _ = self::begin(&mut reactant, &mut (), &document);

  assert!(log.borrow().is_empty());
  assert!(reactant.poll(&mut ()).unwrap().is_empty());
  assert_eq!(&*log.borrow(), &["mount-only", "always"]);
  assert!(reactant.refresh(&mut ()).unwrap().is_empty());
  assert_eq!(&*log.borrow(), &["mount-only", "always"]);
  assert!(reactant.poll(&mut ()).unwrap().is_empty());
  assert_eq!(&*log.borrow(), &["mount-only", "always", "always"]);
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn replacements_and_unmounts_clean_up_children_before_parents() {
  let document = self::document();
  let log = Rc::new(RefCell::new(Vec::new()));
  let view_log = Rc::clone(&log);
  let mut game = OrderGame {
    show_child: true,
    show_parent: true,
    ..OrderGame::default()
  };
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), move |game: &OrderGame| {
    game.show_parent.then(|| OrderedParent {
      dependency: game.dependency,
      show_child: game.show_child,
      log: Rc::clone(&view_log),
    })
  });
  let _ = self::begin(&mut reactant, &mut game, &document);
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  assert_eq!(&*log.borrow(), &["child setup 0", "parent setup 0"]);

  game.dependency = 1;
  assert!(reactant.refresh(&mut game).unwrap().is_empty());
  assert_eq!(
    log.borrow().len(),
    2,
    "the current commit does not run effects"
  );
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  assert_eq!(
    &*log.borrow(),
    &[
      "child setup 0",
      "parent setup 0",
      "child cleanup 0",
      "child setup 1",
      "parent cleanup 0",
      "parent setup 1",
    ]
  );

  game.show_child = false;
  let child_unmount = reactant.refresh(&mut game).unwrap();
  assert!(!child_unmount.is_empty());
  let _ = child_unmount.into_groups();
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  assert_eq!(log.borrow().last().unwrap(), "child cleanup 1");

  game.show_parent = false;
  let parent_unmount = reactant.refresh(&mut game).unwrap();
  assert!(!parent_unmount.is_empty());
  let _ = parent_unmount.into_groups();
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  assert_eq!(log.borrow().last().unwrap(), "parent cleanup 1");
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn reconnect_defers_effects_and_shutdown_flushes_replacement_and_final_cleanup() {
  let document = self::document();
  let log = Rc::new(RefCell::new(Vec::new()));
  let view_log = Rc::clone(&log);
  let mut game = OrderGame {
    show_parent: true,
    ..OrderGame::default()
  };
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), move |game: &OrderGame| OrderedParent {
    dependency: game.dependency,
    show_child: false,
    log: Rc::clone(&view_log),
  });
  let _ = self::begin(&mut reactant, &mut game, &document);
  let _ = self::begin(&mut reactant, &mut game, &document);
  assert!(log.borrow().is_empty(), "reconnect does not flush setup");
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  assert_eq!(&*log.borrow(), &["parent setup 0"]);

  game.dependency = 1;
  assert!(reactant.refresh(&mut game).unwrap().is_empty());
  let shutdown = reactant.shutdown(&mut game);
  assert!(!shutdown.is_empty());
  let _ = shutdown.into_groups();
  assert_eq!(
    &*log.borrow(),
    &[
      "parent setup 0",
      "parent cleanup 0",
      "parent setup 1",
      "parent cleanup 1",
    ]
  );
  assert!(reactant.shutdown(&mut game).is_empty());
}

#[test]
fn effect_panics_poison_before_the_next_host_commit() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| PanicEffect);
  let initial = self::begin(&mut reactant, &mut (), &document);
  let label = initial.ui[0].children[0].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).expect("initial UI is valid");

  let failed = panic::catch_unwind(AssertUnwindSafe(|| reactant.refresh(&mut ())))
    .err()
    .expect("effect setup should panic");
  assert_eq!(self::panic_message(failed), "effect failed");
  assert_eq!(world.element(label).unwrap().text(), Some("committed"));
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut ()))).is_err());
}

#[test]
fn a_mount_effect_survives_a_render_phase_retry() {
  let document = self::document();
  let setups = Rc::new(Cell::new(0));
  let view_setups = Rc::clone(&setups);
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), move |_: &()| RetriedEffect {
    setups: Rc::clone(&view_setups),
  });
  let _ = self::begin(&mut reactant, &mut (), &document);

  assert!(reactant.poll(&mut ()).unwrap().is_empty());
  assert_eq!(setups.get(), 1);
  let _ = reactant.shutdown(&mut ()).into_groups();
}

fn begin<G: 'static>(reactant: &mut Reactant<G>, game: &mut G, document: &UiDocument) -> Snapshot {
  reactant
    .begin_session(game)
    .unwrap()
    .into_parts(self::snapshot(document))
    .0
}

fn apply(world: &mut UiWorld, commit: ReactantCommit) {
  for body in commit.into_groups().into_iter().flatten() {
    match body {
      CommandBody::VisualElementCreate(value) => world.create(*value).unwrap(),
      CommandBody::VisualElementUpdate(value) => world.update(*value).unwrap(),
      CommandBody::VisualElementDestroy(value) => world.destroy(value.object_id).unwrap(),
      _ => panic!("Reactant emitted a non-UI command"),
    }
  }
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(document: &UiDocument) -> Snapshot {
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(scene_id, "test/scene")],
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

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
  payload.downcast_ref::<&str>().map_or_else(
    || payload.downcast_ref::<String>().cloned().unwrap(),
    |message| (*message).to_owned(),
  )
}
