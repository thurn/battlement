use std::{
  collections::BTreeSet,
  num::NonZeroU64,
  sync::Arc,
  time::{Duration, Instant},
};

use battlement::{
  Color, Connect, Display, DisplayId, DisplayOrientation, ElementGeometry, FlexDirection,
  FocusEvent, GeometryGeneration, GeometryObservation, GeometryObservationBatch,
  GeometryObservationResult, GeometryObservationTarget, GeometryObservationValue,
  GeometryUnavailable, GeometryValue, KeyModifiers, Length, LengthOrAuto, ObjectId,
  OverlayPlacement, PanelPoint, PointerBoundaryEvent, PointerButton, PointerButtonEvent,
  PointerType, PreparedAsset, Projective2, Prop, Rect, ScreenSize, StaggerDirection, StyleValue,
  UiElement, UiElementKind, UiEvent, UiEventBody, UiVisualElementProperties, VariantWhen, Vector,
  ViewportGeometry, ViewportPoint, ViewportRect, WorldBoundsGeometry, WorldPointGeometry,
};
use battlement_fake::{assets::FakeAssetCatalog, client::ui::UiClient, journal::ExecutedCommand};
use battlement_rules::{
  CONTENT_SCENE, MOTION_AUDIO_CLIP, MOTION_MATERIAL, MOTION_TEXTURE, ROOT_ID, application,
  generated_asset_addresses,
};

const EVENTS_WORD_BUDGET: usize = 20;
const STATE_WORD_BUDGET: usize = 24;
const CONTEXT_WORD_BUDGET: usize = 24;
const EFFECTS_WORD_BUDGET: usize = 22;
const RESOURCES_WORD_BUDGET: usize = 18;
const REFS_WORD_BUDGET: usize = 48;

use reactant_testing::Display as ReactantDisplay;

type ReactantEngine = reactant::ApplicationEngine;

fn mount() -> ReactantDisplay {
  ReactantDisplay::mount(application, catalog())
}

fn mount_with(connect: Connect) -> ReactantDisplay {
  ReactantDisplay::mount_with(application, catalog(), connect)
}

#[path = "composition/layout_motion_flows.rs"]
mod layout_motion_flows;
#[path = "composition/state_effect_resource_flows.rs"]
mod state_effect_resource_flows;

fn navigate_brand(client: &mut ReactantDisplay, navigation_names: &[&str]) {
  for navigation in navigation_names {
    let target = find_named(&client.ui(), ROOT_ID, navigation);
    client.ui().send_event(UiEvent::click(
      target,
      battlement::ClickEvent::pointer(
        0,
        PanelPoint::default(),
        PointerButton::Left,
        1,
        KeyModifiers::default(),
      ),
    ));
  }
}

fn catalog() -> Arc<FakeAssetCatalog> {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene(CONTENT_SCENE);
  catalog.add_textures(generated_asset_addresses());
  catalog.add_material(MOTION_MATERIAL);
  catalog.add_texture(MOTION_TEXTURE);
  catalog.add_audio_clip(MOTION_AUDIO_CLIP);
  Arc::new(catalog)
}

fn find_named(ui: &UiClient<'_, ReactantEngine>, root: ObjectId, expected: &str) -> ObjectId {
  let mut pending = vec![root];
  while let Some(object_id) = pending.pop() {
    let element = ui.element(object_id);
    if element.name() == Some(expected) {
      return object_id;
    }
    pending.extend(element.children());
  }
  panic!("missing UI element named {expected}");
}

fn wait_for_named(client: &mut ReactantDisplay, root: ObjectId, expected: &str) -> ObjectId {
  let deadline = Instant::now() + Duration::from_secs(2);
  loop {
    let ui = client.ui();
    let mut pending = vec![root];
    while let Some(object_id) = pending.pop() {
      let element = ui.element(object_id);
      if element.name() == Some(expected) {
        return object_id;
      }
      pending.extend(element.children());
    }
    assert!(
      Instant::now() < deadline,
      "timed out waiting for {expected}"
    );
    std::thread::yield_now();
    client.poll();
  }
}
