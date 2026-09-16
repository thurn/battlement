use trox::ls;
mod runtime_support;

use std::{num::NonZeroU32, rc::Rc, slice, sync::Arc};

use battlement::{
  Align, CameraState, FlexDirection, FlexWrap, GameObject, GameObjectKind, GridItem, GridTrack,
  Justify, LowerLimit, ObjectId, OverlayPlacement, PanelScaleMode, PanelSettings, ParentScene,
  PreparedAsset, Prop, Response, ResponseMessage, Scene, SceneId, SessionId, Snapshot, StackItem,
  Sticky, Style, UiDocument, UiDocumentState, UiElement, UiElementKind, UiNode,
  UiVisualElementProperties, UpperLimit,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{
  ConnectView, Engine, EngineError, EngineResponse, FlatBufferSubmitError, UiEventActionView,
  UiEventResult,
};
use reactant_core::{
  component::Component,
  element_ref::use_element_ref,
  executor::{BoxFuture, SpawnedTask, Spawner},
  props::Missing,
  render::{Node, Render},
  runtime::Reactant,
};
use uuid::Uuid;

struct IdleSpawner;

struct SessionEngine<G: 'static> {
  game: G,
  reactant: Reactant<G>,
  document: UiDocument,
  recorded: Rc<std::cell::RefCell<Option<Response>>>,
}

struct RequiredOptions {
  emphasized: bool,
}

struct GeneratedCard<Title = Missing, Child = Missing> {
  required: (Title, Child),
  optional: RequiredOptions,
}

struct ManualCard<Title = Missing, Child = Missing> {
  required: (Title, Child),
  optional: RequiredOptions,
}

struct OverlayFixture {
  target: reactant_core::portal::PortalTarget,
}

impl Component for OverlayFixture {
  fn render(&self) -> impl Render {
    let anchor = use_element_ref();
    reactant_core::host::Stack::new()
      .child(reactant_core::host::ButtonHost::new(ls("anchor")).element_ref(anchor.clone()))
      .child(
        reactant_core::overlay::Overlay::popover(self.target.clone(), anchor)
          .placement(battlement::PopoverPlacement::bottom_start().offset(6.0))
          .child(reactant_core::render::Fragment::new((
            reactant_core::host::Label::new(ls("one")),
            reactant_core::host::Label::new(ls("two")),
          ))),
      )
      .child(reactant_core::overlay::OverlayHost::new(
        self.target.clone(),
      ))
  }
}

reactant_core::required_props!(GeneratedCard, title: String, child: reactant_core::host::Label);

impl GeneratedCard<Missing, Missing> {
  fn new() -> Self {
    Self {
      required: (Missing, Missing),
      optional: RequiredOptions { emphasized: false },
    }
  }
}

impl<Title, Child> GeneratedCard<Title, Child> {
  fn emphasized(mut self, value: bool) -> Self {
    self.optional.emphasized = value;
    self
  }
}

impl Component for GeneratedCard<String, reactant_core::host::Label> {
  fn render(&self) -> impl Render {
    card_tree(&self.required.0, &self.required.1, self.optional.emphasized)
  }
}

impl ManualCard<Missing, Missing> {
  fn new() -> Self {
    Self {
      required: (Missing, Missing),
      optional: RequiredOptions { emphasized: false },
    }
  }
}

impl<Child> ManualCard<Missing, Child> {
  fn title(self, value: String) -> ManualCard<String, Child> {
    ManualCard {
      required: (value, self.required.1),
      optional: self.optional,
    }
  }
}

impl<Title> ManualCard<Title, Missing> {
  fn child(
    self,
    value: reactant_core::host::Label,
  ) -> ManualCard<Title, reactant_core::host::Label> {
    ManualCard {
      required: (self.required.0, value),
      optional: self.optional,
    }
  }
}

impl<Title, Child> ManualCard<Title, Child> {
  fn emphasized(mut self, value: bool) -> Self {
    self.optional.emphasized = value;
    self
  }
}

impl Component for ManualCard<String, reactant_core::host::Label> {
  fn render(&self) -> impl Render {
    card_tree(&self.required.0, &self.required.1, self.optional.emphasized)
  }
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl<G: 'static> Engine for SessionEngine<G> {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_CONTRACT_DIGEST_C;

