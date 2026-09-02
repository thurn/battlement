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
  /// Unity-local gestures, constrained drag, momentum, scroll, and viewport state.
  GesturesDrag,
  /// Public Flex, Grid, Stack, sticky, popover, and modal application flow.
  LayoutGallery,
  /// Native layout projection, shared handoffs, and drag reorder.
  LayoutReorder,
  /// Complex public Motion compositions and reduced-motion behavior.
  ComposedEffects,
  /// Fixed mixed-layout release workload and native layout diagnostics.
  LayoutPerformance,
  /// Fixed release workloads and runtime diagnostics.
  MotionPerformance,
}

impl Screen {
  /// Every screen in navigation order.
  pub const ALL: [Self; 20] = [
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
    Self::GesturesDrag,
    Self::LayoutGallery,
    Self::LayoutReorder,
    Self::ComposedEffects,
    Self::LayoutPerformance,
    Self::MotionPerformance,
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
      Self::GesturesDrag => "gestures-drag",
      Self::LayoutGallery => "layout-gallery",
      Self::LayoutReorder => "layout-reorder",
      Self::ComposedEffects => "composed-effects",
      Self::LayoutPerformance => "layout-performance",
      Self::MotionPerformance => "motion-performance",
    }
  }
}
