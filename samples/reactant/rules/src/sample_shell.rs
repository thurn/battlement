use crate::{
  Control, Game, Interaction, Screen, animation_validation, assets, composed_effects, context_memo,
  design_system, effects_stores, events_portals, gestures_drag, layout_gallery, layout_performance,
  layout_reorder, motion_performance, physical_motion, presence_lifecycle, refs_geometry,
  resources_boundaries, state_identity, styles_decorations, values_time_controls,
  variants_orchestration,
};
use crate::{
  composition::Composition, controls, navigation::Navigation, preview_resource::Preview,
};
use battlement_reactant::prelude::*;

#[builder]
pub(crate) struct Shell {
  #[builder(required)]
  screen: Screen,
  reversed: bool,
  event_active: bool,
  event_trace: Vec<&'static str>,
  #[builder(required)]
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
  gestures_drag: gestures_drag::GesturesDragState,
  layout_gallery: layout_gallery::LayoutGalleryState,
  layout_reorder: layout_reorder::LayoutReorderState,
  composed_effects: composed_effects::ComposedEffectsState,
  layout_performance: layout_performance::LayoutPerformanceState,
  motion_performance: motion_performance::MotionPerformanceState,
  #[builder(required)]
  preview_resource: Preview,
  #[builder(required)]
  store: effects_stores::SampleStore,
  #[builder(required)]
  store_phase: effects_stores::StorePhase,
  interaction: Interaction,
}

pub(crate) fn view(game: &Game, event_overlay: PortalTarget, preview_resource: Preview) -> Shell {
  Shell::new()
    .screen(game.screen)
    .reversed(game.reversed)
    .event_active(game.event_active)
    .event_trace(game.event_trace.clone())
    .event_overlay(event_overlay.clone())
    .context_overridden(game.context_overridden)
    .context_unrelated(game.context_unrelated)
    .effects_enabled(game.effects_enabled)
    .boundary_failed(game.boundary_failed)
    .boundary_retry_revision(game.boundary_retry_revision)
    .refs_active(game.refs_active)
    .geometry_effect_runs(game.geometry_effect_runs)
    .assets_resized(game.assets_resized)
    .animation_validation(game.animation_validation.clone())
    .physical_motion(game.physical_motion.clone())
    .styles_decorations(game.styles_decorations.clone())
    .variants_orchestration(game.variants_orchestration.clone())
    .presence_lifecycle(game.presence_lifecycle.clone())
    .values_time_controls(game.values_time_controls.clone())
    .gestures_drag(game.gestures_drag.clone())
    .layout_gallery(game.layout_gallery.clone())
    .layout_reorder(game.layout_reorder.clone())
    .composed_effects(game.composed_effects.clone())
    .layout_performance(game.layout_performance)
    .motion_performance(game.motion_performance.clone())
    .preview_resource(preview_resource.clone())
    .store(match game.store_phase {
      effects_stores::StorePhase::Primary => game.primary_store.clone(),
      _ => game.secondary_store.clone(),
    })
    .store_phase(game.store_phase)
    .interaction(game.interaction)
}