  fn connect(&mut self, _message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    let response = self
      .reactant
      .begin_session(&mut self.game)
      .expect("fixture render succeeds")
      .into_response(snapshot(
        SessionId::new_v4(),
        slice::from_ref(&self.document),
      ));
    self.recorded.replace(Some(response.clone()));
    runtime_support::encoded(response)
  }

  fn submit(&mut self, _message: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    Err(FlatBufferSubmitError::engine(EngineError::new(
      "fixture does not accept actions",
    )))
  }

  fn submit_ui_event(
    &mut self,
    message: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    let session_id =
      SessionId::from_uuid(uuid::Uuid::from_bytes(message.session_id())).expect("verified session");
    Ok(UiEventResult {
      disposition: if message.default_prevented() {
        battlement::UiEventDisposition::PreventDefault
      } else {
        battlement::UiEventDisposition::Continue
      },
      response: runtime_support::encoded(Response::empty(session_id))?,
    })
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    Ok(None)
  }
}

impl<G: 'static> Drop for SessionEngine<G> {
  fn drop(&mut self) {
    let _ = self.reactant.shutdown(&mut self.game).into_groups();
  }
}

#[test]
fn generated_and_handwritten_required_props_render_equivalent_fake_trees() {
  let document = document();
  let root_id = document.root_id;
  let recorded = Rc::new(std::cell::RefCell::new(None));
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| {
    (
      GeneratedCard::new()
        .emphasized(true)
        .child(reactant_core::host::Label::new(ls("body")))
        .title("Citadel".to_owned()),
      ManualCard::new()
        .title("Citadel".to_owned())
        .emphasized(true)
        .child(reactant_core::host::Label::new(ls("body"))),
    )
  });
  let engine = SessionEngine {
    game: (),
    reactant,
    document,
    recorded,
  };

  let mut client = FakeClient::connect(engine, catalog());

  let cards = client.ui().element(root_id).children().to_vec();
  assert_eq!(cards.len(), 2);
  for card in cards {
    let (kind, classes, children) = {
      let ui = client.ui();
      let rendered = ui.element(card);
      (
        rendered.kind(),
        rendered.classes().map(<[String]>::to_vec),
        rendered.children().to_vec(),
      )
    };
    assert_eq!(kind, UiElementKind::VisualElement);
    assert_eq!(classes, Some(vec!["emphasized".to_owned()]));
    assert_eq!(client.ui().element(children[0]).text(), Some("Citadel"));
    assert_eq!(client.ui().element(children[1]).text(), Some("body"));
  }
}

