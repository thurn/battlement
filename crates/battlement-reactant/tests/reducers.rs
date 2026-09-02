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
  component::Component,
  executor::{BoxFuture, SpawnedTask, Spawner},
  hooks::{self, ReducerDispatch},
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
  generation: u8,
  order: Vec<u8>,
  step: i32,
}

#[derive(Clone)]
enum CountAction {
  Add,
  Set(i32),
}

#[derive(Clone)]
struct Counter {
  dispatch: Rc<RefCell<Option<ReducerDispatch<CountAction>>>>,
  initializations: Rc<Cell<usize>>,
  renders: Rc<Cell<usize>>,
  step: i32,
}

impl Component for Counter {
  fn render(&self) -> impl Render {
    self.renders.set(self.renders.get() + 1);
    let initializations = Rc::clone(&self.initializations);
    let step = self.step;
    let (count, dispatch) = hooks::use_reducer_with(
      move |state, action| match action {
        CountAction::Add => state + step,
        CountAction::Set(value) => value,
      },
      move || {
        initializations.set(initializations.get() + 1);
        0
      },
    );
    self.dispatch.replace(Some(dispatch.clone()));
    (
      battlement_reactant::host::Button::new("Reduce").on_click(move |_game: &mut Game| {
        dispatch.send(CountAction::Add);
        dispatch.send(CountAction::Set(10));
        dispatch.send(CountAction::Add);
      }),
      battlement_reactant::host::Label::new(format!("Count {count}")),
    )
  }
}

#[derive(Clone)]
struct KeyedCounter {
  dispatches: Rc<RefCell<Vec<Option<ReducerDispatch<u8>>>>>,
  id: u8,
}

impl Component for KeyedCounter {
  fn render(&self) -> impl Render {
    let (count, dispatch) = hooks::use_reducer(|state, action| state + action, 0_u8);
    self.dispatches.borrow_mut()[usize::from(self.id)] = Some(dispatch);
    battlement_reactant::host::Label::new(format!("{}:{count}", self.id))
  }
}

#[derive(Clone)]
struct FailingReducer {
  dispatch: Rc<RefCell<Option<ReducerDispatch<()>>>>,
}

#[derive(Clone)]
struct HookingReducer {
  dispatch: Rc<RefCell<Option<ReducerDispatch<()>>>>,
}

impl Component for FailingReducer {
  fn render(&self) -> impl Render {
    let (value, dispatch) = hooks::use_reducer(
      |_state, ()| -> u8 {
        panic!("reducer failed");
      },
      0,
    );
    self.dispatch.replace(Some(dispatch));
    battlement_reactant::host::Label::new(value.to_string())
  }
}

impl Component for HookingReducer {
  fn render(&self) -> impl Render {
    let (value, dispatch) = hooks::use_reducer(
      |state, ()| {
        let _ = hooks::use_state(0_u8);
        *state
      },
      0_u8,
    );
    self.dispatch.replace(Some(dispatch));
    battlement_reactant::host::Label::new(value.to_string())
  }
}

struct RenderPhaseReducer;

impl Component for RenderPhaseReducer {
  fn render(&self) -> impl Render {
    let (value, dispatch) = hooks::use_reducer(|state, ()| state + 1, 0_u8);
    if value < 3 {
      dispatch.send(());
    }
    battlement_reactant::host::Label::new(format!("Reduced {value}"))
  }
}

#[derive(Clone)]
struct VariableKind {
  reducer: bool,
}

impl Component for VariableKind {
  fn render(&self) -> impl Render {
    if self.reducer {
      let _: (u8, ReducerDispatch<()>) = hooks::use_reducer(|state, ()| *state, 0);
    } else {
      let _ = hooks::use_state(0_u8);
    }
    battlement_reactant::host::Label::new("stable")
  }
}

