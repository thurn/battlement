use std::{cell::RefCell, collections::BTreeSet, num::NonZeroU64, rc::Rc, sync::Arc};

use battlement::{
  ActionId, Color, Connect, Display, DisplayId, DisplayOrientation, ElementGeometry, FlexDirection,
  FocusEvent, GeometryGeneration, GeometryObservation, GeometryObservationBatch,
  GeometryObservationResult, GeometryObservationTarget, GeometryObservationValue,
  GeometryUnavailable, GeometryValue, KeyModifiers, Length, LengthOrAuto, ObjectId,
  OverlayPlacement, PanelPoint, PointerBoundaryEvent, PointerButton, PointerButtonEvent,
  PointerType, PreparedAsset, Projective2, Prop, Rect, ScreenSize, StaggerDirection, StyleValue,
  UiElement, UiElementKind, UiEvent, UiEventBody, UiVisualElementProperties, VariantWhen, Vector,
  ViewportGeometry, ViewportPoint, ViewportRect, WorldBoundsGeometry, WorldPointGeometry,
};
use battlement_fake::{
  assets::FakeAssetCatalog,
  client::{FakeClient, ui::UiClient},
  journal::ExecutedCommand,
};
use battlement_native::{
  ConnectView, Engine, EngineError, EngineResponse, FlatBufferSubmitError, UiEventActionView,
  UiEventResult,
};
use battlement_rules::{
  CONTENT_SCENE, MOTION_AUDIO_CLIP, MOTION_MATERIAL, MOTION_TEXTURE, ROOT_ID, ReactantEngine,
  Screen, create_engine, generated_asset_addresses,
};

const EVENTS_WORD_BUDGET: usize = 20;
const STATE_WORD_BUDGET: usize = 24;
const CONTEXT_WORD_BUDGET: usize = 24;
const EFFECTS_WORD_BUDGET: usize = 22;
const RESOURCES_WORD_BUDGET: usize = 18;
const REFS_WORD_BUDGET: usize = 48;

type Correlations = Rc<RefCell<Vec<(ActionId, Vec<Option<ActionId>>)>>>;

struct CorrelationEngine {
  inner: ReactantEngine,
  correlations: Correlations,
}

impl Engine for CorrelationEngine {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_CONTRACT_DIGEST_C;

  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    self.inner.connect(message)
  }

  fn submit(&mut self, message: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    let action_id = match battlement_flatbuffers::CoreClientMessageView::read(message) {
      Ok(battlement_flatbuffers::CoreClientMessageView::Action(action)) => {
        ActionId::from_bytes(action.action_id()).expect("verified action UUID")
      }
      _ => panic!("the fake UI submits actions"),
    };
    let response = self.inner.submit(message)?;
    record_causes(&self.correlations, action_id, response.as_bytes());
    Ok(response)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    let action_id = ActionId::from_bytes(action.action_id()).expect("verified action UUID");
    let response = self.inner.submit_ui_event(action)?;
    record_causes(&self.correlations, action_id, response.response.as_bytes());
    Ok(response)
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    self.inner.poll()
  }
}

fn record_causes(correlations: &Correlations, action_id: ActionId, bytes: &[u8]) {
  use battlement_flatbuffers::schema_generated::response_generated::battlement::flat_buffers::generated as wire;
  battlement_flatbuffers::ResponseView::read(bytes).expect("engine response is verified");
  let response = wire::size_prefixed_root_as_response(bytes).expect("verified response root");
  let causes = response
    .messages()
    .iter()
    .filter_map(|message| message.message_as_batch())
    .map(|batch| {
      batch.caused_by_action_id().map(|value| {
        let bytes = std::array::from_fn(|index| value.bytes().get(index));
        ActionId::from_bytes(bytes).expect("verified causing action UUID")
      })
    })
    .collect::<Vec<_>>();
  if !causes.is_empty() {
    correlations.borrow_mut().push((action_id, causes));
  }
}

#[path = "composition/layout_motion_flows.rs"]
mod layout_motion_flows;
#[path = "composition/state_effect_resource_flows.rs"]
mod state_effect_resource_flows;

fn navigate_brand(client: &mut FakeClient<ReactantEngine>, navigation_names: &[&str]) {
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

fn find_named<E>(ui: &UiClient<'_, E>, root: ObjectId, expected: &str) -> ObjectId
where
  E: Engine,
{
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