#[test]
fn common_facades_lower_layout_item_descriptors() {
  let document = document();
  let grid_item = GridItem::default();
  let stack_item = StackItem::default();
  let sticky = Sticky {
    top: Some(0.0),
    ..Sticky::default()
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), move |_: &()| {
    (
      reactant_core::host::Grid::new().child(reactant_core::host::View::new().grid_item(grid_item)),
      reactant_core::host::Stack::new()
        .align_items(Align::Center)
        .justify_items(Align::FlexEnd)
        .child(reactant_core::host::ButtonHost::new(ls("stack")).stack_item(stack_item)),
      reactant_core::host::ScrollView::new()
        .child(reactant_core::host::Label::new(ls("sticky")).sticky(sticky)),
    )
  });

  let rendered = reactant
    .begin_session(&mut ())
    .expect("descriptor render succeeds")
    .into_parts(snapshot(
      SessionId::new_v4(),
      std::slice::from_ref(&document),
    ))
    .0;
  let children = &rendered.ui[0].children;

  assert_eq!(
    children[0].children[0].element.visual_element().grid_item,
    Prop::Set(grid_item)
  );
  assert_eq!(
    children[1].children[0].element.visual_element().stack_item,
    Prop::Set(stack_item)
  );
  let UiElement::Stack(stack) = &children[1].element else {
    panic!("Stack facade must lower to UiStack")
  };
  assert_eq!(stack.align_items, Prop::Set(Align::Center));
  assert_eq!(stack.justify_items, Prop::Set(Align::FlexEnd));
  assert_eq!(
    children[2].children[0].element.visual_element().sticky,
    Prop::Set(sticky)
  );
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn overlay_helpers_resolve_first_mount_refs_and_wrap_fragment_children() {
  let document = document();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let target = reactant.create_portal_target();
  reactant.register_root(document.clone(), move |_: &()| OverlayFixture {
    target: target.clone(),
  });

  let rendered = reactant
    .begin_session(&mut ())
    .expect("overlay render succeeds")
    .into_parts(snapshot(
      SessionId::new_v4(),
      std::slice::from_ref(&document),
    ))
    .0;
  let root = &rendered.ui[0].children[0];
  let anchor = root.children[0].object_id;
  let overlay_host = &root.children[1];
  let wrapper = &overlay_host.children[0];
  assert!(matches!(
    wrapper.element.visual_element().overlay_placement,
    Prop::Set(OverlayPlacement::Popover {
      anchor: value,
      placement
    }) if value == anchor && placement.main_offset == 6.0
  ));
  assert_eq!(wrapper.children.len(), 2);
  assert_eq!(
    overlay_host.element.visual_element().picking_mode,
    Prop::Set(battlement::PickingMode::Ignore)
  );
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn decorative_view_ignores_pointer_picking() {
  let document = document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| {
    reactant_core::host::View::decorative()
  });

  let rendered = reactant
    .begin_session(&mut ())
    .expect("decorative view renders")
    .into_parts(snapshot(
      SessionId::new_v4(),
      std::slice::from_ref(&document),
    ))
    .0;
  assert_eq!(
    rendered.ui[0].children[0]
      .element
      .visual_element()
      .picking_mode,
    Prop::Set(battlement::PickingMode::Ignore)
  );
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn flex_facade_preserves_specific_gap_overrides_in_either_builder_order() {
  let document = document();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| {
    (
      reactant_core::host::Flex::new()
        .direction(FlexDirection::RowReverse)
        .wrap(FlexWrap::Wrap)
        .align_items(Align::Center)
        .justify_content(Justify::SpaceBetween)
        .row_gap(1.0)
        .gap(2.0)
        .on_click(|_: &mut ()| {}),
      reactant_core::host::Flex::new().gap(2.0).row_gap(1.0),
    )
  });

  let rendered = reactant
    .begin_session(&mut ())
    .expect("flex render succeeds")
    .into_parts(snapshot(
      SessionId::new_v4(),
      std::slice::from_ref(&document),
    ))
    .0;
  for child in &rendered.ui[0].children {
    let UiElement::Flex(flex) = &child.element else {
      panic!("Flex facade lowered to a different protocol host");
    };
    assert_eq!(flex.row_gap, Prop::Set(1.0));
    assert_eq!(flex.column_gap, Prop::Set(2.0));
  }
  let UiElement::Flex(flex) = &rendered.ui[0].children[0].element else {
    unreachable!();
  };
  assert_eq!(flex.direction, Prop::Set(FlexDirection::RowReverse));
  assert_eq!(flex.wrap, Prop::Set(FlexWrap::Wrap));
  assert_eq!(flex.align_items, Prop::Set(Align::Center));
  assert_eq!(flex.justify_content, Prop::Set(Justify::SpaceBetween));
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn grid_facade_lowers_explicit_implicit_and_alignment_sizing() {
  let document = document();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| {
    (
      reactant_core::host::Grid::new()
        .columns([GridTrack::px(80.0), GridTrack::fr(1.0)])
        .rows([GridTrack::auto()])
        .auto_columns(GridTrack::fr(2.0))
        .auto_rows(GridTrack::px(24.0))
        .row_gap(1.0)
        .gap(2.0)
        .align_items(Align::Center)
        .justify_items(Align::FlexEnd)
        .on_click(|_: &mut ()| {}),
      reactant_core::host::Grid::new().gap(2.0).row_gap(1.0),
    )
  });

  let rendered = reactant
    .begin_session(&mut ())
    .expect("grid render succeeds")
    .into_parts(snapshot(
      SessionId::new_v4(),
      std::slice::from_ref(&document),
    ))
    .0;
  for child in &rendered.ui[0].children {
    let UiElement::Grid(grid) = &child.element else {
      panic!("Grid facade lowered to a different protocol host");
    };
    assert_eq!(grid.row_gap, Prop::Set(1.0));
    assert_eq!(grid.column_gap, Prop::Set(2.0));
  }
  let UiElement::Grid(grid) = &rendered.ui[0].children[0].element else {
    unreachable!();
  };
  assert_eq!(
    grid.columns,
    Prop::Set(vec![GridTrack::px(80.0), GridTrack::fr(1.0)])
  );
  assert_eq!(grid.rows, Prop::Set(vec![GridTrack::auto()]));
  assert_eq!(grid.auto_columns, Prop::Set(GridTrack::fr(2.0)));
  assert_eq!(grid.auto_rows, Prop::Set(GridTrack::px(24.0)));
  assert_eq!(grid.align_items, Prop::Set(Align::Center));
  assert_eq!(grid.justify_items, Prop::Set(Align::FlexEnd));
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn fake_client_receives_every_primitive_and_legal_child_family() {
  let document = document();
  let root_id = document.root_id;
  let recorded = Rc::new(std::cell::RefCell::new(None));
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| primitive_catalog());
  let engine = SessionEngine {
    game: (),
    reactant,
    document,
    recorded: Rc::clone(&recorded),
  };

  let mut client = FakeClient::connect(engine, catalog());

  let response = recorded.borrow();
  let ResponseMessage::Snapshot(rendered) =
    &response.as_ref().expect("response recorded").messages[0]
  else {
    panic!("session response did not begin with a snapshot");
  };
  let mut rendered_nodes = Vec::new();
  collect_nodes(&rendered.ui[0].children, &mut rendered_nodes);
  for (object_id, kind) in &rendered_nodes {
    assert_eq!(client.ui().element(*object_id).kind(), *kind);
  }
  assert_eq!(
    client.ui().element(root_id).children(),
    &[rendered.ui[0].children[0].object_id]
  );
  for kind in every_kind() {
    assert!(
      rendered_nodes.iter().any(|(_, actual)| *actual == kind),
      "catalog omitted {kind:?}"
    );
  }
  let root = &rendered.ui[0].children[0];
  assert_eq!(root.element.kind(), UiElementKind::VisualElement);
  assert_eq!(root.children[0].element.kind(), UiElementKind::Box);
  assert_eq!(root.children[0].children.len(), 1);
  let toggle_group = root
    .children
    .iter()
    .find(|node| node.element.kind() == UiElementKind::ToggleButtonGroup)
    .expect("toggle group rendered");
  assert_eq!(
    toggle_group.children[0].element.kind(),
    UiElementKind::Button
  );
  let tab_view = root
    .children
    .iter()
    .find(|node| node.element.kind() == UiElementKind::TabView)
    .expect("tab view rendered");
  assert_eq!(tab_view.children[0].element.kind(), UiElementKind::Tab);
  assert_eq!(
    tab_view.children[0].children[0].element.kind(),
    UiElementKind::Label
  );
}

