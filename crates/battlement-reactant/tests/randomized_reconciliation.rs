use std::{
  collections::HashSet,
  error::Error,
  fmt,
  panic::{self, AssertUnwindSafe},
  time::Instant,
};

use battlement::{
  CameraState, CommandBody, GameObject, GameObjectKind, LengthOrAuto, ObjectId, PanelScaleMode,
  PanelSettings, ParentScene, PreparedAsset, Prop, Scene, SceneId, SessionId, Snapshot, Style,
  StyleValue, UiDocument, UiDocumentState, UiElement, UiNode,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  component::Component,
  error_boundary::ErrorBoundary,
  executor::{BoxFuture, SpawnedTask, Spawner},
  key::KeyRenderExt,
  portal::{PortalTarget, create_portal},
  render::{Either, Fragment, Node, Render},
  runtime::{Reactant, RenderError},
};

const SEEDS: [u64; 8] = [1, 2, 3, 5, 8, 13, 21, 34];
const STEPS: usize = 64;

#[derive(Clone)]
struct Item {
  key: u16,
  revision: u16,
  value: u16,
  width: u8,
  portal: bool,
  visible: bool,
  kind: u8,
  wrapper: u8,
  fail: bool,
}

struct Model {
  items: Vec<Item>,
  next_key: u16,
}

#[derive(Clone, Debug, PartialEq)]
struct OracleNode {
  kind: &'static str,
  text: Option<String>,
  name: Option<String>,
  width: Option<f32>,
  children: Vec<OracleNode>,
}

#[derive(Clone, Copy)]
enum Mutation {
  Changed,
  Reordered,
  Unchanged,
}

struct IdleSpawner;

struct ItemView(Item);

struct RootView {
  fail: bool,
  value: u16,
}

#[derive(Debug)]
struct ItemError(u16);

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl Component for ItemView {
  fn render(&self) -> impl Render {
    if self.0.fail {
      return Node::new(Err::<Node, _>(ItemError(self.0.key)));
    }
    let text = format!("item:{}:v{}:k{}", self.0.key, self.0.value, self.0.kind);
    let name = format!("item-{}", self.0.key);
    let style = Style::new().width(f32::from(self.0.width));
    let host = match self.0.kind {
      0 => Node::new(
        battlement_reactant::host::Label::new(text)
          .name(name)
          .style(style),
      ),
      1 => Node::new(
        battlement_reactant::host::Button::new(text)
          .name(name)
          .style(style),
      ),
      _ => Node::new(
        battlement_reactant::host::View::new()
          .name(format!("wrapper-{}", self.0.key))
          .child(
            battlement_reactant::host::Label::new(text)
              .name(name)
              .style(style),
          ),
      ),
    };
    let visible = self.0.visible.then_some(host);
    match self.0.wrapper {
      0 => Node::new(visible),
      1 => Node::new(Fragment::new(visible)),
      _ => Node::new(Either::<_, Option<Node>>::Left(visible)),
    }
  }
}

impl Component for RootView {
  fn render(&self) -> impl Render {
    if self.fail {
      Err(ItemError(self.value))
    } else {
      Ok(battlement_reactant::host::Label::new(format!(
        "root:{}",
        self.value
      )))
    }
  }
}

impl fmt::Display for ItemError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "item {} failed", self.0)
  }
}

impl Error for ItemError {}

