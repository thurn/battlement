//! Native Rust engine for the standalone Reactant sample.

use std::{
  collections::VecDeque,
  sync::{Arc, Mutex},
  task::{Context, Poll, Waker},
};

mod animation_validation;
mod assets;
mod context_memo;
mod design_system;
mod effects_stores;
mod events_portals;
mod physical_motion;
mod presence_lifecycle;
mod refs_geometry;
mod resources_boundaries;
mod state_identity;
mod styles_decorations;
#[cfg(test)]
mod tests;
mod values_time_controls;
mod variants_orchestration;

use battlement::{
  ActionBody, Batch, BatchId, CameraClearMode, CameraProjection, CameraState, ClientMessage, Color,
  Command, Connect, CoreErrorCode, GameObject, GameObjectKind, ObjectId, PanelScaleMode,
  PanelSettings, ParallelCommandGroup, ParentScene, PickingMode, PreparedAsset, Response,
  ResponseMessage, Scene, SceneId, SessionId, Snapshot, Style, TextureAddress, UiDocument,
  UiDocumentState, Vector3, object_id, scene_id,
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
const MISSING_GEOMETRY_TARGET_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000006");

/// Address of the sample's authored content scene.
pub const CONTENT_SCENE: &str = "reactant/content";
/// Address of the sample's prepared UI shader material.
pub const MOTION_MATERIAL: &str = "reactant/assets/motion-material";
/// Address of the sample's prepared audio-playhead pulse.
pub const MOTION_AUDIO_CLIP: battlement::AudioClipAddress = values_time_controls::AUDIO_CLIP;
/// Address of the sample's prepared motion texture.
pub const MOTION_TEXTURE: &str = "reactant/assets/texture";
/// Machine-readable registry derived from the Reactant screen inventory.
pub const DITTO_VISUAL_STATE_REGISTRY: &str = include_str!("../../ditto-visual-states.toml");
/// Stable identity of the projected world specimen.
pub const GEOMETRY_TARGET_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000005");
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
  /// Stable element refs and queued host actions.
  RefsGeometry,
  /// Generated advanced paint and resizable nine-slice assets.
  Assets,
  /// Typed Motion targets, timelines, repeats, and interruption.
  TargetsTimelines,
  /// Spring, inertia, velocity handoff, and playback outcomes.
  PhysicalMotion,
  /// CSS transitions, reusable animations, decorations, and advanced paint.
  StylesDecorations,
  /// Typed variants, logical propagation, and child orchestration.
  VariantsOrchestration,
  /// Retained exits, manual holds, and lifecycle ordering.
  PresenceLifecycle,
  /// Native motion values, time sources, audio transport, and imperative controls.
  ValuesTimeControls,
}

impl Screen {
  /// Every screen in navigation order.
  pub const ALL: [Self; 14] = [
    Self::Composition,
    Self::EventsPortals,
    Self::StateIdentity,
    Self::ContextMemo,
    Self::EffectsStores,
    Self::ResourcesBoundaries,
    Self::RefsGeometry,
    Self::Assets,
    Self::TargetsTimelines,
    Self::PhysicalMotion,
    Self::StylesDecorations,
    Self::VariantsOrchestration,
    Self::PresenceLifecycle,
    Self::ValuesTimeControls,
  ];

  /// Returns the canonical coverage registry key.
  pub const fn registry_key(self) -> &'static str {
    match self {
      Self::Composition => "composition",
      Self::EventsPortals => "events-portals",
      Self::StateIdentity => "state-identity",
      Self::ContextMemo => "context-memo",
      Self::EffectsStores => "effects-stores",
      Self::ResourcesBoundaries => "resources-boundaries",
      Self::RefsGeometry => "refs-geometry",
      Self::Assets => "assets",
      Self::TargetsTimelines => "targets-timelines",
      Self::PhysicalMotion => "physical-motion",
      Self::StylesDecorations => "styles-decorations",
      Self::VariantsOrchestration => "variants-orchestration",
      Self::PresenceLifecycle => "presence-lifecycle",
      Self::ValuesTimeControls => "values-time-controls",
    }
  }
}

/// Native Reactant sample rules engine.
pub struct ReactantEngine {
  session_id: SessionId,
  game: Game,
  reactant: Reactant<Game>,
  spawner: ManualSpawner,
  preview_resource: Resource<u32, u32>,
  document: UiDocument,
}

