mod runtime_support;

use std::{
  cell::{Cell, RefCell},
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{
  CameraState, ClickEvent, CommandBody, GameObject, GameObjectKind, ObjectId, PanelScaleMode,
  PanelSettings, ParentScene, PreparedAsset, Scene, SceneId, SessionId, Snapshot, UiDocument,
  UiDocumentState, UiEvent,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  component::{Component, RenderCallback},
  executor::{BoxFuture, SpawnedTask, Spawner},
  hooks::{StateSetter, use_state, use_state_with},
  key::KeyRenderExt,
  render::Render,
  runtime::{Reactant, ReactantCommit},
};
struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[derive(Default)]
struct Game {
  order: Vec<u8>,
  visible: bool,
}

#[derive(Clone)]
struct Counter {
  initializations: Rc<Cell<usize>>,
  renders: Rc<Cell<usize>>,
  setter: Rc<RefCell<Option<StateSetter<i32>>>>,
}

impl Component for Counter {
  fn render(&self) -> impl Render {
    self.renders.set(self.renders.get() + 1);
    let initializations = Rc::clone(&self.initializations);
    let (count, setter) = use_state_with(move || {
      initializations.set(initializations.get() + 1);
      0
    });
    self.setter.replace(Some(setter.clone()));
    (
      battlement_reactant::host::Button::new(trox::assert_localized("Queue updates")).on_click(
        move |_game: &mut Game| {
          setter.update(|value| value + 1);
          setter.set(10);
          setter.update(|value| value + 1);
        },
      ),
      battlement_reactant::host::Label::new(trox::assert_localized(format!("Count {count}"))),
    )
  }
}

#[derive(Clone)]
struct KeyedCounter {
  id: u8,
  setters: Rc<RefCell<Vec<Option<StateSetter<u8>>>>>,
}

impl Component for KeyedCounter {
  fn render(&self) -> impl Render {
    let (count, setter) = use_state(0_u8);
    self.setters.borrow_mut()[usize::from(self.id)] = Some(setter);
    battlement_reactant::host::Label::new(trox::assert_localized(format!("{}:{count}", self.id)))
  }
}

struct RenderPhaseCounter;

impl Component for RenderPhaseCounter {
  fn render(&self) -> impl Render {
    let (count, setter) = use_state(0);
    if count < 3 {
      setter.update(|value| value + 1);
    }
    battlement_reactant::host::Label::new(trox::assert_localized(format!("Retried {count}")))
  }
}

struct Overflow;

struct CallbackCounter;

impl Component for Overflow {
  fn render(&self) -> impl Render {
    let (count, setter) = use_state(0);
    setter.update(|value| value + 1);
    battlement_reactant::host::Label::new(trox::assert_localized(count.to_string()))
  }
}

impl Component for CallbackCounter {
  fn render(&self) -> impl Render {
    let (count, setter) = use_state(0_u32);
    (
      battlement_reactant::host::Button::new(trox::assert_localized("Increment"))
        .on_click(setter.update_callback(|value| value + 1)),
      battlement_reactant::host::Button::new(trox::assert_localized("Replace"))
        .on_click(setter.callback().map_input(|()| 12)),
      battlement_reactant::host::Label::new(trox::assert_localized(count.to_string())),
    )
  }
}

#[derive(Clone)]
struct VariableHooks {
  second: bool,
  alternate_type: bool,
}

impl Component for VariableHooks {
  fn render(&self) -> impl Render {
    if self.alternate_type {
      let _ = use_state(0_u16);
    } else {
      let _ = use_state(0_u8);
    }
    if self.second {
      let _ = use_state(1_u8);
    }
    battlement_reactant::host::Label::new(trox::assert_localized("stable"))
  }
}

struct BadInitializer;

impl Component for BadInitializer {
  fn render(&self) -> impl Render {
    let _ = use_state_with(|| {
      let _ = use_state(0);
      0
    });
    battlement_reactant::host::Label::new(trox::assert_localized("invalid"))
  }
}

struct ParentUpdate;

struct ChildUpdate {
  parent: StateSetter<u8>,
}

impl Component for ParentUpdate {
  fn render(&self) -> impl Render {
    let (_, setter) = use_state(0_u8);
    ChildUpdate { parent: setter }
  }
}

impl Component for ChildUpdate {
  fn render(&self) -> impl Render {
    self.parent.set(1);
    battlement_reactant::host::Label::new(trox::assert_localized("invalid"))
  }
}

#[test]
fn event_updates_batch_in_order_and_lazy_state_and_setters_are_stable() {
  let initializations = Rc::new(Cell::new(0));
  let renders = Rc::new(Cell::new(0));
  let setter = Rc::new(RefCell::new(None));
  let counter = Counter {
    initializations: Rc::clone(&initializations),
    renders: Rc::clone(&renders),
    setter: Rc::clone(&setter),
  };
  let document = self::document();
  let mut game = Game::default();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), move |_| counter.clone());
  let initial = self::begin(&mut reactant, &mut game, &document);
  let button = initial.ui[0].children[0].object_id;
  let label = initial.ui[0].children[1].object_id;
  let first_setter = setter.borrow().clone().expect("setter was rendered");
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();

  self::apply(
    &mut world,
    reactant
      .dispatch(
        &mut game,
        UiEvent::click(button, ClickEvent::NavigationSubmit),
      )
      .unwrap()
      .into_commit(),
  );

  assert_eq!(world.element(label).unwrap().text(), Some("Count 11"));
  assert_eq!(renders.get(), 2, "one event produces one refresh");
  assert_eq!(initializations.get(), 1);
  assert!(first_setter == setter.borrow().clone().expect("setter remains rendered"));

  first_setter.set(11);
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  assert_eq!(renders.get(), 2, "equal state does not rerender");
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn state_callback_factories_remain_live_across_renders() {
  let document = self::document();
  let mut game = Game::default();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), |_| CallbackCounter);
  let initial = self::begin(&mut reactant, &mut game, &document);
  let increment = initial.ui[0].children[0].object_id;
  let replace = initial.ui[0].children[1].object_id;
  let label = initial.ui[0].children[2].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();

  for expected in ["1", "2"] {
    self::apply(
      &mut world,
      reactant
        .dispatch(
          &mut game,
          UiEvent::click(increment, ClickEvent::NavigationSubmit),
        )
        .unwrap()
        .into_commit(),
    );
    assert_eq!(world.element(label).unwrap().text(), Some(expected));
  }
  self::apply(
    &mut world,
    reactant
      .dispatch(
        &mut game,
        UiEvent::click(replace, ClickEvent::NavigationSubmit),
      )
      .unwrap()
      .into_commit(),
  );
  assert_eq!(world.element(label).unwrap().text(), Some("12"));
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn queued_updates_poll_and_keyed_state_follows_identity_while_unmounted_setters_noop() {
  let setters = Rc::new(RefCell::new(vec![None, None, None]));
  let document = self::document();
  let mut game = Game {
    order: vec![1, 2],
    visible: true,
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let view_setters = Rc::clone(&setters);
  reactant.register_root(document.clone(), move |game: &Game| {
    game.visible.then(|| {
      game
        .order
        .iter()
        .map(|id| {
          KeyedCounter {
            id: *id,
            setters: Rc::clone(&view_setters),
          }
          .key(*id)
        })
        .collect::<Vec<_>>()
    })
  });
  let initial = self::begin(&mut reactant, &mut game, &document);
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  let first = setters.borrow()[1].clone().unwrap();
  first.set(7);
  self::apply(&mut world, reactant.poll(&mut game).unwrap());
  assert_eq!(self::texts(&world, document.root_id), ["1:7", "2:0"]);

  game.order.reverse();
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  assert_eq!(self::texts(&world, document.root_id), ["2:0", "1:7"]);
  assert!(first == setters.borrow()[1].clone().unwrap());

  game.visible = false;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  first.set(99);
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn render_phase_updates_retry_locally_and_overflow_poisons() {
  let document = self::document();
  let mut game = Game::default();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), |_| RenderPhaseCounter);
  let initial = self::begin(&mut reactant, &mut game, &document);
  let label = initial.ui[0].children[0].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  assert_eq!(world.element(label).unwrap().text(), Some("Retried 3"));

  let overflow_document = self::document();
  let mut overflow = runtime_support::reactant(IdleSpawner);
  overflow.register_root(overflow_document.clone(), |_| Overflow);
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _ = overflow.begin_session(&mut game);
    }))
    .is_err()
  );
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _ = overflow.begin_session(&mut game);
    }))
    .is_err()
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn invalid_hook_context_count_type_and_cross_component_updates_poison_transactionally() {
  assert!(panic::catch_unwind(|| use_state(0)).is_err());
  assert!(panic::catch_unwind(|| RenderCallback::new(|()| use_state(0)).call(())).is_err());

  let document = self::document();
  let second = Rc::new(Cell::new(false));
  let alternate_type = Rc::new(Cell::new(false));
  let second_view = Rc::clone(&second);
  let type_view = Rc::clone(&alternate_type);
  let mut game = Game::default();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), move |_| VariableHooks {
    second: second_view.get(),
    alternate_type: type_view.get(),
  });
  let initial = self::begin(&mut reactant, &mut game, &document);
  let original = initial.ui[0].children[0].object_id;
  second.set(true);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.refresh(&mut game))).is_err());
  assert_eq!(original, initial.ui[0].children[0].object_id);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.refresh(&mut game))).is_err());

  let type_document = self::document();
  let alternate_type = Rc::new(Cell::new(false));
  let type_view = Rc::clone(&alternate_type);
  let mut typed = runtime_support::reactant(IdleSpawner);
  typed.register_root(type_document.clone(), move |_| VariableHooks {
    second: false,
    alternate_type: type_view.get(),
  });
  let _ = self::begin(&mut typed, &mut game, &type_document);
  alternate_type.set(true);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| typed.refresh(&mut game))).is_err());

  let cross_document = self::document();
  let mut cross = runtime_support::reactant(IdleSpawner);
  cross.register_root(cross_document, |_| ParentUpdate);
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _session = cross.begin_session(&mut game);
    }))
    .is_err()
  );

  let initializer_document = self::document();
  let mut initializer = runtime_support::reactant(IdleSpawner);
  initializer.register_root(initializer_document, |_| BadInitializer);
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _session = initializer.begin_session(&mut game);
    }))
    .is_err()
  );
}

fn begin(reactant: &mut Reactant<Game>, game: &mut Game, document: &UiDocument) -> Snapshot {
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

fn texts(world: &UiWorld, root: ObjectId) -> Vec<&str> {
  world
    .element(root)
    .unwrap()
    .children()
    .iter()
    .map(|id| world.element(*id).unwrap().text().unwrap())
    .collect()
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