#[test]
fn deterministic_randomized_reconciliation_matches_the_physical_oracle() {
  let started = Instant::now();
  let mut command_total = 0;
  for seed in SEEDS {
    eprintln!("reactant randomized seed={seed}");
    let document = self::document();
    let mut model = self::model(seed);
    let mut reactant = Reactant::new(IdleSpawner);
    let target = reactant.create_portal_target();
    reactant.register_root(document.clone(), move |model: &Model| {
      let children = model
        .items
        .iter()
        .map(|item| self::render_item(item, &target))
        .collect::<Vec<_>>();
      (
        battlement_reactant::host::View::new()
          .name("inline")
          .child(Fragment::new(children)),
        battlement_reactant::host::View::new()
          .name("portal")
          .portal_target(target.clone()),
      )
    });
    let initial = reactant
      .begin_session(&mut model)
      .unwrap()
      .into_parts(self::snapshot(&document))
      .0;
    let mut known = HashSet::new();
    self::collect_document_ids(&initial.ui[0], &mut known);
    let mut world = UiWorld::default();
    world.replace(initial.ui).unwrap();
    self::assert_oracle(&world, document.root_id, &model, &known);
    let mut rng = Rng(seed);

    for step in 0..STEPS {
      let mutation = self::mutate(&mut model, &mut rng);
      let before = world.journal().len();
      let groups = reactant.refresh(&mut model).unwrap().into_groups();
      let commands = groups.iter().map(Vec::len).sum::<usize>();
      command_total += commands;
      if matches!(mutation, Mutation::Unchanged) {
        assert_eq!(commands, 0, "seed {seed} step {step}");
      }
      if matches!(mutation, Mutation::Reordered) {
        assert!(commands <= model.items.len(), "seed {seed} step {step}");
      }
      assert!(
        commands <= model.items.len() * 4 + 4,
        "seed {seed} step {step}"
      );
      self::apply_tracked(&mut world, groups, &mut known);
      if matches!(mutation, Mutation::Unchanged) {
        assert_eq!(world.journal().len(), before, "seed {seed} step {step}");
      }
      self::assert_oracle(&world, document.root_id, &model, &known);
    }
    let _ = reactant.shutdown(&mut model).into_groups();
  }
  eprintln!(
    "reactant randomized baseline seeds={} steps={} commands={} elapsed_us={}",
    SEEDS.len(),
    SEEDS.len() * STEPS,
    command_total,
    started.elapsed().as_micros()
  );
}

#[test]
fn deterministic_render_failures_preserve_the_last_committed_tree() {
  for seed in SEEDS {
    eprintln!("reactant failure seed={seed}");
    let document = self::document();
    let mut rng = Rng(seed);
    let mut model = RootView {
      fail: false,
      value: seed as u16,
    };
    let mut reactant = Reactant::new(IdleSpawner);
    reactant.register_root(document.clone(), |model: &RootView| RootView {
      fail: model.fail,
      value: model.value,
    });
    let initial = reactant
      .begin_session(&mut model)
      .unwrap()
      .into_parts(self::snapshot(&document))
      .0;
    let mut world = UiWorld::default();
    world.replace(initial.ui).unwrap();
    let retained = world.element(document.root_id).unwrap().children()[0];
    let mut committed = model.value;
    for _ in 0..24 {
      model.value = rng.next() as u16;
      model.fail = rng.next() % 3 == 0;
      let before = world.journal().len();
      match reactant.refresh(&mut model) {
        Ok(commit) => {
          self::apply(&mut world, commit.into_groups());
          committed = model.value;
        }
        Err(error) => {
          assert_eq!(error.downcast_ref::<ItemError>().unwrap().0, model.value);
          assert_eq!(world.journal().len(), before);
        }
      }
      assert_eq!(
        world.element(document.root_id).unwrap().children(),
        &[retained]
      );
      let expected = format!("root:{committed}");
      assert_eq!(
        world.element(retained).unwrap().text(),
        Some(expected.as_str())
      );
    }
    model.fail = false;
    let _ = reactant.shutdown(&mut model).into_groups();
  }
}

