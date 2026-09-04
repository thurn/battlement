use trox::ls;
mod runtime_support;

use std::{
  cell::RefCell,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{
  CameraState, ClickEvent, CommandBody, GameObject, GameObjectKind, ObjectId, PanelScaleMode,
  PanelSettings, ParentScene, PreparedAsset, Scene, SceneId, SessionId, Snapshot, UiDocument,
  UiDocumentState, UiEvent, UiVisualElementProperties, VisualElementAction,
};
use battlement_reactant::{
  component::Component,
  element_ref::{ElementRef, use_element_ref},
  executor::{BoxFuture, SpawnedTask, Spawner},
  render::{Node, Render},
  runtime::Reactant,
};

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[derive(Default)]
struct Game {
  duplicate: bool,
  invalid: Option<InvalidRender>,
  key: u8,
  reversed: bool,
  show: bool,
}

#[derive(Clone, Copy)]
enum InvalidRender {
  Action,
  Query,
}

#[derive(Clone)]
struct MovingFixture {
  handle: Rc<RefCell<Option<ElementRef>>>,
  key: u8,
  reversed: bool,
  show: bool,
}

impl Component for MovingFixture {
  fn render(&self) -> impl Render {
    let element_ref = use_element_ref();
    self.handle.replace(Some(element_ref.clone()));
    let target = Node::new(
      battlement_reactant::host::ButtonHost::new(ls("target"))
        .name("target")
        .key(self.key)
        .element_ref(element_ref),
    );
    let sibling = Node::new(battlement_reactant::host::Label::new(ls("sibling")).key("sibling"));
    let mut children = vec![target, sibling];
    if self.reversed {
      children.reverse();
    }
    battlement_reactant::host::View::new()
      .name("container")
      .child(self.show.then_some(children))
  }
}

#[derive(Clone)]
struct DuplicateFixture {
  duplicate: bool,
}

impl Component for DuplicateFixture {
  fn render(&self) -> impl Render {
    let element_ref = use_element_ref();
    let second = self.duplicate.then(|| {
      battlement_reactant::host::ButtonHost::new(ls("second")).element_ref(element_ref.clone())
    });
    battlement_reactant::host::View::new()
      .child(battlement_reactant::host::ButtonHost::new(ls("first")).element_ref(element_ref))
      .child(second)
  }
}

#[derive(Clone)]
struct InvalidRenderFixture {
  handle: Rc<RefCell<Option<ElementRef>>>,
  invalid: Option<InvalidRender>,
}

impl Component for InvalidRenderFixture {
  fn render(&self) -> impl Render {
    let element_ref = use_element_ref();
    self.handle.replace(Some(element_ref.clone()));
    match self.invalid {
      Some(InvalidRender::Action) => element_ref.focus(),
      Some(InvalidRender::Query) => {
        let _ = element_ref.is_attached();
      }
      None => {}
    }
    battlement_reactant::host::ButtonHost::new(ls("target")).element_ref(element_ref)
  }
}

#[derive(Clone)]
struct ActionFixture {
  button: Rc<RefCell<Option<ElementRef>>>,
  child: Rc<RefCell<Option<ElementRef>>>,
  scroll: Rc<RefCell<Option<ElementRef>>>,
  text: Rc<RefCell<Option<ElementRef>>>,
}

#[derive(Clone)]
struct InvalidTargetFixture {
  handle: Rc<RefCell<Option<ElementRef>>>,
}

impl Component for InvalidTargetFixture {
  fn render(&self) -> impl Render {
    let element_ref = use_element_ref();
    self.handle.replace(Some(element_ref.clone()));
    battlement_reactant::host::Label::new(ls("not focusable")).element_ref(element_ref)
  }
}

impl Component for ActionFixture {
  fn render(&self) -> impl Render {
    let button = use_element_ref();
    let scroll = use_element_ref();
    let child = use_element_ref();
    let text = use_element_ref();
    self.button.replace(Some(button.clone()));
    self.scroll.replace(Some(scroll.clone()));
    self.child.replace(Some(child.clone()));
    self.text.replace(Some(text.clone()));
    battlement_reactant::host::View::new()
      .child(battlement_reactant::host::ButtonHost::new(ls("focus")).element_ref(button))
      .child(
        battlement_reactant::host::ScrollView::new()
          .child(battlement_reactant::host::Label::new(ls("inside")).element_ref(child))
          .element_ref(scroll),
      )
      .child(
        battlement_reactant::host::TextField::new()
          .value("A🚀B")
          .element_ref(text),
      )
  }
}

#[test]
fn refs_survive_keyed_moves_and_stale_actions_do_not_follow_remounts() {
  let handle = Rc::new(RefCell::new(None));
  let view_handle = Rc::clone(&handle);
  let document = self::document();
  let mut game = Game {
    show: true,
    ..Game::default()
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), move |game: &Game| MovingFixture {
    handle: Rc::clone(&view_handle),
    key: game.key,
    reversed: game.reversed,
    show: game.show,
  });
  let first = self::begin(&mut reactant, &mut game, &document);
  let first_id = self::named_id(&first, "target");
  let element_ref = handle.borrow().clone().expect("ref should render");
  assert!(element_ref.is_attached());

  element_ref.focus();
  game.reversed = true;
  let groups = reactant.refresh(&mut game).unwrap().into_groups();
  assert_eq!(self::action_targets(&groups), [first_id]);

  element_ref.focus();
  game.key = 1;
  let groups = reactant.refresh(&mut game).unwrap().into_groups();
  assert!(self::action_targets(&groups).is_empty());
  let remounted = self::created_named_id(&groups, "target");
  assert_ne!(remounted, first_id);
  assert!(element_ref.is_attached());

  element_ref.focus();
  assert_eq!(
    self::action_targets(&reactant.poll(&mut game).unwrap().into_groups()),
    [remounted]
  );

  game.show = false;
  let _ = reactant.refresh(&mut game).unwrap().into_groups();
  assert!(!element_ref.is_attached());
  element_ref.focus();
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn reconnect_rebinds_refs_and_consumes_actions_for_the_old_attachment() {
  let handle = Rc::new(RefCell::new(None));
  let view_handle = Rc::clone(&handle);
  let document = self::document();
  let mut game = Game {
    show: true,
    ..Game::default()
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), move |game: &Game| MovingFixture {
    handle: Rc::clone(&view_handle),
    key: game.key,
    reversed: game.reversed,
    show: game.show,
  });
  let initial = self::begin(&mut reactant, &mut game, &document);
  let target = self::named_id(&initial, "target");
  let element_ref = handle.borrow().clone().expect("ref should render");
  element_ref.focus();

  let (_, commit) = reactant
    .begin_session(&mut game)
    .unwrap()
    .into_parts(self::snapshot(&document));
  assert!(commit.is_empty());
  assert!(element_ref.is_attached());

  element_ref.focus();
  assert_eq!(
    self::action_targets(&reactant.poll(&mut game).unwrap().into_groups()),
    [target]
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn every_supported_host_action_is_queued_in_invocation_order() {
  let fixture = ActionFixture {
    button: Rc::new(RefCell::new(None)),
    child: Rc::new(RefCell::new(None)),
    scroll: Rc::new(RefCell::new(None)),
    text: Rc::new(RefCell::new(None)),
  };
  let view = fixture.clone();
  let document = self::document();
  let mut game = Game::default();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), move |_| view.clone());
  let _ = self::begin(&mut reactant, &mut game, &document);
  let button = fixture.button.borrow().clone().unwrap();
  let scroll = fixture.scroll.borrow().clone().unwrap();
  let child = fixture.child.borrow().clone().unwrap();
  let text = fixture.text.borrow().clone().unwrap();

  button.focus();
  button.blur();
  button.capture_pointer(7);
  button.release_pointer(7);
  scroll.scroll_to(&child);
  text.select_text(3, 1);
  let actions = reactant
    .poll(&mut game)
    .unwrap()
    .into_groups()
    .into_iter()
    .map(|group| match &group[0] {
      CommandBody::VisualElementPerformAction(value) => value.action.clone(),
      _ => panic!("host actions should be isolated command groups"),
    })
    .collect::<Vec<_>>();
  assert!(matches!(actions[0], VisualElementAction::Focus));
  assert!(matches!(actions[1], VisualElementAction::Blur));
  assert!(matches!(
    actions[2],
    VisualElementAction::CapturePointer { pointer_id: 7 }
  ));
  assert!(matches!(
    actions[3],
    VisualElementAction::ReleasePointer { pointer_id: 7 }
  ));
  assert!(matches!(actions[4], VisualElementAction::ScrollTo { .. }));
  assert!(matches!(
    actions[5],
    VisualElementAction::SelectText {
      cursor_index: 3,
      selection_index: 1
    }
  ));
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn duplicate_attachments_and_render_time_access_poison_transactionally() {
  let document = self::document();
  let mut game = Game::default();
  let mut duplicate = runtime_support::reactant(IdleSpawner);
  duplicate.register_root(document.clone(), |game: &Game| DuplicateFixture {
    duplicate: game.duplicate,
  });
  let _ = self::begin(&mut duplicate, &mut game, &document);
  game.duplicate = true;
  assert!(panic::catch_unwind(AssertUnwindSafe(|| duplicate.refresh(&mut game))).is_err());

  for invalid in [InvalidRender::Action, InvalidRender::Query] {
    let handle = Rc::new(RefCell::new(None));
    let view_handle = Rc::clone(&handle);
    let document = self::document();
    let mut game = Game::default();
    let mut runtime = runtime_support::reactant(IdleSpawner);
    runtime.register_root(document.clone(), move |game: &Game| InvalidRenderFixture {
      handle: Rc::clone(&view_handle),
      invalid: game.invalid,
    });
    let _ = self::begin(&mut runtime, &mut game, &document);
    assert!(handle.borrow().as_ref().unwrap().is_attached());
    game.invalid = Some(invalid);
    assert!(panic::catch_unwind(AssertUnwindSafe(|| runtime.refresh(&mut game))).is_err());
  }
}

#[test]
fn actions_from_another_runtime_and_invalid_targets_panic() {
  let foreign = Rc::new(RefCell::new(None));
  let view_foreign = Rc::clone(&foreign);
  let first_document = self::document();
  let mut first_game = Game {
    show: true,
    ..Game::default()
  };
  let mut first = runtime_support::reactant(IdleSpawner);
  first.register_root(first_document.clone(), move |game: &Game| MovingFixture {
    handle: Rc::clone(&view_foreign),
    key: game.key,
    reversed: game.reversed,
    show: game.show,
  });
  let _ = self::begin(&mut first, &mut first_game, &first_document);

  let foreign = foreign.borrow().clone().unwrap();
  let second_document = self::document();
  let mut second_game = Game::default();
  let mut second = runtime_support::reactant(IdleSpawner);
  second.register_root(second_document.clone(), move |_| {
    let foreign = foreign.clone();
    battlement_reactant::host::ButtonHost::new(ls("cross"))
      .on_click(move |_game: &mut Game| foreign.focus())
  });
  let second_snapshot = self::begin(&mut second, &mut second_game, &second_document);
  let target = second_snapshot.ui[0].children[0].object_id;
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _ = second.dispatch(
        &mut second_game,
        UiEvent::click(target, ClickEvent::NavigationSubmit),
      );
    }))
    .is_err()
  );

  let handle = Rc::new(RefCell::new(None));
  let view_handle = Rc::clone(&handle);
  let document = self::document();
  let mut game = Game::default();
  let mut invalid = runtime_support::reactant(IdleSpawner);
  invalid.register_root(document.clone(), move |_| InvalidTargetFixture {
    handle: Rc::clone(&view_handle),
  });
  let _ = self::begin(&mut invalid, &mut game, &document);
  handle.borrow().as_ref().unwrap().focus();
  assert!(panic::catch_unwind(AssertUnwindSafe(|| invalid.poll(&mut game))).is_err());
  let _ = first.shutdown(&mut first_game).into_groups();
}

