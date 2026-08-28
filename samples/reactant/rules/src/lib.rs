//! Native Rust engine for the standalone Reactant sample.

mod design_system;

use battlement::{
  ActionBody, CameraState, ClientMessage, Command, Connect, CoreErrorCode, GameObject,
  GameObjectKind, ObjectId, PanelScaleMode, PanelSettings, ParentScene, PickingMode, PreparedAsset,
  Response, Scene, SceneId, SessionId, Snapshot, UiDocument, UiDocumentState, object_id, scene_id,
};
use battlement_native::{Engine, EngineError};
use battlement_reactant::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  prelude::*,
  runtime::{Reactant, ResponseReactantExt},
};

const CAMERA_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000001");
const DOCUMENT_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000002");
const SCENE_ID: SceneId = scene_id!("25300000-0000-4000-8000-000000000003");

/// Address of the sample's authored content scene.
pub const CONTENT_SCENE: &str = "reactant/content";
/// Stable identity of the Reactant document root.
pub const ROOT_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000004");

/// A screen available in the Reactant sample.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
  /// Component and structural composition.
  Composition,
}

/// Native Reactant sample rules engine.
pub struct ReactantEngine {
  session_id: SessionId,
  game: Game,
  reactant: Reactant<Game>,
  document: UiDocument,
}

/// Creates the engine used by the Reactant sample.
pub fn create_engine() -> Result<ReactantEngine, EngineError> {
  let document = UiDocument::with_root_id(DOCUMENT_ID, ROOT_ID)
    .name("battlement-reactant")
    .picking_mode(PickingMode::Ignore)
    .style(design_system::root());
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |game: &Game| Shell {
    screen: game.screen,
    reversed: game.reversed,
  });
  Ok(ReactantEngine {
    session_id: SessionId::new_v4(),
    game: Game {
      screen: Screen::Composition,
      reversed: false,
    },
    reactant,
    document,
  })
}

impl ReactantEngine {
  /// Returns the currently selected sample screen.
  #[must_use]
  pub const fn screen(&self) -> Screen {
    self.game.screen
  }
}

impl Engine for ReactantEngine {
  type ActionPayload = ();
  type ErrorCode = CoreErrorCode;
  type Command = Command;

  fn connect(&mut self, _message: Connect) -> Result<Response, EngineError> {
    self.session_id = SessionId::new_v4();
    Ok(
      self
        .reactant
        .begin_session(&mut self.game)
        .expect("sample render should succeed")
        .into_response(snapshot(self.session_id, &self.document)),
    )
  }

  fn submit(
    &mut self,
    message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
  ) -> Result<Response, EngineError> {
    let Some(action) = message.into_action() else {
      return Ok(Response::empty(self.session_id));
    };
    let ActionBody::VisualElement(event) = action.body else {
      return Ok(Response::empty(self.session_id));
    };
    Ok(
      Response::empty(self.session_id).append_reactant_for_action(
        action.action_id,
        self
          .reactant
          .dispatch(&mut self.game, event)
          .expect("sample event dispatch should succeed"),
      ),
    )
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(None)
  }
}

struct Game {
  screen: Screen,
  reversed: bool,
}

struct IdleSpawner;

struct Shell {
  screen: Screen,
  reversed: bool,
}

struct Navigation;

struct Composition {
  reversed: bool,
}

struct Badge {
  text: &'static str,
}

struct Specimen<Heading = Missing, Child = Missing> {
  required: (Heading, Child),
  optional: (),
}

required_props!(Specimen, heading: String, child: Node);

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl Component for Shell {
  fn render(&self) -> impl Render {
    let page = match self.screen {
      Screen::Composition => Composition {
        reversed: self.reversed,
      },
    };
    VisualElement::new()
      .name("sample-shell")
      .style(design_system::root())
      .child(Navigation)
      .child(page)
  }
}

impl Component for Navigation {
  fn render(&self) -> impl Render {
    VisualElement::new()
      .name("navigation")
      .style(design_system::navigation())
      .child(Label::new("REACTANT").style(design_system::brand()))
      .child(
        Button::new("01  COMPOSITION")
          .name("composition-navigation")
          .style(design_system::navigation_item()),
      )
  }
}

impl Component for Composition {
  fn render(&self) -> impl Render {
    VisualElement::new()
      .name("composition-canvas")
      .style(design_system::canvas())
      .child(Label::new("COMPOSITION").style(design_system::eyebrow()))
      .child(
        Label::new("Build declaratively")
          .name("page-title")
          .style(design_system::title()),
      )
      .child(
        Button::new(if self.reversed { "RESTORE" } else { "REORDER" })
          .name("composition-action")
          .style(design_system::navigation_item())
          .on_click(|game: &mut Game| game.reversed = !game.reversed),
      )
      .child(Fragment::new(
        Specimen::new()
          .child(composition_badges(self.reversed))
          .heading("Owned components".to_owned()),
      ))
  }
}

impl Component for Badge {
  fn render(&self) -> impl Render {
    VisualElement::new()
      .style(design_system::badge())
      .child(Label::new(self.text).style(design_system::badge_text()))
  }
}

impl Specimen<Missing, Missing> {
  fn new() -> Self {
    Self {
      required: (Missing, Missing),
      optional: (),
    }
  }
}

impl Component for Specimen<String, Node> {
  fn render(&self) -> impl Render {
    VisualElement::new()
      .name("composition-specimen")
      .style(design_system::specimen())
      .child(
        Label::new(self.required.0.clone())
          .name("specimen-heading")
          .style(design_system::specimen_title()),
      )
      .child(self.required.1.clone())
  }
}

fn composition_badges(reversed: bool) -> Node {
  let mut badges = vec![
    Badge {
      text: "Required props",
    },
    Badge {
      text: "Structural values",
    },
    Badge {
      text: "Primitive children",
    },
  ];
  if reversed {
    badges.reverse();
  }
  Node::new(
    VisualElement::new()
      .name("composition-badges")
      .style(design_system::badge_row())
      .child(Fragment::new(badges)),
  )
}

fn snapshot(session_id: SessionId, document: &UiDocument) -> Snapshot {
  let ui_host = GameObject::new(
    document.document_id,
    GameObjectKind::UiDocument(
      UiDocumentState::new(document.root_id)
        .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantPixelSize)),
    ),
  )
  .parent_scene(ParentScene::Persistent);
  Snapshot::new(
    session_id,
    vec![PreparedAsset::scene(CONTENT_SCENE)],
    vec![Scene::new(SCENE_ID, CONTENT_SCENE)],
    vec![
      GameObject::new(CAMERA_ID, CameraState::new()).parent_scene(ParentScene::Persistent),
      ui_host,
    ],
    CAMERA_ID,
  )
}

battlement_native::export_engine!(create_engine);
