use crate::{
  Screen, animation_validation, composed_effects, effects_stores, gestures_drag, layout_gallery,
  layout_performance, layout_reorder, motion_performance, physical_motion, presence_lifecycle,
  styles_decorations, values_time_controls, variants_orchestration,
};
use reactant::{callback::Callback as EventCallback, prelude::*};
use std::{cell::RefCell, env, rc::Rc};
/// State owned by the Reactant demonstration screens.
#[derive(Clone, PartialEq)]
pub struct Game {
  pub(crate) screen: Screen,
  pub(crate) reversed: bool,
  pub(crate) identity_location: u32,
  pub(crate) event_active: bool,
  pub(crate) event_trace: Vec<&'static str>,
  pub(crate) context_overridden: bool,
  pub(crate) context_unrelated: u8,
  pub(crate) effects_enabled: bool,
  pub(crate) boundary_failed: bool,
  pub(crate) boundary_retry_revision: u32,
  pub(crate) refs_active: bool,
  pub(crate) geometry_effect_runs: u32,
  pub(crate) assets_resized: bool,
  pub(crate) animation_validation: animation_validation::ValidationUiState,
  pub(crate) physical_motion: physical_motion::PhysicalMotionState,
  pub(crate) styles_decorations: styles_decorations::StylesDecorationsState,
  pub(crate) variants_orchestration: variants_orchestration::VariantsOrchestrationState,
  pub(crate) presence_lifecycle: presence_lifecycle::PresenceLifecycleState,
  pub(crate) values_time_controls: values_time_controls::ValuesTimeControlsState,
  pub(crate) gestures_drag: gestures_drag::GesturesDragState,
  pub(crate) layout_gallery: layout_gallery::LayoutGalleryState,
  pub(crate) layout_reorder: layout_reorder::LayoutReorderState,
  pub(crate) composed_effects: composed_effects::ComposedEffectsState,
  pub(crate) layout_performance: layout_performance::LayoutPerformanceState,
  pub(crate) motion_performance: motion_performance::MotionPerformanceState,
  pub(crate) primary_store: effects_stores::SampleStore,
  pub(crate) secondary_store: effects_stores::SampleStore,
  pub(crate) store_phase: effects_stores::StorePhase,
  pub(crate) interaction: Interaction,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Control {
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

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub(crate) struct Interaction {
  pub(crate) hovered: Option<Control>,
  pub(crate) pressed: Option<Control>,
  pub(crate) focused: Option<Control>,
}

pub(crate) fn new() -> Game {
  let performance_profile = env::args().find_map(|argument| {
    argument
      .strip_prefix("--reactant-performance=")
      .map(str::to_owned)
  });
  let motion_profile = performance_profile
    .as_deref()
    .and_then(motion_performance::MotionPerformanceState::profiled);
  let layout_profile = performance_profile.as_deref() == Some("layout-mixed");
  Game {
    screen: if layout_profile {
      Screen::LayoutPerformance
    } else if motion_profile.is_some() {
      Screen::MotionPerformance
    } else {
      Screen::Composition
    },
    reversed: false,
    identity_location: 0,
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
    gestures_drag: gestures_drag::GesturesDragState::default(),
    layout_gallery: layout_gallery::LayoutGalleryState::default(),
    layout_reorder: layout_reorder::LayoutReorderState::default(),
    composed_effects: composed_effects::ComposedEffectsState::default(),
    layout_performance: layout_performance::LayoutPerformanceState::default(),
    motion_performance: motion_profile.unwrap_or_default(),
    primary_store: effects_stores::SampleStore::new("SOURCE A", 12),
    secondary_store: effects_stores::SampleStore::new("SOURCE B", 40),
    store_phase: effects_stores::StorePhase::Primary,
    interaction: Interaction::default(),
  }
}

/// Component-local observable state shared by the laboratory demonstrations.
#[derive(Clone)]
pub(crate) struct LaboratoryStore(Rc<RefCell<Game>>);

impl LaboratoryStore {
  pub(crate) fn new(game: Game) -> Self {
    Self(Rc::new(RefCell::new(game)))
  }

  pub(crate) fn snapshot(&self) -> Game {
    self.0.borrow().clone()
  }

  fn update(&self, update: impl FnOnce(&mut Game)) {
    update(&mut self.0.borrow_mut());
  }
}

impl PartialEq for LaboratoryStore {
  fn eq(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.0, &other.0)
  }
}

/// State facade shared with the laboratory's demonstration components.
#[derive(Clone, PartialEq)]
pub(crate) struct GameDispatch {
  store: LaboratoryStore,
  revision: StateSetter<u64>,
}

impl GameDispatch {
  pub(crate) fn new(store: LaboratoryStore, revision: StateSetter<u64>) -> Self {
    Self { store, revision }
  }

  pub(crate) fn snapshot(&self) -> Game {
    self.store.snapshot()
  }

  pub(crate) fn update(&self, update: impl FnOnce(&mut Game)) {
    self.store.update(update);
    self.revision.update(|revision| revision.wrapping_add(1));
  }
}

impl GameDispatch {
  /// Creates an application-independent callback that updates laboratory state.
  pub(crate) fn event<A: Clone + 'static>(
    &self,
    update: impl Fn(&mut Game, A) + 'static,
  ) -> EventCallback<A> {
    let dispatch = self.clone();
    let update = Rc::new(update);
    EventCallback::new(move |value: A| {
      update(&mut dispatch.store.0.borrow_mut(), value.clone());
      dispatch
        .revision
        .update(|revision| revision.wrapping_add(1));
    })
  }

  /// Creates a callback that borrows its native event while updating display state.
  pub(crate) fn borrowed_event<A: Clone + 'static, U>(
    &self,
    update: U,
  ) -> impl Fn(&mut (), &A) + use<A, U>
  where
    U: Fn(&mut Game, &A) + 'static,
  {
    let dispatch = self.clone();
    let update = Rc::new(update);
    move |_, value| {
      update(&mut dispatch.store.0.borrow_mut(), value);
      dispatch
        .revision
        .update(|revision| revision.wrapping_add(1));
    }
  }

  /// Creates a payload-free callback that updates laboratory state.
  pub(crate) fn action(&self, update: impl Fn(&mut Game) + 'static) -> EventCallback<()> {
    self.event(move |game, ()| update(game))
  }

  /// Creates a model-free application callback that updates laboratory state.
  pub(crate) fn app_action<U>(&self, update: U) -> impl Fn(&mut ()) + use<U>
  where
    U: Fn(&mut Game) + 'static,
  {
    let dispatch = self.clone();
    let update = Rc::new(update);
    move |_| {
      update(&mut dispatch.store.0.borrow_mut());
      dispatch
        .revision
        .update(|revision| revision.wrapping_add(1));
    }
  }
}

/// Returns the laboratory state dispatcher for a demonstration component.
pub(crate) fn use_game_dispatch() -> GameDispatch {
  use_required_context::<GameDispatch>()
}
