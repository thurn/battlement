use std::{cell::RefCell, collections::BTreeSet, num::NonZeroU64, rc::Rc, sync::Arc};

use battlement::{
  ActionId, ClientMessage, Color, Command, Connect, CoreErrorCode, Display, DisplayId,
  DisplayOrientation, ElementGeometry, FlexDirection, FocusEvent, GeometryGeneration,
  GeometryObservation, GeometryObservationBatch, GeometryObservationResult,
  GeometryObservationTarget, GeometryObservationValue, GeometryUnavailable, GeometryValue,
  KeyModifiers, Length, LengthOrAuto, ObjectId, OverlayPlacement, PanelPoint, PointerBoundaryEvent,
  PointerButton, PointerButtonEvent, PointerType, PreparedAsset, Projective2, Prop, Rect, Response,
  ResponseMessage, ScreenSize, StaggerDirection, StyleValue, UiElement, UiElementKind, UiEvent,
  UiEventAction, UiEventBody, UiEventResponse, UiVisualElementProperties, VariantWhen, Vector,
  ViewportGeometry, ViewportPoint, ViewportRect, WorldBoundsGeometry, WorldPointGeometry,
};
use battlement_fake::{
  assets::FakeAssetCatalog,
  client::{FakeClient, ui::UiClient},
  journal::ExecutedCommand,
};
use battlement_native::{Engine, EngineError};
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
  type ActionPayload = ();
  type ErrorCode = CoreErrorCode;
  type Command = Command;

  fn connect(&mut self, message: Connect) -> Result<Response, EngineError> {
    self.inner.connect(message)
  }

  fn submit(
    &mut self,
    message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
  ) -> Result<Response, EngineError> {
    let ClientMessage::Action(action) = &message else {
      panic!("the fake UI submits actions");
    };
    let action_id = action.action_id;
    let response = self.inner.submit(message)?;
    let causes = response
      .messages
      .iter()
      .filter_map(|message| match message {
        ResponseMessage::Batch(batch) => Some(batch.caused_by_action_id),
        _ => None,
      })
      .collect::<Vec<_>>();
    if !causes.is_empty() {
      self.correlations.borrow_mut().push((action_id, causes));
    }
    Ok(response)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventAction,
  ) -> Result<UiEventResponse<Self::Command>, EngineError> {
    let action_id = action.action_id;
    let response = self.inner.submit_ui_event(action)?;
    let causes = response
      .response
      .messages
      .iter()
      .filter_map(|message| match message {
        ResponseMessage::Batch(batch) => Some(batch.caused_by_action_id),
        _ => None,
      })
      .collect::<Vec<_>>();
    if !causes.is_empty() {
      self.correlations.borrow_mut().push((action_id, causes));
    }
    Ok(response)
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    self.inner.poll()
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
  E: Engine<Command = Command>,
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
