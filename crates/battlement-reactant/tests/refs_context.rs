mod runtime_support;

use std::{
  cell::{Cell, RefCell},
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{
  CameraState, ClickEvent, GameObject, GameObjectKind, ObjectId, PanelScaleMode, PanelSettings,
  ParentScene, PreparedAsset, Prop, Scene, SceneId, SessionId, Snapshot, UiDocument,
  UiDocumentState, UiElement, UiEvent, UiNode,
};
use battlement_reactant::{
  component::Component,
  context::{Context, RequiredContext},
  executor::{BoxFuture, SpawnedTask, Spawner},
  hooks::{self, Ref, StateSetter},
  render::{Node, Render},
  runtime::Reactant,
};

thread_local! {
  static DEFAULT_CALLS: Cell<usize> = const { Cell::new(0) };
}

static PRIMARY: Context<&'static str> = Context::new(primary_default);
static SECONDARY: Context<&'static str> = Context::new(|| "secondary-default");
static REQUIRED: RequiredContext<&'static str> = RequiredContext::new();
static NUMBER: Context<u8> = Context::new(|| 0);

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[derive(Default)]
struct Game {
  ref_operation: Option<RefOperation>,
}

#[derive(Clone)]
struct RefFixture {
  initializations: Rc<Cell<usize>>,
  renders: Rc<Cell<usize>>,
  handle: Rc<RefCell<Option<Ref<Vec<u8>>>>>,
}

impl Component for RefFixture {
  fn render(&self) -> impl Render {
    self.renders.set(self.renders.get() + 1);
    let initializations = Rc::clone(&self.initializations);
    let reference = hooks::use_ref_with(move || {
      initializations.set(initializations.get() + 1);
      Vec::new()
    });
    self.handle.replace(Some(reference.clone()));
    battlement_reactant::host::Button::new(trox::assert_localized("mutate ref")).on_click(
      move |_game: &mut Game| {
        reference.with_mut(|values| values.push(2));
      },
    )
  }
}

struct InvalidRefAccess {
  operation: Option<RefOperation>,
}

impl Component for InvalidRefAccess {
  fn render(&self) -> impl Render {
    let reference = hooks::use_ref_with(Vec::<u8>::new);
    match self.operation {
      Some(RefOperation::Get) => drop(reference.get()),
      Some(RefOperation::Replace) => drop(reference.replace(vec![1])),
      Some(RefOperation::With) => reference.with(|value| assert!(value.is_empty())),
      Some(RefOperation::WithMut) => reference.with_mut(|value| value.push(1)),
      None => {}
    }
    battlement_reactant::host::Label::new(trox::assert_localized("ref"))
  }
}

#[derive(Clone, Copy)]
enum RefOperation {
  Get,
  Replace,
  With,
  WithMut,
}

struct ContextConsumer {
  name: &'static str,
}

impl Component for ContextConsumer {
  fn render(&self) -> impl Render {
    battlement_reactant::host::Label::new(trox::assert_localized(format!(
      "{}:{}/{}",
      self.name,
      hooks::use_context(&PRIMARY),
      hooks::use_context(&SECONDARY)
    )))
  }
}

struct RequiredConsumer;

impl Component for RequiredConsumer {
  fn render(&self) -> impl Render {
    battlement_reactant::host::Label::new(trox::assert_localized(hooks::use_required_context(
      &REQUIRED,
    )))
  }
}

struct NestedRuntime;

impl Component for NestedRuntime {
  fn render(&self) -> impl Render {
    let document = self::document();
    let mut game = Game::default();
    let mut reactant = runtime_support::reactant(IdleSpawner);
    reactant.register_root(document.clone(), |_| ContextConsumer { name: "nested" });
    let text = self::texts(
      &self::begin(&mut reactant, &mut game, &document),
      document.root_id,
    )
    .join("");
    let _ = reactant.shutdown(&mut game).into_groups();
    battlement_reactant::host::Label::new(trox::assert_localized(text))
  }
}

#[derive(Clone)]
struct StatefulConsumer {
  setter: Rc<RefCell<Option<StateSetter<u8>>>>,
}

impl Component for StatefulConsumer {
  fn render(&self) -> impl Render {
    let (value, setter) = hooks::use_state(0_u8);
    self.setter.replace(Some(setter));
    battlement_reactant::host::Label::new(trox::assert_localized(value.to_string()))
  }
}

#[test]
fn ref_mutation_survives_without_pending_render_work_and_handles_are_stable() {
  let initializations = Rc::new(Cell::new(0));
  let renders = Rc::new(Cell::new(0));
  let handle = Rc::new(RefCell::new(None));
  let fixture = RefFixture {
    initializations: Rc::clone(&initializations),
    renders: Rc::clone(&renders),
    handle: Rc::clone(&handle),
  };
  let document = self::document();
  let mut game = Game::default();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), move |_| fixture.clone());
  let snapshot = self::begin(&mut reactant, &mut game, &document);
  let button = snapshot.ui[0].children[0].object_id;
  let first = handle.borrow().clone().expect("ref should render");

  let callback_ref = first.clone();
  let callback = move || callback_ref.with_mut(|values| values.push(1));
  callback();
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  assert_eq!(renders.get(), 1);
  assert_eq!(first.get(), vec![1]);

  let _ = reactant
    .dispatch(
      &mut game,
      UiEvent::click(button, ClickEvent::NavigationSubmit),
    )
    .unwrap()
    .into_groups();
  assert_eq!(first.get(), vec![1, 2]);
  assert!(first == handle.borrow().clone().expect("ref should remain rendered"));
  assert_eq!(initializations.get(), 1);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn every_ref_value_operation_panics_during_render() {
  for operation in [
    RefOperation::Get,
    RefOperation::Replace,
    RefOperation::With,
    RefOperation::WithMut,
  ] {
    let document = self::document();
    let mut game = Game::default();
    let mut reactant = runtime_support::reactant(IdleSpawner);
    reactant.register_root(document.clone(), move |game: &Game| InvalidRefAccess {
      operation: game.ref_operation,
    });
    let _ = self::begin(&mut reactant, &mut game, &document);
    game.ref_operation = Some(operation);
    assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.refresh(&mut game))).is_err());
  }
}