fn action_targets(groups: &[Vec<CommandBody>]) -> Vec<ObjectId> {
  groups
    .iter()
    .flatten()
    .filter_map(|body| match body {
      CommandBody::VisualElementPerformAction(value) => Some(value.object_id),
      _ => None,
    })
    .collect()
}

fn created_named_id(groups: &[Vec<CommandBody>], name: &str) -> ObjectId {
  groups
    .iter()
    .flatten()
    .find_map(|body| match body {
      CommandBody::VisualElementCreate(value) => self::named_node_id(&value.node, name),
      _ => None,
    })
    .expect("named host should be created")
}

fn named_node_id(node: &battlement::UiNode, name: &str) -> Option<ObjectId> {
  let matches =
    matches!(&node.element.visual_element().name, battlement::Prop::Set(value) if value == name);
  if matches {
    return Some(node.object_id);
  }
  node
    .children
    .iter()
    .find_map(|child| self::named_node_id(child, name))
}

fn named_id(snapshot: &Snapshot, name: &str) -> ObjectId {
  snapshot
    .ui
    .iter()
    .flat_map(|document| &document.children)
    .find_map(|node| self::named_node_id(node, name))
    .expect("named element should exist")
}

fn begin(reactant: &mut Reactant<Game>, game: &mut Game, document: &UiDocument) -> Snapshot {
  reactant
    .begin_session(game)
    .unwrap()
    .into_parts(self::snapshot(document))
    .0
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
