use crate::{
  Screen, animation_validation, composed_effects, effects_stores, gestures_drag, layout_gallery,
  layout_performance, layout_reorder, motion_performance, physical_motion, presence_lifecycle,
  styles_decorations, values_time_controls, variants_orchestration,
};
use std::env;
/// State owned by the Reactant demonstration screens.
pub struct Game {
  pub(crate) screen: Screen,
  pub(crate) reversed: bool,
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

#[derive(Clone, Copy, Default)]
pub(crate) struct Interaction {
  pub(crate) hovered: Option<Control>,
  pub(crate) pressed: Option<Control>,
  pub(crate) focused: Option<Control>,
}

impl Game {
  /// Returns the currently selected demonstration screen.
  pub fn screen(&self) -> Screen {
    self.screen
  }
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