#[test]
fn clicks_batch_ordered_actions_and_the_current_render_supplies_the_reducer() {
  let dispatch = Rc::new(RefCell::new(None));
  let initializations = Rc::new(Cell::new(0));
  let renders = Rc::new(Cell::new(0));
  let document = self::document();
  let mut game = Game {
    step: 1,
    ..Game::default()
  };
  let mut reactant = Reactant::new(IdleSpawner);
  let view_dispatch = Rc::clone(&dispatch);
  let view_initializations = Rc::clone(&initializations);
  let view_renders = Rc::clone(&renders);
  reactant.register_root(document.clone(), move |game: &Game| Counter {
    dispatch: Rc::clone(&view_dispatch),
    initializations: Rc::clone(&view_initializations),
    renders: Rc::clone(&view_renders),
    step: game.step,
  });
  let initial = self::begin(&mut reactant, &mut game, &document);
  let button = initial.ui[0].children[0].object_id;
  let label = initial.ui[0].children[1].object_id;
  let first_dispatch = dispatch.borrow().clone().unwrap();
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
  assert_eq!(renders.get(), 2, "one click produces one reduced render");
  assert_eq!(initializations.get(), 1);
  assert!(first_dispatch == dispatch.borrow().clone().unwrap());

  first_dispatch.send(CountAction::Add);
  game.step = 4;
  self::apply(&mut world, reactant.poll(&mut game).unwrap());
  assert_eq!(world.element(label).unwrap().text(), Some("Count 15"));
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn keyed_reorder_preserves_reducer_state_and_changed_identity_resets_it() {
  let dispatches = Rc::new(RefCell::new(vec![None, None, None]));
  let document = self::document();
  let mut game = Game {
    order: vec![1, 2],
    ..Game::default()
  };
  let mut reactant = Reactant::new(IdleSpawner);
  let view_dispatches = Rc::clone(&dispatches);
  reactant.register_root(document.clone(), move |game: &Game| {
    game
      .order
      .iter()
      .map(|id| {
        KeyedCounter {
          dispatches: Rc::clone(&view_dispatches),
          id: *id,
        }
        .key((game.generation, *id))
      })
      .collect::<Vec<_>>()
  });
  let initial = self::begin(&mut reactant, &mut game, &document);
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  let first = dispatches.borrow()[1].clone().unwrap();
  first.send(7);
  self::apply(&mut world, reactant.poll(&mut game).unwrap());
  assert_eq!(self::texts(&world, document.root_id), ["1:7", "2:0"]);

  game.order.reverse();
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  assert_eq!(self::texts(&world, document.root_id), ["2:0", "1:7"]);
  assert!(first == dispatches.borrow()[1].clone().unwrap());

  game.generation += 1;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  assert_eq!(self::texts(&world, document.root_id), ["2:0", "1:0"]);
  assert!(first != dispatches.borrow()[1].clone().unwrap());
  first.send(9);
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn render_phase_actions_retry_and_reducer_failure_and_kind_changes_poison() {
  let document = self::document();
  let mut game = Game::default();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_| RenderPhaseReducer);
  let initial = self::begin(&mut reactant, &mut game, &document);
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  assert_eq!(self::texts(&world, document.root_id), ["Reduced 3"]);

  let dispatch = Rc::new(RefCell::new(None));
  let failing_document = self::document();
  let mut failing = Reactant::new(IdleSpawner);
  let view_dispatch = Rc::clone(&dispatch);
  failing.register_root(failing_document.clone(), move |_| FailingReducer {
    dispatch: Rc::clone(&view_dispatch),
  });
  let original = self::begin(&mut failing, &mut game, &failing_document);
  let mut failing_world = UiWorld::default();
  failing_world.replace(original.ui).unwrap();
  dispatch.borrow().clone().unwrap().send(());
  assert!(panic::catch_unwind(AssertUnwindSafe(|| failing.poll(&mut game))).is_err());
  assert_eq!(self::texts(&failing_world, failing_document.root_id), ["0"]);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| failing.refresh(&mut game))).is_err());

  let hooking_dispatch = Rc::new(RefCell::new(None));
  let hooking_document = self::document();
  let mut hooking = Reactant::new(IdleSpawner);
  let view_dispatch = Rc::clone(&hooking_dispatch);
  hooking.register_root(hooking_document.clone(), move |_| HookingReducer {
    dispatch: Rc::clone(&view_dispatch),
  });
  let _ = self::begin(&mut hooking, &mut game, &hooking_document);
  hooking_dispatch.borrow().clone().unwrap().send(());
  assert!(panic::catch_unwind(AssertUnwindSafe(|| hooking.poll(&mut game))).is_err());

  let reducer = Rc::new(Cell::new(false));
  let view_reducer = Rc::clone(&reducer);
  let kind_document = self::document();
  let mut kind = Reactant::new(IdleSpawner);
  kind.register_root(kind_document.clone(), move |_| VariableKind {
    reducer: view_reducer.get(),
  });
  let _ = self::begin(&mut kind, &mut game, &kind_document);
  reducer.set(true);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| kind.refresh(&mut game))).is_err());
  let _ = reactant.shutdown(&mut game).into_groups();
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
        GameObjectKind::UiDocument(
          UiDocumentState::new(document.root_id)
            .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantPixelSize)),
        ),
      )
      .parent_scene(ParentScene::Persistent),
    ],
    camera_id,
  )
}