/// Creates the engine used by the Reactant sample.
pub fn create_engine() -> Result<ReactantEngine, EngineError> {
  animation_validation::fixture_registry()
    .validate()
    .expect("animation validation registry should be valid");
  let document = UiDocument::with_root_id(DOCUMENT_ID, ROOT_ID)
    .name("battlement-reactant")
    .picking_mode(PickingMode::Ignore)
    .style(design_system::root(false));
  let spawner = ManualSpawner::default();
  let mut reactant = Reactant::new(spawner.clone());
  let preview_resource = Resource::new(|key| async move { key });
  let view_resource = preview_resource.clone();
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
    refs_active: game.refs_active,
    geometry_effect_runs: game.geometry_effect_runs,
    assets_resized: game.assets_resized,
    animation_validation: game.animation_validation.clone(),
    physical_motion: game.physical_motion.clone(),
    styles_decorations: game.styles_decorations.clone(),
    variants_orchestration: game.variants_orchestration.clone(),
    presence_lifecycle: game.presence_lifecycle.clone(),
    values_time_controls: game.values_time_controls.clone(),
    preview_resource: view_resource.clone(),
    store: match game.store_phase {
      effects_stores::StorePhase::Primary => game.primary_store.clone(),
      _ => game.secondary_store.clone(),
    },
    store_phase: game.store_phase,
    interaction: game.interaction,
    compact: game.compact,
    phone: game.phone,
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
      refs_active: false,
      geometry_effect_runs: 0,
      assets_resized: false,
      animation_validation: animation_validation::ValidationUiState::default(),
      physical_motion: physical_motion::PhysicalMotionState::default(),
      styles_decorations: styles_decorations::StylesDecorationsState::default(),
      variants_orchestration: variants_orchestration::VariantsOrchestrationState::default(),
      presence_lifecycle: presence_lifecycle::PresenceLifecycleState::default(),
      values_time_controls: values_time_controls::ValuesTimeControlsState::default(),
      pending_commands: Vec::new(),
      resource_resolution_requested: false,
      resource_invalidation_requested: false,
      primary_store: effects_stores::SampleStore::new("SOURCE A", 12),
      secondary_store: effects_stores::SampleStore::new("SOURCE B", 40),
      store_phase: effects_stores::StorePhase::Primary,
      interaction: Interaction::default(),
      compact: false,
      phone: false,
    },
    reactant,
    spawner,
    preview_resource,
    document,
  })
}