impl Component for Shell {
  fn render(&self) -> impl Render {
    let viewport = use_viewport_size();
    let width = viewport.width as f64;
    let compact = width < 1_100.0;
    let phone = width < 600.0;
    let page = match self.screen {
      Screen::Composition => Node::new(
        Composition::new()
          .reversed(self.reversed)
          .interaction(self.interaction)
          .compact(compact),
      ),
      Screen::EventsPortals => Node::new(
        events_portals::EventsPortals::new()
          .active(self.event_active)
          .trace(self.event_trace.clone())
          .overlay(self.event_overlay.clone())
          .interaction(self.interaction)
          .compact(compact),
      ),
      Screen::StateIdentity => Node::new(state_identity::StateIdentity::new().compact(compact)),
      Screen::ContextMemo => Node::new(
        context_memo::ContextMemo::new()
          .overridden(self.context_overridden)
          .unrelated(self.context_unrelated)
          .interaction(controls::control_state(
            self.interaction,
            Control::ContextAction,
          ))
          .unrelated_interaction(controls::control_state(
            self.interaction,
            Control::ContextUnrelatedAction,
          ))
          .compact(compact),
      ),
      Screen::EffectsStores => Node::new(
        effects_stores::EffectsStores::new()
          .enabled(self.effects_enabled)
          .effect_interaction(controls::control_state(
            self.interaction,
            Control::EffectsAction,
          ))
          .store(self.store.clone())
          .store_phase(self.store_phase)
          .store_interaction(controls::control_state(
            self.interaction,
            Control::StoreAction,
          ))
          .compact(compact),
      ),
      Screen::ResourcesBoundaries => Node::new(
        resources_boundaries::ResourcesBoundaries::new()
          .failed(self.boundary_failed)
          .retry_revision(self.boundary_retry_revision)
          .preview_resource(self.preview_resource.clone())
          .interaction(self.interaction)
          .compact(compact),
      ),
      Screen::RefsGeometry => Node::new(
        refs_geometry::RefsGeometry::new()
          .active(self.refs_active)
          .effect_runs(self.geometry_effect_runs)
          .interaction(self.interaction)
          .compact(compact),
      ),
      Screen::Assets => Node::new(
        assets::Assets::new()
          .resized(self.assets_resized)
          .interaction(self.interaction)
          .compact(compact),
      ),
      Screen::TargetsTimelines => Node::new(
        animation_validation::ValidationScreen::new()
          .state(self.animation_validation.clone())
          .compact(compact),
      ),
      Screen::PhysicalMotion => Node::new(
        physical_motion::PhysicalMotion::new()
          .state(self.physical_motion.clone())
          .compact(compact),
      ),
      Screen::StylesDecorations => Node::new(
        styles_decorations::StylesDecorations::new()
          .state(self.styles_decorations.clone())
          .compact(compact),
      ),
      Screen::VariantsOrchestration => Node::new(
        variants_orchestration::VariantsOrchestration::new()
          .state(self.variants_orchestration.clone())
          .compact(compact),
      ),
      Screen::PresenceLifecycle => Node::new(
        presence_lifecycle::PresenceLifecycle::new()
          .state(self.presence_lifecycle.clone())
          .compact(compact),
      ),
      Screen::ValuesTimeControls => Node::new(
        values_time_controls::ValuesTimeControls::new()
          .state(self.values_time_controls.clone())
          .compact(compact),
      ),
      Screen::GesturesDrag => Node::new(
        gestures_drag::GesturesDrag::new()
          .state(self.gestures_drag.clone())
          .compact(compact),
      ),
      Screen::LayoutGallery => Node::new(
        layout_gallery::LayoutGallery::new()
          .state(self.layout_gallery.clone())
          .compact(compact)
          .overlay(self.event_overlay.clone()),
      ),
      Screen::LayoutReorder => Node::new(
        layout_reorder::LayoutReorder::new()
          .state(self.layout_reorder.clone())
          .compact(compact),
      ),
      Screen::ComposedEffects => Node::new(
        composed_effects::ComposedEffects::new()
          .state(self.composed_effects.clone())
          .compact(compact),
      ),
      Screen::LayoutPerformance => Node::new(
        layout_performance::LayoutPerformance::new()
          .state(self.layout_performance)
          .overlay(self.event_overlay.clone()),
      ),
      Screen::MotionPerformance => Node::new(
        motion_performance::MotionPerformance::new()
          .state(self.motion_performance.clone())
          .compact(compact),
      ),
    };
    battlement_reactant::host::Stack::new()
      .style(design_system::root(compact))
      .child(
        battlement_reactant::host::View::new()
          .name("sample-shell")
          .style(design_system::root(compact))
          .child(
            Navigation::new()
              .screen(self.screen)
              .interaction(self.interaction)
              .compact(compact)
              .phone(phone),
          )
          .child(page),
      )
      .child(OverlayHost::new(self.event_overlay.clone()))
  }
}