fn primitive_catalog() -> impl reactant_core::render::Render {
  reactant_core::host::View::new()
    .name("catalog")
    .children(vec![
      Node::new(
        reactant_core::host::Box::new()
          .name("box")
          .child(reactant_core::host::Label::new(ls("box child"))),
      ),
      Node::new(reactant_core::host::Label::new(ls("label"))),
      Node::new(reactant_core::host::TextElement::new(ls("text element"))),
      Node::new(
        reactant_core::host::TextField::new()
          .label(ls("text field"))
          .value("value"),
      ),
      Node::new(
        reactant_core::host::ToggleHost::new()
          .text(ls("toggle"))
          .value(true),
      ),
      Node::new(
        reactant_core::host::RadioButton::new()
          .text(ls("radio"))
          .value(false),
      ),
      Node::new(
        reactant_core::host::RadioButtonGroup::new()
          .label(ls("quality"))
          .choices([ls("low"), ls("high")])
          .selected_index(0),
      ),
      Node::new(
        reactant_core::host::ToggleButtonGroup::new()
          .label(ls("alignment"))
          .child(reactant_core::host::ButtonHost::new(ls("left"))),
      ),
      Node::new(
        reactant_core::host::DropdownField::new()
          .label(ls("mode"))
          .choices([ls("one")])
          .selection(0, ls("one")),
      ),
      Node::new(reactant_core::host::ButtonHost::new(ls("button"))),
      Node::new(reactant_core::host::RepeatButton::new(
        ls("repeat"),
        100,
        NonZeroU32::new(25).expect("nonzero interval"),
      )),
      Node::new(
        reactant_core::host::GroupBox::new()
          .text(ls("group"))
          .child(reactant_core::host::Label::new(ls("group child"))),
      ),
      Node::new(
        reactant_core::host::PopupWindow::new()
          .text(ls("popup"))
          .content_container_style(Style::new().padding(8.0))
          .child(reactant_core::host::Label::new(ls("popup child"))),
      ),
      Node::new(
        reactant_core::host::ScrollView::new()
          .child(reactant_core::host::Label::new(ls("scroll child"))),
      ),
      Node::new(
        reactant_core::host::Scroller::new()
          .low_value(0.0)
          .high_value(10.0)
          .value(5.0),
      ),
      Node::new(
        reactant_core::host::SliderHost::new()
          .low_value(0.0)
          .high_value(10.0)
          .value(5.0),
      ),
      Node::new(
        reactant_core::host::SliderInt::new()
          .low_value(0)
          .high_value(10)
          .value(5),
      ),
      Node::new(
        reactant_core::host::MinMaxSlider::new()
          .low_limit(LowerLimit::Inclusive(0.0))
          .high_limit(UpperLimit::Inclusive(10.0))
          .min_value(2.0)
          .max_value(8.0),
      ),
      Node::new(
        reactant_core::host::ProgressBar::new()
          .title(ls("progress"))
          .low_value(0.0)
          .high_value(10.0)
          .value(5.0),
      ),
      Node::new(
        reactant_core::host::TabView::new()
          .selected_tab_index(0)
          .child(
            reactant_core::host::TabHost::new(ls("tab"))
              .closeable(true)
              .child(reactant_core::host::Label::new(ls("tab child"))),
          ),
      ),
      Node::new(reactant_core::host::ImageHost::new().name("image")),
    ])
}

