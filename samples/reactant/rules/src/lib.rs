//! Native Rust engine for the standalone Reactant sample.

mod context_memo;
mod design_system;
mod effects_stores;
mod events_portals;
mod resources_boundaries;
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
  /// Logical context inheritance and memoization.
  ContextMemo,
  /// Passive effects and external stores.
  EffectsStores,
  /// Fallible rendering and resource recovery.
  ResourcesBoundaries,
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
    .style(design_system::root(false));
  let mut reactant = Reactant::new(IdleSpawner);
  let event_overlay = reactant.create_portal_target();
  reactant.register_root(document.clone(), move |game: &Game| Shell {
    screen: game.screen,
    reversed: game.reversed,
    event_active: game.event_active,
    event_trace: game.event_trace.clone(),
    event_overlay: event_overlay.clone(),
    context_overridden: game.context_overridden,
    context_unrelated: game.context_unrelated,
    effects_enabled: game.effects_enabled,
    boundary_failed: game.boundary_failed,
    boundary_retry_revision: game.boundary_retry_revision,
    boundary_reports: game.boundary_reports,
    store: match game.store_phase {
      effects_stores::StorePhase::Primary => game.primary_store.clone(),
      _ => game.secondary_store.clone(),
    },
    store_phase: game.store_phase,
    interaction: game.interaction,
    compact: game.compact,
  });
  Ok(ReactantEngine {
    session_id: SessionId::new_v4(),
    game: Game {
      screen: Screen::Composition,
      reversed: false,
      event_active: false,
      event_trace: Vec::new(),
      context_overridden: false,
      context_unrelated: 0,
      effects_enabled: false,
      boundary_failed: false,
      boundary_retry_revision: 0,
      boundary_reports: 0,
      primary_store: effects_stores::SampleStore::new("SOURCE A", 12),
      secondary_store: effects_stores::SampleStore::new("SOURCE B", 40),
      store_phase: effects_stores::StorePhase::Primary,
      interaction: Interaction::default(),
      compact: false,
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

  fn connect(&mut self, message: Connect) -> Result<Response, EngineError> {
    self.session_id = SessionId::new_v4();
    self.game.compact = message.screen.width < 1_100;
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
    let commit = self
      .reactant
      .poll(&mut self.game)
      .expect("sample poll should succeed");
    if commit.is_empty() {
      return Ok(None);
    }
    Ok(Some(
      Response::empty(self.session_id).append_reactant(commit),
    ))
  }
}

struct Game {
  screen: Screen,
  reversed: bool,
  event_active: bool,
  event_trace: Vec<&'static str>,
  context_overridden: bool,
  context_unrelated: u8,
  effects_enabled: bool,
  boundary_failed: bool,
  boundary_retry_revision: u32,
  boundary_reports: u32,
  primary_store: effects_stores::SampleStore,
  secondary_store: effects_stores::SampleStore,
  store_phase: effects_stores::StorePhase,
  interaction: Interaction,
  compact: bool,
}

struct IdleSpawner;

struct Shell {
  screen: Screen,
  reversed: bool,
  event_active: bool,
  event_trace: Vec<&'static str>,
  event_overlay: PortalTarget,
  context_overridden: bool,
  context_unrelated: u8,
  effects_enabled: bool,
  boundary_failed: bool,
  boundary_retry_revision: u32,
  boundary_reports: u32,
  store: effects_stores::SampleStore,
  store_phase: effects_stores::StorePhase,
  interaction: Interaction,
  compact: bool,
}

struct Navigation {
  screen: Screen,
  interaction: Interaction,
  compact: bool,
}

struct Composition {
  reversed: bool,
  interaction: Interaction,
  compact: bool,
}

struct Badge {
  text: &'static str,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Control {
  CompositionNavigation,
  EventsNavigation,
  StateNavigation,
  ContextNavigation,
  EffectsNavigation,
  ResourcesNavigation,
  CompositionAction,
  EventsAction,
  ContextAction,
  ContextUnrelatedAction,
  EffectsAction,
  StoreAction,
  BoundaryAction,
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
        compact: self.compact,
      }),
      Screen::EventsPortals => Node::new(events_portals::EventsPortals {
        active: self.event_active,
        trace: self.event_trace.clone(),
        overlay: self.event_overlay.clone(),
        interaction: self.interaction,
        compact: self.compact,
      }),
      Screen::StateIdentity => Node::new(state_identity::StateIdentity {
        compact: self.compact,
      }),
      Screen::ContextMemo => Node::new(context_memo::ContextMemo {
        overridden: self.context_overridden,
        unrelated: self.context_unrelated,
        interaction: self::control_state(self.interaction, Control::ContextAction),
        unrelated_interaction: self::control_state(
          self.interaction,
          Control::ContextUnrelatedAction,
        ),
        compact: self.compact,
      }),
      Screen::EffectsStores => Node::new(effects_stores::EffectsStores {
        enabled: self.effects_enabled,
        effect_interaction: self::control_state(self.interaction, Control::EffectsAction),
        store: self.store.clone(),
        store_phase: self.store_phase,
        store_interaction: self::control_state(self.interaction, Control::StoreAction),
        compact: self.compact,
      }),
      Screen::ResourcesBoundaries => Node::new(resources_boundaries::ResourcesBoundaries {
        failed: self.boundary_failed,
        retry_revision: self.boundary_retry_revision,
        reports: self.boundary_reports,
        interaction: self.interaction,
        compact: self.compact,
      }),
    };
    VisualElement::new()
      .name("sample-shell")
      .style(design_system::root(self.compact))
      .child(Navigation {
        screen: self.screen,
        interaction: self.interaction,
        compact: self.compact,
      })
      .child(page)
      .on_geometry_changed_event(|game: &mut Game, event| {
        game.compact = event.payload().current.width < 1_100.0;
      })
  }
}