/// Returns every linked generated texture address used by the sample gallery.
pub fn generated_asset_addresses() -> Vec<TextureAddress> {
  assets::addresses()
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
    self.game.phone = message.screen.width < 600;
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
    let commit = match action.body {
      ActionBody::VisualElement(event) => self
        .reactant
        .dispatch(&mut self.game, event)
        .expect("sample event dispatch should succeed"),
      ActionBody::GeometryObservations(batch) => self
        .reactant
        .observe_geometry(&mut self.game, batch)
        .expect("sample geometry observation should succeed"),
      ActionBody::MotionEvents(batch) => self
        .reactant
        .motion_events(&mut self.game, batch)
        .expect("sample Motion event dispatch should succeed"),
      _ => return Ok(Response::empty(self.session_id)),
    };
    if self.game.presence_lifecycle.take_reconnect_request() {
      let _ = commit.into_groups();
      return Ok(
        self
          .reactant
          .begin_session(&mut self.game)
          .expect("sample reconnect render should succeed")
          .into_response(snapshot(self.session_id, &self.document)),
      );
    }
    let mut response =
      Response::empty(self.session_id).append_reactant_for_action(action.action_id, commit);
    if !self.game.pending_commands.is_empty() {
      let mut batch = Batch::new(
        BatchId::new_v4(),
        self.session_id,
        vec![ParallelCommandGroup::new(std::mem::take(
          &mut self.game.pending_commands,
        ))],
      );
      batch.caused_by_action_id = Some(action.action_id);
      response.messages.push(ResponseMessage::Batch(batch));
    }
    if self.game.resource_invalidation_requested {
      self.game.resource_invalidation_requested = false;
      self.reactant.invalidate(&self.preview_resource, &1);
      response = response.append_reactant_for_action(
        action.action_id,
        self
          .reactant
          .poll(&mut self.game)
          .expect("sample resource invalidation should render"),
      );
    }
    if self.game.resource_resolution_requested {
      self.game.resource_resolution_requested = false;
      self.spawner.run_next();
      response = response.append_reactant_for_action(
        action.action_id,
        self
          .reactant
          .poll(&mut self.game)
          .expect("sample resource completion should render"),
      );
    }
    Ok(response)
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

impl Drop for ReactantEngine {
  fn drop(&mut self) {
    let _ = self.reactant.shutdown(&mut self.game).into_groups();
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
  refs_active: bool,
  geometry_effect_runs: u32,
  assets_resized: bool,
  animation_validation: animation_validation::ValidationUiState,
  physical_motion: physical_motion::PhysicalMotionState,
  styles_decorations: styles_decorations::StylesDecorationsState,
  variants_orchestration: variants_orchestration::VariantsOrchestrationState,
  presence_lifecycle: presence_lifecycle::PresenceLifecycleState,
  values_time_controls: values_time_controls::ValuesTimeControlsState,
  pending_commands: Vec<Command>,
  resource_resolution_requested: bool,
  resource_invalidation_requested: bool,
  primary_store: effects_stores::SampleStore,
  secondary_store: effects_stores::SampleStore,
  store_phase: effects_stores::StorePhase,
  interaction: Interaction,
  compact: bool,
  phone: bool,
}

#[derive(Clone, Default)]
struct ManualSpawner {
  tasks: Arc<Mutex<VecDeque<BoxFuture<'static, ()>>>>,
}

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
  refs_active: bool,
  geometry_effect_runs: u32,
  assets_resized: bool,
  animation_validation: animation_validation::ValidationUiState,
  physical_motion: physical_motion::PhysicalMotionState,
  styles_decorations: styles_decorations::StylesDecorationsState,
  variants_orchestration: variants_orchestration::VariantsOrchestrationState,
  presence_lifecycle: presence_lifecycle::PresenceLifecycleState,
  values_time_controls: values_time_controls::ValuesTimeControlsState,
  preview_resource: Resource<u32, u32>,
  store: effects_stores::SampleStore,
  store_phase: effects_stores::StorePhase,
  interaction: Interaction,
  compact: bool,
  phone: bool,
}

struct Navigation {
  screen: Screen,
  interaction: Interaction,
  compact: bool,
  phone: bool,
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
  RefsNavigation,
  AssetsNavigation,
  CompositionAction,
  EventsAction,
  ContextAction,
  ContextUnrelatedAction,
  EffectsAction,
  StoreAction,
  BoundaryAction,
  ResourceAction,
  RefsAction,
  AssetsAction,
  PreviousNavigation,
  NextNavigation,
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

impl Spawner for ManualSpawner {
  fn spawn(&self, task: BoxFuture<'static, ()>) -> SpawnedTask {
    self
      .tasks
      .lock()
      .expect("executor queue lock")
      .push_back(task);
    SpawnedTask::detached()
  }
}

impl ManualSpawner {
  fn run_next(&self) {
    let task = self
      .tasks
      .lock()
      .expect("executor queue lock")
      .pop_front()
      .expect("resource completion was requested without pending work");
    let mut task = task;
    let mut context = Context::from_waker(Waker::noop());
    assert_eq!(task.as_mut().poll(&mut context), Poll::Ready(()));
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
        preview_resource: self.preview_resource.clone(),
        interaction: self.interaction,
        compact: self.compact,
      }),
      Screen::RefsGeometry => Node::new(refs_geometry::RefsGeometry {
        active: self.refs_active,
        effect_runs: self.geometry_effect_runs,
        interaction: self.interaction,
        compact: self.compact,
      }),
      Screen::Assets => Node::new(assets::Assets {
        resized: self.assets_resized,
        interaction: self.interaction,
        compact: self.compact,
      }),
      Screen::TargetsTimelines => Node::new(animation_validation::ValidationScreen {
        state: self.animation_validation.clone(),
        compact: self.compact,
      }),
      Screen::PhysicalMotion => Node::new(physical_motion::PhysicalMotion {
        state: self.physical_motion.clone(),
        compact: self.compact,
      }),
      Screen::StylesDecorations => Node::new(styles_decorations::StylesDecorations {
        state: self.styles_decorations.clone(),
        compact: self.compact,
      }),
      Screen::VariantsOrchestration => Node::new(variants_orchestration::VariantsOrchestration {
        state: self.variants_orchestration.clone(),
        compact: self.compact,
      }),
      Screen::PresenceLifecycle => Node::new(presence_lifecycle::PresenceLifecycle {
        state: self.presence_lifecycle.clone(),
        compact: self.compact,
      }),
      Screen::ValuesTimeControls => Node::new(values_time_controls::ValuesTimeControls {
        state: self.values_time_controls.clone(),
        compact: self.compact,
      }),
    };
    battlement_reactant::host::View::new()
      .name("sample-shell")
      .style(design_system::root(self.compact))
      .child(Navigation {
        screen: self.screen,
        interaction: self.interaction,
        compact: self.compact,
        phone: self.phone,
      })
      .child(page)
      .on_geometry_changed_event(|game: &mut Game, event| {
        game.compact = event.payload().current.width < 1_100.0;
        game.phone = event.payload().current.width < 600.0;
      })
  }
}