fn card_tree(title: &str, child: &reactant_core::host::Label, emphasized: bool) -> impl Render {
  let card = if emphasized {
    reactant_core::host::View::new().class("emphasized")
  } else {
    reactant_core::host::View::new()
  };
  card
    .child(reactant_core::host::Label::new(ls(title)))
    .child(child.clone())
}

fn every_kind() -> [UiElementKind; 23] {
  [
    UiElementKind::VisualElement,
    UiElementKind::Box,
    UiElementKind::Label,
    UiElementKind::TextElement,
    UiElementKind::TextField,
    UiElementKind::Toggle,
    UiElementKind::RadioButton,
    UiElementKind::RadioButtonGroup,
    UiElementKind::ToggleButtonGroup,
    UiElementKind::DropdownField,
    UiElementKind::Button,
    UiElementKind::RepeatButton,
    UiElementKind::GroupBox,
    UiElementKind::PopupWindow,
    UiElementKind::ScrollView,
    UiElementKind::Scroller,
    UiElementKind::Slider,
    UiElementKind::SliderInt,
    UiElementKind::MinMaxSlider,
    UiElementKind::ProgressBar,
    UiElementKind::Tab,
    UiElementKind::TabView,
    UiElementKind::Image,
  ]
}

fn collect_nodes(nodes: &[UiNode], output: &mut Vec<(ObjectId, UiElementKind)>) {
  for node in nodes {
    output.push((node.object_id, node.element.kind()));
    collect_nodes(&node.children, output);
  }
}

fn document() -> UiDocument {
  UiDocument::with_root_id(object_id(10), object_id(11))
}

fn snapshot(session_id: SessionId, documents: &[UiDocument]) -> Snapshot {
  let camera_id = object_id(1);
  let mut objects = vec![GameObject::new(camera_id, CameraState::new())];
  objects.extend(documents.iter().map(|document| {
    GameObject::new(
      document.document_id,
      GameObjectKind::UiDocument(UiDocumentState::new(document.root_id).panel_settings(
        PanelSettings::new().scale_mode(PanelScaleMode::constant_logical_pixel_size()),
      )),
    )
    .parent_scene(ParentScene::Persistent)
  }));
  Snapshot::new(
    session_id,
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    objects,
    camera_id,
  )
}

fn catalog() -> Arc<FakeAssetCatalog> {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene("test/scene");
  Arc::new(catalog)
}

fn object_id(value: u128) -> ObjectId {
  ObjectId::from_uuid(Uuid::from_u128(value)).expect("fixture ID is nonzero")
}
