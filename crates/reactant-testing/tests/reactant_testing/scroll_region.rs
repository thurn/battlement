//! Accessible scrolling follows native position and changing measured content.

use std::num::NonZeroU64;

use battlement::{
  AccessibilityScrollDirection, DisplayId, ElementGeometry, GeometryGeneration,
  GeometryObservationBatch, GeometryObservationResult, GeometryObservationTarget,
  GeometryObservationValue, GeometryValue, ObjectId, Projective2, Rect, ScrollEvent, Style,
  UiAccessibilityAction, UiAccessibilityActionEvent, UiEvent, UiEventBody, Vector, ViewportRect,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{components::ScrollRegion, host::View, testing::App};
use reactant_testing::Display;
use trox::tx;

#[test]
fn measured_region_tracks_native_scroll_and_clamps_after_resize() {
  let app = App::new("scroll/scene").ui(
    ScrollRegion::new(tx("Choices", "Scrollable choices in the test."))
      .host_name("choices")
      .style(Style::new().width(300).height(200))
      .child(View::new().style(Style::new().height(1000))),
  );
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("scroll/scene");
  let mut display = Display::connect(app, assets);
  let id = display.semantic_node("Choices").object_id;
  self::event(
    &mut display,
    id,
    UiEventBody::ScrollChanged(ScrollEvent {
      offset: Vector::new(0.0, 160.0),
    }),
  );
  self::measure(&mut display, id, 1, 200.0, 1000.0);
  assert_eq!(
    display.semantic_node("Choices").actions.scroll,
    [
      AccessibilityScrollDirection::Forward,
      AccessibilityScrollDirection::Backward
    ],
    "native focus scrolling before the first measurement must survive hydration"
  );
  self::event(
    &mut display,
    id,
    UiEventBody::ScrollChanged(ScrollEvent {
      offset: Vector::new(0.0, 0.0),
    }),
  );
  assert_eq!(
    display.semantic_node("Choices").actions.scroll,
    [AccessibilityScrollDirection::Forward]
  );
  for _ in 0..5 {
    self::event(
      &mut display,
      id,
      UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
        backend_generation: 0,
        action: UiAccessibilityAction::ScrollForward,
      }),
    );
  }
  assert_eq!(
    display.semantic_node("Choices").actions.scroll,
    [AccessibilityScrollDirection::Backward]
  );
  self::event(
    &mut display,
    id,
    UiEventBody::ScrollChanged(ScrollEvent {
      offset: Vector::new(0.0, 0.0),
    }),
  );
  assert_eq!(
    display.semantic_node("Choices").actions.scroll,
    [AccessibilityScrollDirection::Forward]
  );
  self::event(
    &mut display,
    id,
    UiEventBody::ScrollChanged(ScrollEvent {
      offset: Vector::new(0.0, 800.0),
    }),
  );
  self::measure(&mut display, id, 2, 900.0, 1000.0);
  assert_eq!(
    display.semantic_node("Choices").actions.scroll,
    [AccessibilityScrollDirection::Backward]
  );
  self::measure(&mut display, id, 3, 1000.0, 1000.0);
  assert!(display.semantic_node("Choices").actions.scroll.is_empty());
  self::measure(&mut display, id, 4, 200.0, 1000.0);
  assert_eq!(
    display.semantic_node("Choices").actions.scroll,
    [AccessibilityScrollDirection::Forward]
  );
}

fn event(display: &mut Display<App>, target_id: ObjectId, body: UiEventBody) {
  display.deliver_ui_event(UiEvent {
    target_id,
    cancelable: true,
    default_prevented: false,
    body,
  });
  display.settle();
}

fn measure(
  display: &mut Display<App>,
  viewport: ObjectId,
  generation: u64,
  height: f64,
  extent: f64,
) {
  let changed = display
    .geometry_registry()
    .iter()
    .map(|(observation_id, target)| {
      let GeometryObservationTarget::UiElement { object_id } = target else {
        panic!("unexpected geometry target")
      };
      GeometryObservationValue {
        observation_id: *observation_id,
        result: GeometryObservationResult::Current(GeometryValue::Element(self::geometry(
          viewport,
          if *object_id == viewport {
            height
          } else {
            extent
          },
        ))),
      }
    })
    .collect();
  display.deliver_geometry(GeometryObservationBatch {
    generation: GeometryGeneration(NonZeroU64::new(generation).unwrap()),
    changed,
  });
  display.settle();
}

fn geometry(panel_id: ObjectId, height: f64) -> ElementGeometry {
  let identity = Projective2 {
    m11: 1.0,
    m12: 0.0,
    m13: 0.0,
    m21: 0.0,
    m22: 1.0,
    m23: 0.0,
    m31: 0.0,
    m32: 0.0,
    m33: 1.0,
  };
  ElementGeometry {
    layout: Rect::new(0.0, 0.0, 300.0, height),
    viewport_bound: ViewportRect {
      x: 0.0,
      y: 0.0,
      width: 300.0,
      height,
      display_id: DisplayId(0),
    },
    viewport_from_local: identity,
    viewport_from_parent: identity,
    panel_id,
  }
}