#[test]
fn duplicate_randomized_keys_fail_before_fake_world_mutation() {
  let document = self::document();
  let mut duplicate = false;
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |duplicate: &bool| {
    vec![
      Node::new(battlement_reactant::host::Label::new("first").key(7_u8)),
      Node::new(
        battlement_reactant::host::Label::new("second").key(if *duplicate { 7_u8 } else { 8_u8 }),
      ),
    ]
  });
  let initial = reactant
    .begin_session(&mut duplicate)
    .unwrap()
    .into_parts(self::snapshot(&document))
    .0;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  let before = world.journal().len();
  let children = world.element(document.root_id).unwrap().children().to_vec();
  duplicate = true;

  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.refresh(&mut duplicate))).is_err());
  assert_eq!(world.journal().len(), before);
  assert_eq!(
    world.element(document.root_id).unwrap().children(),
    children
  );
}

fn render_item(item: &Item, target: &PortalTarget) -> Node {
  let key = item.key;
  let boundary = ErrorBoundary::new(move |_: &RenderError| {
    battlement_reactant::host::Label::new(format!("error:{key}"))
  })
  .reset_on(item.revision)
  .child(ItemView(item.clone()));
  if item.portal {
    Node::new(create_portal(boundary, target.clone()).key(item.key))
  } else {
    Node::new(boundary.key(item.key))
  }
}

fn model(seed: u64) -> Model {
  let mut rng = Rng(seed);
  let items = (0..8)
    .map(|key| self::item(key, &mut rng))
    .collect::<Vec<_>>();
  Model { items, next_key: 8 }
}

fn item(key: u16, rng: &mut Rng) -> Item {
  Item {
    key,
    revision: 0,
    value: rng.next() as u16,
    width: 40 + (rng.next() % 160) as u8,
    portal: rng.next() % 3 == 0,
    visible: rng.next() % 4 != 0,
    kind: (rng.next() % 3) as u8,
    wrapper: (rng.next() % 3) as u8,
    fail: rng.next() % 7 == 0,
  }
}

fn mutate(model: &mut Model, rng: &mut Rng) -> Mutation {
  let operation = rng.next() % 9;
  if operation == 0 {
    return Mutation::Unchanged;
  }
  if operation == 1 && model.items.len() > 1 {
    let first = rng.index(model.items.len());
    let second = rng.index(model.items.len());
    model.items.swap(first, second);
    return Mutation::Reordered;
  }
  if operation == 2 && model.items.len() > 4 {
    let index = rng.index(model.items.len());
    model.items.remove(index);
    return Mutation::Changed;
  }
  if operation == 3 && model.items.len() < 12 {
    let index = rng.index(model.items.len() + 1);
    let item = self::item(model.next_key, rng);
    model.next_key += 1;
    model.items.insert(index, item);
    return Mutation::Changed;
  }
  let index = rng.index(model.items.len());
  let item = &mut model.items[index];
  item.revision = item.revision.wrapping_add(1);
  match operation {
    4 => item.portal = !item.portal,
    5 => item.visible = !item.visible,
    6 => item.kind = (item.kind + 1) % 3,
    7 => item.fail = !item.fail,
    _ => {
      item.value = rng.next() as u16;
      item.width = 40 + (rng.next() % 160) as u8;
      item.wrapper = (item.wrapper + 1) % 3;
    }
  }
  Mutation::Changed
}

fn assert_oracle(world: &UiWorld, root: ObjectId, model: &Model, known: &HashSet<ObjectId>) {
  let inline = model
    .items
    .iter()
    .filter(|item| !item.portal)
    .filter_map(self::expected_node)
    .collect();
  let portal = model
    .items
    .iter()
    .filter(|item| item.portal)
    .filter_map(self::expected_node)
    .collect();
  let expected = self::container(
    None,
    vec![
      self::container(Some("inline".to_owned()), inline),
      self::container(Some("portal".to_owned()), portal),
    ],
  );
  assert_eq!(self::actual_node(world, root), expected);

  let mut reachable = HashSet::new();
  self::collect_world_ids(world, root, &mut reachable);
  for object_id in known {
    assert!(
      reachable.contains(object_id) || world.element(*object_id).is_none(),
      "detached host {object_id} remains live"
    );
  }
}

