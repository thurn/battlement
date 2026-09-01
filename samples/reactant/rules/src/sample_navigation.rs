use crate::Screen;

pub(crate) fn previous(screen: Screen) -> Screen {
  match screen {
    Screen::Composition => Screen::MotionPerformance,
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
    Screen::GesturesDrag => Screen::ValuesTimeControls,
    Screen::LayoutReorder => Screen::GesturesDrag,
    Screen::ComposedEffects => Screen::LayoutReorder,
    Screen::MotionPerformance => Screen::ComposedEffects,
  }
}

pub(crate) fn next(screen: Screen) -> Screen {
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
    Screen::ValuesTimeControls => Screen::GesturesDrag,
    Screen::GesturesDrag => Screen::LayoutReorder,
    Screen::LayoutReorder => Screen::ComposedEffects,
    Screen::ComposedEffects => Screen::MotionPerformance,
    Screen::MotionPerformance => Screen::Composition,
  }
}

pub(crate) fn phone_name(screen: Screen) -> &'static str {
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
    Screen::GesturesDrag => "15 GESTURES & DRAG",
    Screen::LayoutReorder => "16 LAYOUT & REORDER",
    Screen::ComposedEffects => "17 COMPOSED EFFECTS",
    Screen::MotionPerformance => "18 MOTION PERFORMANCE",
  }
}
