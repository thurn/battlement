use std::{
  cell::{Cell, RefCell},
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{
  CameraState, CommandBody, GameObject, GameObjectKind, Label, ObjectId, PanelScaleMode,
  PanelSettings, ParentScene, PreparedAsset, Prop, Scene, SceneId, SessionId, Snapshot, UiDocument,
  UiDocumentState, UiElement,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  component::{self, Component},
  context::{Context, RequiredContext},
  executor::{BoxFuture, SpawnedTask, Spawner},
  hooks::{self, Callback, StateSetter},
  render::Render,
  runtime::{Reactant, ReactantCommit},
};

static THEME: Context<&'static str> = Context::new(|| "default");
static REQUIRED_THEME: RequiredContext<&'static str> = RequiredContext::new();

type MemoCallbacks = Rc<RefCell<Vec<Callback<Box<dyn Fn() -> u8>>>>>;

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

struct Game {
  unrelated: u8,
  prop: u8,
  dependency: u8,
  theme: &'static str,
}

struct MemoFixture {
  prop: u8,
  dependency: u8,
  renders: Rc<Cell<usize>>,
  calculations: Rc<Cell<usize>>,
  callbacks: MemoCallbacks,
  setter: Rc<RefCell<Option<StateSetter<u8>>>>,
}

impl PartialEq for MemoFixture {
  fn eq(&self, other: &Self) -> bool {
    self.prop == other.prop && self.dependency == other.dependency
  }
}

impl Component for MemoFixture {
  fn render(&self) -> impl Render {
    self.renders.set(self.renders.get() + 1);
    let calculations = Rc::clone(&self.calculations);
    let dependency = self.dependency;
    let memoized = hooks::use_memo(
      move || {
        calculations.set(calculations.get() + 1);
        dependency * 10
      },
      self.dependency,
    );
    let captured = self.dependency;
    let callback = hooks::use_callback(
      Box::new(move || captured) as Box<dyn Fn() -> u8>,
      self.dependency,
    );
    self.callbacks.borrow_mut().push(callback);
    let (local, setter) = hooks::use_state(0_u8);
    self.setter.replace(Some(setter));
    Label::new(format!(
      "{}/{}/{}/{}",
      self.prop,
      memoized,
      hooks::use_context(&THEME),
      local
    ))
  }
}

struct InvalidMemo;

struct ProviderBoundary {
  renders: Rc<Cell<usize>>,
}

struct RequiredConsumer;

impl PartialEq for ProviderBoundary {
  fn eq(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.renders, &other.renders)
  }
}

impl Component for ProviderBoundary {
  fn render(&self) -> impl Render {
    self.renders.set(self.renders.get() + 1);
    REQUIRED_THEME.provider("inner").child(RequiredConsumer)
  }
}

impl Component for RequiredConsumer {
  fn render(&self) -> impl Render {
    Label::new(hooks::use_required_context(&REQUIRED_THEME))
  }
}

impl Component for InvalidMemo {
  fn render(&self) -> impl Render {
    let _ = hooks::use_memo(|| hooks::use_ref(0_u8), ());
    Label::new("invalid")
  }
}