impl Component for Navigation {
  fn render(&self) -> impl Render {
    if self.phone {
      return Node::new(
        battlement_reactant::host::View::new()
          .name("navigation")
          .style(design_system::phone_navigation())
          .child(battlement_reactant::host::Label::new("R").style(design_system::phone_brand()))
          .child(self::interactive_button(
            "<",
            "previous-navigation",
            design_system::phone_navigation_action(self::control_state(
              self.interaction,
              Control::PreviousNavigation,
            )),
            Control::PreviousNavigation,
            |game| game.screen = self::previous_screen(game.screen),
          ))
          .child(
            battlement_reactant::host::Label::new(self::phone_screen_name(self.screen))
              .name("phone-current-screen")
              .style(design_system::phone_navigation_label()),
          )
          .child(self::interactive_button(
            ">",
            "next-navigation",
            design_system::phone_navigation_action(self::control_state(
              self.interaction,
              Control::NextNavigation,
            )),
            Control::NextNavigation,
            |game| game.screen = self::next_screen(game.screen),
          )),
      );
    }
    Node::new(
      battlement_reactant::host::View::new()
        .name("navigation")
        .style(design_system::navigation(self.compact))
        .child(
          battlement_reactant::host::Label::new(if self.screen == Screen::TargetsTimelines {
            "VALUES & TIME"
          } else {
            "REACTANT"
          })
          .name(if self.screen == Screen::TargetsTimelines {
            "values-navigation"
          } else {
            "targets-timelines-navigation"
          })
          .style(design_system::brand(self.compact))
          .on_click(|game: &mut Game| {
            game.screen = if game.screen == Screen::TargetsTimelines {
              Screen::ValuesTimeControls
            } else {
              Screen::TargetsTimelines
            };
          }),
        )
        .child(
          battlement_reactant::host::View::new()
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
            ))
            .child(self::interactive_button(
              if self.compact {
                "07  Refs"
              } else {
                "07  REFS & GEOMETRY"
              },
              "refs-navigation",
              design_system::navigation_item(
                self.screen == Screen::RefsGeometry,
                self::control_state(self.interaction, Control::RefsNavigation),
                self.compact,
              ),
              Control::RefsNavigation,
              |game| game.screen = Screen::RefsGeometry,
            ))
            .child(self::interactive_button(
              "08  ASSETS",
              "assets-navigation",
              design_system::navigation_item(
                self.screen == Screen::Assets,
                self::control_state(self.interaction, Control::AssetsNavigation),
                self.compact,
              ),
              Control::AssetsNavigation,
              |game| game.screen = Screen::Assets,
            )),
        ),
    )
  }
}