#[test]
fn providers_use_nearest_logical_value_and_same_typed_contexts_do_not_alias() {
  DEFAULT_CALLS.set(0);
  let document = self::document();
  let mut game = Game::default();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), |_| {
    (
      ContextConsumer { name: "outside" },
      PRIMARY.provider("outer").child((
        ContextConsumer { name: "outer" },
        PRIMARY
          .provider("inner")
          .child(ContextConsumer { name: "inner" }),
        SECONDARY
          .provider("secondary")
          .child(ContextConsumer { name: "separate" }),
      )),
    )
  });
  let snapshot = self::begin(&mut reactant, &mut game, &document);
  assert_eq!(
    self::texts(&snapshot, document.root_id),
    [
      "outside:primary-default/secondary-default",
      "outer:outer/secondary-default",
      "inner:inner/secondary-default",
      "separate:outer/secondary",
    ]
  );
  assert_eq!(DEFAULT_CALLS.get(), 1);

  let _ = reactant
    .begin_session(&mut game)
    .unwrap()
    .into_parts(self::snapshot(&document));
  assert_eq!(
    DEFAULT_CALLS.get(),
    1,
    "reconnect reuses the runtime default"
  );

  let second_document = self::document();
  let mut second = runtime_support::reactant(IdleSpawner);
  second.register_root(second_document.clone(), |_| ContextConsumer { name: "new" });
  let _ = self::begin(&mut second, &mut game, &second_document);
  assert_eq!(
    DEFAULT_CALLS.get(),
    2,
    "a second runtime owns its own default"
  );
  let _ = reactant.shutdown(&mut game).into_groups();
  let _ = second.shutdown(&mut game).into_groups();
}