#[test]
fn memo_bailout_observes_props_dependencies_context_and_local_work() {
  let renders = Rc::new(Cell::new(0));
  let calculations = Rc::new(Cell::new(0));
  let callbacks = Rc::new(RefCell::new(Vec::new()));
  let setter = Rc::new(RefCell::new(None));
  let document = self::document();
  let mut game = Game {
    unrelated: 0,
    prop: 0,
    dependency: 0,
    theme: "light",
  };
  let mut reactant = Reactant::new(IdleSpawner);
  let view_renders = Rc::clone(&renders);
  let view_calculations = Rc::clone(&calculations);
  let view_callbacks = Rc::clone(&callbacks);
  let view_setter = Rc::clone(&setter);
  reactant.register_root(document.clone(), move |game: &Game| {
    (
      Label::new(format!("unrelated {}", game.unrelated)),
      THEME
        .provider(game.theme)
        .child(component::memo(MemoFixture {
          prop: game.prop,
          dependency: game.dependency,
          renders: Rc::clone(&view_renders),
          calculations: Rc::clone(&view_calculations),
          callbacks: Rc::clone(&view_callbacks),
          setter: Rc::clone(&view_setter),
        })),
    )
  });

  let initial = self::begin(&mut reactant, &mut game, &document);
  let memo_host = initial.ui[0].children[1].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  assert_eq!(
    self::texts(&world, document.root_id),
    ["unrelated 0", "0/0/light/0"]
  );
  assert_eq!((renders.get(), calculations.get()), (1, 1));

  game.unrelated = 1;
  let commands = self::commands(reactant.refresh(&mut game).unwrap());
  assert_eq!(commands.len(), 1);
  let CommandBody::VisualElementUpdate(update) = &commands[0] else {
    panic!("unrelated parent state should only update its label");
  };
  assert_ne!(update.object_id(), memo_host);
  self::apply(&mut world, &commands);
  assert_eq!(
    world.element(document.root_id).unwrap().children()[1],
    memo_host
  );
  assert_eq!(
    self::texts(&world, document.root_id),
    ["unrelated 1", "0/0/light/0"]
  );
  assert_eq!((renders.get(), calculations.get()), (1, 1));

  game.prop = 1;
  self::apply(
    &mut world,
    &self::commands(reactant.refresh(&mut game).unwrap()),
  );
  assert_eq!(
    self::texts(&world, document.root_id),
    ["unrelated 1", "1/0/light/0"]
  );
  assert_eq!((renders.get(), calculations.get()), (2, 1));
  assert!(callbacks.borrow()[0] == callbacks.borrow()[1]);

  game.dependency = 1;
  self::apply(
    &mut world,
    &self::commands(reactant.refresh(&mut game).unwrap()),
  );
  assert_eq!(
    self::texts(&world, document.root_id),
    ["unrelated 1", "1/10/light/0"]
  );
  assert_eq!((renders.get(), calculations.get()), (3, 2));
  assert!(callbacks.borrow()[1] != callbacks.borrow()[2]);
  assert_eq!((callbacks.borrow()[1])(), 0);
  assert_eq!((callbacks.borrow()[2])(), 1);

  game.theme = "dark";
  self::apply(
    &mut world,
    &self::commands(reactant.refresh(&mut game).unwrap()),
  );
  assert_eq!(
    self::texts(&world, document.root_id),
    ["unrelated 1", "1/10/dark/0"]
  );
  assert_eq!((renders.get(), calculations.get()), (4, 2));
  assert!(callbacks.borrow()[2] == callbacks.borrow()[3]);

  setter
    .borrow()
    .clone()
    .expect("state setter should render")
    .set(7);
  self::apply(
    &mut world,
    &self::commands(reactant.poll(&mut game).unwrap()),
  );
  assert_eq!(
    self::texts(&world, document.root_id),
    ["unrelated 1", "1/10/dark/7"]
  );
  assert_eq!((renders.get(), calculations.get()), (5, 2));
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn memo_calculations_forbid_hooks_and_panics_poison_the_runtime() {
  let document = self::document();
  let mut game = Game {
    unrelated: 0,
    prop: 0,
    dependency: 0,
    theme: "light",
  };
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document, |_| InvalidMemo);
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _ = reactant.begin_session(&mut game);
    }))
    .is_err()
  );
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _ = reactant.begin_session(&mut game);
    }))
    .is_err()
  );
}

#[test]
fn memo_bailout_preserves_provider_ancestry_for_required_contexts() {
  let renders = Rc::new(Cell::new(0));
  let document = self::document();
  let mut game = Game {
    unrelated: 0,
    prop: 0,
    dependency: 0,
    theme: "light",
  };
  let mut reactant = Reactant::new(IdleSpawner);
  let view_renders = Rc::clone(&renders);
  reactant.register_root(document.clone(), move |game: &Game| {
    (
      Label::new(format!("unrelated {}", game.unrelated)),
      component::memo(ProviderBoundary {
        renders: Rc::clone(&view_renders),
      }),
    )
  });
  let initial = self::begin(&mut reactant, &mut game, &document);
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  assert_eq!(
    self::texts(&world, document.root_id),
    ["unrelated 0", "inner"]
  );

  game.unrelated = 1;
  self::apply(
    &mut world,
    &self::commands(reactant.refresh(&mut game).unwrap()),
  );
  assert_eq!(
    self::texts(&world, document.root_id),
    ["unrelated 1", "inner"]
  );
  assert_eq!(renders.get(), 1);
  let _ = reactant.shutdown(&mut game).into_groups();
}

fn begin(reactant: &mut Reactant<Game>, game: &mut Game, document: &UiDocument) -> Snapshot {
  reactant
    .begin_session(game)
    .unwrap()
    .into_parts(self::snapshot(document))
    .0
}

fn commands(commit: ReactantCommit) -> Vec<CommandBody> {
  commit.into_groups().into_iter().flatten().collect()
}

fn apply(world: &mut UiWorld, commands: &[CommandBody]) {
  for command in commands {
    match command {
      CommandBody::VisualElementCreate(value) => world.create(*value.clone()).unwrap(),
      CommandBody::VisualElementUpdate(value) => world.update(*value.clone()).unwrap(),
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
    .flat_map(|child| self::node_texts(world, *child))
    .collect()
}

fn node_texts(world: &UiWorld, node: ObjectId) -> Vec<&str> {
  let element = world.element(node).unwrap();
  let text = match element.element() {
    UiElement::Label(label) => match &label.text {
      Prop::Set(value) => Some(value.as_str()),
      Prop::Unset | Prop::Reset => None,
    },
    _ => None,
  };
  text
    .into_iter()
    .chain(
      element
        .children()
        .iter()
        .flat_map(|child| self::node_texts(world, *child)),
    )
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