impl Component for Composition {
  fn render(&self) -> impl Render {
    battlement_reactant::host::View::new()
      .name("composition-canvas")
      .style(design_system::canvas(self.compact))
      .child(battlement_reactant::host::Label::new("COMPOSITION").style(design_system::eyebrow()))
      .child(
        battlement_reactant::host::Label::new("Build declaratively")
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
    battlement_reactant::host::View::new()
      .style(design_system::badge())
      .child(battlement_reactant::host::Label::new(self.text).style(design_system::badge_text()))
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
    battlement_reactant::host::View::new()
      .name("composition-specimen")
      .style(design_system::specimen())
      .child(
        battlement_reactant::host::Label::new(self.required.0.clone())
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
    battlement_reactant::host::View::new()
      .name("composition-badges")
      .style(design_system::badge_row())
      .child(Fragment::new(badges)),
  )
}

fn previous_screen(screen: Screen) -> Screen {
  match screen {
    Screen::Composition => Screen::ValuesTimeControls,
    Screen::EventsPortals => Screen::Composition,
    Screen::StateIdentity => Screen::EventsPortals,
    Screen::ContextMemo => Screen::StateIdentity,
    Screen::EffectsStores => Screen::ContextMemo,
    Screen::ResourcesBoundaries => Screen::EffectsStores,
    Screen::RefsGeometry => Screen::ResourcesBoundaries,
    Screen::Assets => Screen::RefsGeometry,
    Screen::TargetsTimelines => Screen::Assets,
    Screen::PhysicalMotion => Screen::TargetsTimelines,
    Screen::StylesDecorations => Screen::PhysicalMotion,
    Screen::VariantsOrchestration => Screen::StylesDecorations,
    Screen::PresenceLifecycle => Screen::VariantsOrchestration,
    Screen::ValuesTimeControls => Screen::PresenceLifecycle,
  }
}

fn next_screen(screen: Screen) -> Screen {
  match screen {
    Screen::Composition => Screen::EventsPortals,
    Screen::EventsPortals => Screen::StateIdentity,
    Screen::StateIdentity => Screen::ContextMemo,
    Screen::ContextMemo => Screen::EffectsStores,
    Screen::EffectsStores => Screen::ResourcesBoundaries,
    Screen::ResourcesBoundaries => Screen::RefsGeometry,
    Screen::RefsGeometry => Screen::Assets,
    Screen::Assets => Screen::TargetsTimelines,
    Screen::TargetsTimelines => Screen::PhysicalMotion,
    Screen::PhysicalMotion => Screen::StylesDecorations,
    Screen::StylesDecorations => Screen::VariantsOrchestration,
    Screen::VariantsOrchestration => Screen::PresenceLifecycle,
    Screen::PresenceLifecycle => Screen::ValuesTimeControls,
    Screen::ValuesTimeControls => Screen::Composition,
  }
}

fn phone_screen_name(screen: Screen) -> &'static str {
  match screen {
    Screen::Composition => "01 COMPOSITION",
    Screen::EventsPortals => "02 EVENTS",
    Screen::StateIdentity => "03 STATE",
    Screen::ContextMemo => "04 CONTEXT",
    Screen::EffectsStores => "05 EFFECTS",
    Screen::ResourcesBoundaries => "06 RESOURCES",
    Screen::RefsGeometry => "07 GEOMETRY",
    Screen::Assets => "08 ASSETS",
    Screen::TargetsTimelines => "09 TARGETS & TIMELINES",
    Screen::PhysicalMotion => "10 PHYSICAL MOTION",
    Screen::StylesDecorations => "11 STYLES & DECORATIONS",
    Screen::VariantsOrchestration => "12 VARIANTS & ORCHESTRATION",
    Screen::PresenceLifecycle => "13 PRESENCE & LIFECYCLE",
    Screen::ValuesTimeControls => "14 VALUES, TIME & CONTROLS",
  }
}

fn interactive_button(
  text: &'static str,
  name: &'static str,
  style: Style,
  control: Control,
  click: impl Fn(&mut Game) + 'static,
) -> Button {
  battlement_reactant::host::Button::new(text)
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
  let camera = GameObject::new(
    CAMERA_ID,
    CameraState::new()
      .projection(CameraProjection::Perspective)
      .field_of_view(50.0)
      .clear_mode(CameraClearMode::SolidColor)
      .clear_color(Color::rgb(0.012, 0.025, 0.045)),
  )
  .parent_scene(ParentScene::Persistent)
  .position(Vector3::new(0.0, 0.0, -10.0));
  let specimen = GameObject::new(
    GEOMETRY_TARGET_ID,
    GameObjectKind::Cube {
      materials: Vec::new(),
    },
  )
  .parent_scene(ParentScene::Persistent)
  .position(Vector3::new(3.2, -1.8, 0.0))
  .scale(Vector3::new(1.4, 1.4, 1.4));
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
    vec![
      PreparedAsset::scene(CONTENT_SCENE),
      PreparedAsset::material(MOTION_MATERIAL),
      PreparedAsset::texture(MOTION_TEXTURE),
      PreparedAsset::AudioClip(values_time_controls::AUDIO_CLIP),
    ],
    vec![Scene::new(SCENE_ID, CONTENT_SCENE)],
    vec![camera, specimen, ui_host],
    CAMERA_ID,
  )
}

battlement_native::export_engine!(create_engine);