#[test]
fn nested_runtimes_do_not_inherit_outer_providers() {
  DEFAULT_CALLS.set(0);
  let document = self::document();
  let mut game = Game::default();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), |_| {
    PRIMARY.provider("outer").child(NestedRuntime)
  });
  let snapshot = self::begin(&mut reactant, &mut game, &document);
  assert_eq!(
    self::texts(&snapshot, document.root_id),
    ["nested:primary-default/secondary-default"]
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn provider_value_types_share_reconciliation_identity() {
  let alternate = Rc::new(Cell::new(false));
  let view_alternate = Rc::clone(&alternate);
  let setter = Rc::new(RefCell::new(None));
  let view_setter = Rc::clone(&setter);
  let document = self::document();
  let mut game = Game::default();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), move |_| {
    if view_alternate.get() {
      Node::new(PRIMARY.provider("alternate").child(StatefulConsumer {
        setter: Rc::clone(&view_setter),
      }))
    } else {
      Node::new(NUMBER.provider(7).child(StatefulConsumer {
        setter: Rc::clone(&view_setter),
      }))
    }
  });
  let first = self::begin(&mut reactant, &mut game, &document);
  assert_eq!(self::texts(&first, document.root_id), ["0"]);
  setter
    .borrow()
    .clone()
    .expect("state setter should render")
    .set(1);
  alternate.set(true);

  let second = reactant
    .begin_session(&mut game)
    .unwrap()
    .into_parts(self::snapshot(&document))
    .0;
  assert_eq!(self::texts(&second, document.root_id), ["1"]);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn required_context_accepts_a_provider_and_panics_when_missing() {
  let document = self::document();
  let mut game = Game::default();
  let mut provided = runtime_support::reactant(IdleSpawner);
  provided.register_root(document.clone(), |_| {
    REQUIRED.provider("session").child(RequiredConsumer)
  });
  let snapshot = self::begin(&mut provided, &mut game, &document);
  assert_eq!(self::texts(&snapshot, document.root_id), ["session"]);

  let missing_document = self::document();
  let mut missing = runtime_support::reactant(IdleSpawner);
  missing.register_root(missing_document, |_| RequiredConsumer);
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _ = missing.begin_session(&mut game);
    }))
    .is_err()
  );
  let _ = provided.shutdown(&mut game).into_groups();
}

#[test]
fn ref_and_context_hooks_enforce_positional_kind_and_identity() {
  struct VariableHook {
    alternate: bool,
  }

  impl Component for VariableHook {
    fn render(&self) -> impl Render {
      if self.alternate {
        let _ = hooks::use_state(0_u8);
      } else {
        let _ = hooks::use_ref_with(|| 0_u8);
      }
      battlement_reactant::host::Label::new(trox::assert_localized("hook"))
    }
  }

  let alternate = Rc::new(Cell::new(false));
  let view_alternate = Rc::clone(&alternate);
  let document = self::document();
  let mut game = Game::default();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), move |_| VariableHook {
    alternate: view_alternate.get(),
  });
  let _ = self::begin(&mut reactant, &mut game, &document);
  alternate.set(true);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.refresh(&mut game))).is_err());

  struct VariableContext {
    alternate: bool,
  }

  impl Component for VariableContext {
    fn render(&self) -> impl Render {
      let value = if self.alternate {
        hooks::use_context(&SECONDARY)
      } else {
        hooks::use_context(&PRIMARY)
      };
      battlement_reactant::host::Label::new(trox::assert_localized(value))
    }
  }

  let alternate = Rc::new(Cell::new(false));
  let view_alternate = Rc::clone(&alternate);
  let document = self::document();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), move |_| VariableContext {
    alternate: view_alternate.get(),
  });
  let _ = self::begin(&mut reactant, &mut game, &document);
  alternate.set(true);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.refresh(&mut game))).is_err());
}

fn primary_default() -> &'static str {
  DEFAULT_CALLS.set(DEFAULT_CALLS.get() + 1);
  "primary-default"
}

fn begin(reactant: &mut Reactant<Game>, game: &mut Game, document: &UiDocument) -> Snapshot {
  reactant
    .begin_session(game)
    .unwrap()
    .into_parts(self::snapshot(document))
    .0
}

fn texts(snapshot: &Snapshot, root: ObjectId) -> Vec<&str> {
  let children = &snapshot
    .ui
    .iter()
    .find(|document| document.root_id == root)
    .expect("document should render")
    .children;
  children
    .iter()
    .flat_map(|child| self::node_texts(child))
    .collect()
}

fn node_texts(node: &UiNode) -> Vec<&str> {
  let text = match &node.element {
    UiElement::Label(label) => match &label.text {
      Prop::Set(value) => Some(value.as_str()),
      Prop::Unset | Prop::Reset => None,
    },
    _ => None,
  };
  text
    .into_iter()
    .chain(node.children.iter().flat_map(self::node_texts))
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