fn expected_node(item: &Item) -> Option<OracleNode> {
  if item.fail {
    return Some(OracleNode {
      kind: "label",
      text: Some(format!("error:{}", item.key)),
      name: None,
      width: None,
      children: Vec::new(),
    });
  }
  item.visible.then(|| {
    let leaf = OracleNode {
      kind: if item.kind == 1 { "button" } else { "label" },
      text: Some(format!("item:{}:v{}:k{}", item.key, item.value, item.kind)),
      name: Some(format!("item-{}", item.key)),
      width: Some(f32::from(item.width)),
      children: Vec::new(),
    };
    if item.kind == 2 {
      self::container(Some(format!("wrapper-{}", item.key)), vec![leaf])
    } else {
      leaf
    }
  })
}

fn container(name: Option<String>, children: Vec<OracleNode>) -> OracleNode {
  OracleNode {
    kind: "visual-element",
    text: None,
    name,
    width: None,
    children,
  }
}

fn actual_node(world: &UiWorld, object_id: ObjectId) -> OracleNode {
  let element = world.element(object_id).unwrap();
  OracleNode {
    kind: match element.element() {
      UiElement::VisualElement(_) => "visual-element",
      UiElement::Label(_) => "label",
      UiElement::Button(_) => "button",
      _ => panic!("oracle found an unexpected element"),
    },
    text: element.text().map(str::to_owned),
    name: element.name().map(str::to_owned),
    width: match &element.style().width {
      Prop::Set(StyleValue::Value(LengthOrAuto::Px(value))) => Some(*value),
      Prop::Unset | Prop::Reset | Prop::Set(StyleValue::Keyword { .. }) => None,
      Prop::Set(StyleValue::Value(LengthOrAuto::Percent(_) | LengthOrAuto::Auto)) => None,
    },
    children: element
      .children()
      .iter()
      .map(|child| self::actual_node(world, *child))
      .collect(),
  }
}

fn collect_document_ids(document: &UiDocument, ids: &mut HashSet<ObjectId>) {
  ids.insert(document.root_id);
  for child in &document.children {
    self::collect_node_ids(child, ids);
  }
}

fn collect_node_ids(node: &UiNode, ids: &mut HashSet<ObjectId>) {
  ids.insert(node.object_id);
  for child in &node.children {
    self::collect_node_ids(child, ids);
  }
}

fn collect_world_ids(world: &UiWorld, object_id: ObjectId, ids: &mut HashSet<ObjectId>) {
  assert!(
    ids.insert(object_id),
    "fake hierarchy contains an identity cycle"
  );
  for child in world.element(object_id).unwrap().children() {
    self::collect_world_ids(world, *child, ids);
  }
}

fn apply_tracked(
  world: &mut UiWorld,
  groups: Vec<Vec<CommandBody>>,
  known: &mut HashSet<ObjectId>,
) {
  for body in groups.into_iter().flatten() {
    if let CommandBody::VisualElementCreate(value) = &body {
      self::collect_node_ids(&value.node, known);
    }
    self::apply_body(world, body);
  }
}

fn apply(world: &mut UiWorld, groups: Vec<Vec<CommandBody>>) {
  for body in groups.into_iter().flatten() {
    self::apply_body(world, body);
  }
}

fn apply_body(world: &mut UiWorld, body: CommandBody) {
  match body {
    CommandBody::VisualElementCreate(value) => world.create(*value).unwrap(),
    CommandBody::VisualElementUpdate(value) => world.update(*value).unwrap(),
    CommandBody::VisualElementDestroy(value) => world.destroy(value.object_id).unwrap(),
    _ => panic!("Reactant emitted a non-UI command"),
  }
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

struct Rng(u64);

impl Rng {
  fn next(&mut self) -> u64 {
    self.0 ^= self.0 << 13;
    self.0 ^= self.0 >> 7;
    self.0 ^= self.0 << 17;
    self.0
  }

  fn index(&mut self, length: usize) -> usize {
    self.next() as usize % length
  }
}