impl Component for Navigation {
  fn render(&self) -> impl Render {
    VisualElement::new()
      .name("navigation")
      .style(design_system::navigation(self.compact))
      .child(Label::new("REACTANT").style(design_system::brand(self.compact)))
      .child(
        VisualElement::new()
          .name("navigation-items")
          .style(design_system::navigation_items(self.compact))
          .child(self::interactive_button(
            if self.compact {
              "01  Build"
            } else {
              "01  COMPOSITION"
            },
            "composition-navigation",
            design_system::navigation_item(
              self.screen == Screen::Composition,
              self::control_state(self.interaction, Control::CompositionNavigation),
              self.compact,
            ),
            Control::CompositionNavigation,
            |game| game.screen = Screen::Composition,
          ))
          .child(self::interactive_button(
            if self.compact {
              "02  Events"
            } else {
              "02  EVENTS & PORTALS"
            },
            "events-navigation",
            design_system::navigation_item(
              self.screen == Screen::EventsPortals,
              self::control_state(self.interaction, Control::EventsNavigation),
              self.compact,
            ),
            Control::EventsNavigation,
            |game| game.screen = Screen::EventsPortals,
          ))
          .child(self::interactive_button(
            if self.compact {
              "03  State"
            } else {
              "03  STATE & IDENTITY"
            },
            "state-navigation",
            design_system::navigation_item(
              self.screen == Screen::StateIdentity,
              self::control_state(self.interaction, Control::StateNavigation),
              self.compact,
            ),
            Control::StateNavigation,
            |game| game.screen = Screen::StateIdentity,
          ))
          .child(self::interactive_button(
            if self.compact {
              "04  Context"
            } else {
              "04  CONTEXT & MEMO"
            },
            "context-navigation",
            design_system::navigation_item(
              self.screen == Screen::ContextMemo,
              self::control_state(self.interaction, Control::ContextNavigation),
              self.compact,
            ),
            Control::ContextNavigation,
            |game| game.screen = Screen::ContextMemo,
          ))
          .child(self::interactive_button(
            if self.compact {
              "05  Effects"
            } else {
              "05  EFFECTS & STORES"
            },
            "effects-navigation",
            design_system::navigation_item(
              self.screen == Screen::EffectsStores,
              self::control_state(self.interaction, Control::EffectsNavigation),
              self.compact,
            ),
            Control::EffectsNavigation,
            |game| game.screen = Screen::EffectsStores,
          ))
          .child(self::interactive_button(
            "06  RESOURCES",
            "resources-navigation",
            design_system::navigation_item(
              self.screen == Screen::ResourcesBoundaries,
              self::control_state(self.interaction, Control::ResourcesNavigation),
              self.compact,
            ),
            Control::ResourcesNavigation,
            |game| game.screen = Screen::ResourcesBoundaries,
          )),
      )
  }
}

impl Component for Composition {
  fn render(&self) -> impl Render {
    VisualElement::new()
      .name("composition-canvas")
      .style(design_system::canvas(self.compact))
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
    .on_pointer_cancel(move |game: &mut Game| {
      if game.interaction.pressed == Some(control) {
        game.interaction.pressed = None;
      }
    })
    .on_pointer_capture_out(move |game: &mut Game| {
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
    .on_click(move |game: &mut Game| {
      game.interaction.hovered = None;
      game.interaction.pressed = None;
      game.interaction.focused = None;
      click(game);
    })
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
