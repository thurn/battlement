//! Native Rust engine for the standalone Reactant sample.

mod design_system;
mod state_identity;

use battlement::{
  ActionBody, CameraState, ClientMessage, Command, Connect, CoreErrorCode, GameObject,
  GameObjectKind, ObjectId, PanelScaleMode, PanelSettings, ParentScene, PickingMode, PreparedAsset,
  Response, Scene, SceneId, SessionId, Snapshot, Style, UiDocument, UiDocumentState, object_id,
  scene_id,
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
  /// Logical event routing and portal placement.
  EventsPortals,
  /// Local state and keyed component identity.
  StateIdentity,
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
    event_active: game.event_active,
    event_trace: game.event_trace.clone(),
    interaction: game.interaction,
  });
  Ok(ReactantEngine {
    session_id: SessionId::new_v4(),
    game: Game {
      screen: Screen::Composition,
      reversed: false,
      event_active: false,
      event_trace: Vec::new(),
      interaction: Interaction::default(),
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
  event_active: bool,
  event_trace: Vec<&'static str>,
  interaction: Interaction,
}

struct IdleSpawner;

struct Shell {
  screen: Screen,
  reversed: bool,
  event_active: bool,
  event_trace: Vec<&'static str>,
  interaction: Interaction,
}

struct Navigation {
  screen: Screen,
  interaction: Interaction,
}

struct Composition {
  reversed: bool,
  interaction: Interaction,
}

struct EventsPortals {
  active: bool,
  trace: Vec<&'static str>,
  interaction: Interaction,
}

struct Badge {
  text: &'static str,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Control {
  CompositionNavigation,
  EventsNavigation,
  StateNavigation,
  CompositionAction,
  EventsAction,
}

#[derive(Clone, Copy, Default)]
struct Interaction {
  hovered: Option<Control>,
  pressed: Option<Control>,
  focused: Option<Control>,
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
      Screen::Composition => Node::new(Composition {
        reversed: self.reversed,
        interaction: self.interaction,
      }),
      Screen::EventsPortals => Node::new(EventsPortals {
        active: self.event_active,
        trace: self.event_trace.clone(),
        interaction: self.interaction,
      }),
      Screen::StateIdentity => Node::new(state_identity::StateIdentity),
    };
    VisualElement::new()
      .name("sample-shell")
      .style(design_system::root())
      .child(Navigation {
        screen: self.screen,
        interaction: self.interaction,
      })
      .child(page)
  }
}

impl Component for Navigation {
  fn render(&self) -> impl Render {
    VisualElement::new()
      .name("navigation")
      .style(design_system::navigation())
      .child(Label::new("REACTANT").style(design_system::brand()))
      .child(self::interactive_button(
        "01  COMPOSITION",
        "composition-navigation",
        design_system::navigation_item(
          self.screen == Screen::Composition,
          self::control_state(self.interaction, Control::CompositionNavigation),
        ),
        Control::CompositionNavigation,
        |game| game.screen = Screen::Composition,
      ))
      .child(self::interactive_button(
        "02  EVENTS & PORTALS",
        "events-navigation",
        design_system::navigation_item(
          self.screen == Screen::EventsPortals,
          self::control_state(self.interaction, Control::EventsNavigation),
        ),
        Control::EventsNavigation,
        |game| game.screen = Screen::EventsPortals,
      ))
      .child(self::interactive_button(
        "03  STATE & IDENTITY",
        "state-navigation",
        design_system::navigation_item(
          self.screen == Screen::StateIdentity,
          self::control_state(self.interaction, Control::StateNavigation),
        ),
        Control::StateNavigation,
        |game| game.screen = Screen::StateIdentity,
      ))
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
      .child(self::interactive_button(
        if self.reversed { "RESTORE" } else { "REORDER" },
        "composition-action",
        design_system::primary_action(self::control_state(
          self.interaction,
          Control::CompositionAction,
        )),
        Control::CompositionAction,
        |game| game.reversed = !game.reversed,
      ))
      .child(Fragment::new(
        Specimen::new()
          .child(composition_badges(self.reversed))
          .heading("Owned components".to_owned()),
      ))
  }
}

impl Component for EventsPortals {
  fn render(&self) -> impl Render {
    let status = if self.active {
      Node::new(
        VisualElement::new()
          .name("events-status")
          .style(design_system::event_route())
          .child(
            Label::new("CAPTURE").style(design_system::event_step(self.trace.contains(&"CAPTURE"))),
          )
          .child(Label::new(">").style(design_system::event_arrow()))
          .child(
            Label::new("TARGET").style(design_system::event_step(self.trace.contains(&"TARGET"))),
          )
          .child(Label::new(">").style(design_system::event_arrow()))
          .child(
            Label::new("BUBBLE").style(design_system::event_step(self.trace.contains(&"BUBBLE"))),
          ),
      )
    } else {
      Node::new(
        Label::new("READY")
          .name("events-status")
          .style(design_system::event_ready()),
      )
    };
    VisualElement::new()
      .name("events-canvas")
      .style(design_system::canvas())
      .child(Label::new("EVENTS & PORTALS").style(design_system::eyebrow()))
      .child(
        Label::new("Follow the logical path")
          .name("events-title")
          .style(design_system::title()),
      )
      .child(
        VisualElement::new()
          .name("event-route")
          .style(design_system::specimen())
          .child(Label::new("Propagation").style(design_system::specimen_title()))
          .child(self::interactive_button(
            if self.active { "RESTORE" } else { "RUN EVENT" },
            "events-action",
            design_system::primary_action(self::control_state(
              self.interaction,
              Control::EventsAction,
            )),
            Control::EventsAction,
            |game| {
              game.event_active = !game.event_active;
              if game.event_active {
                game.event_trace.push("TARGET");
              } else {
                game.event_trace.clear();
              }
            },
          ))
          .child(status)
          .on_click_capture(|game: &mut Game| {
            game.event_trace.clear();
            game.event_trace.push("CAPTURE");
          })
          .on_click(|game: &mut Game| game.event_trace.push("BUBBLE")),
      )
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
      text: "01  Required props",
    },
    Badge {
      text: "02  Structural values",
    },
    Badge {
      text: "03  Primitive children",
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

fn interactive_button(
  text: &'static str,
  name: &'static str,
  style: Style,
  control: Control,
  click: impl Fn(&mut Game) + 'static,
) -> impl Render {
  Button::new(text)
    .name(name)
    .style(style)
    .on_pointer_enter(move |game: &mut Game| game.interaction.hovered = Some(control))
    .on_pointer_leave(move |game: &mut Game| {
      if game.interaction.hovered == Some(control) {
        game.interaction.hovered = None;
      }
      if game.interaction.pressed == Some(control) {
        game.interaction.pressed = None;
      }
    })
    .on_pointer_down(move |game: &mut Game| game.interaction.pressed = Some(control))
    .on_pointer_up(move |game: &mut Game| {
      if game.interaction.pressed == Some(control) {
        game.interaction.pressed = None;
      }
    })
    .on_focus(move |game: &mut Game| game.interaction.focused = Some(control))
    .on_blur(move |game: &mut Game| {
      if game.interaction.focused == Some(control) {
        game.interaction.focused = None;
      }
    })
    .on_click(click)
}

fn control_state(interaction: Interaction, control: Control) -> design_system::ControlState {
  if interaction.pressed == Some(control) {
    return design_system::ControlState::Pressed;
  }
  if interaction.focused == Some(control) {
    return design_system::ControlState::Focused;
  }
  if interaction.hovered == Some(control) {
    return design_system::ControlState::Hovered;
  }
  design_system::ControlState::Resting
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
