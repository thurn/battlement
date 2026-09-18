use std::{sync::Arc, time::Duration};

use battlement::{CommandId, Connect, ObjectId, Vector3};
use battlement_fake::{
  assets::FakeAssetCatalog,
  client::FakeClient,
  effects::{AudioOccurrence, ParticleOccurrence},
  world::{FakeAudio, FakeObject},
};
use battlement_native::Engine;

/// A real engine connected to the deterministic in-memory display host.
///
/// Inputs travel through the engine's public submit APIs. Observations come
/// from the fake host after it applies verified engine responses.
pub struct Display<E>
where
  E: Engine,
{
  client: FakeClient<E>,
}

impl<E> Display<E>
where
  E: Engine,
{
  /// Connects an engine with deterministic fake platform metadata.
  #[must_use]
  pub fn connect(engine: E, assets: impl Into<Arc<FakeAssetCatalog>>) -> Self {
    Self {
      client: FakeClient::connect(engine, assets),
    }
  }

  /// Connects an engine with explicit deterministic platform metadata.
  #[must_use]
  pub fn connect_with(
    engine: E,
    assets: impl Into<Arc<FakeAssetCatalog>>,
    connect: Connect,
  ) -> Self {
    Self {
      client: FakeClient::connect_with(engine, assets, connect),
    }
  }

  /// Runs a public app operation without polling, advancing time, or a frame.
  pub fn with_engine<R>(&mut self, operation: impl FnOnce(&mut E) -> R) -> R {
    operation(self.client.engine_mut())
  }

  /// Replaces the engine session without advancing time or rendering a frame.
  pub fn reconnect(&mut self) {
    self.client.reconnect();
  }

  /// Applies at most one response made available by the engine.
  ///
  /// Polling may run engine-owned main-thread work, but it does not advance
  /// presentation time or record a rendered frame.
  pub fn poll(&mut self) {
    self.client.poll();
  }

  /// Presses a geometric primary pointer in upper-left screen pixels; advances no clock or frame.
  pub fn pointer_down(
    &mut self,
    pointer_id: i32,
    position: battlement::PanelPoint,
  ) -> battlement::UiEventDisposition {
    self.client.sample_pointer(pointer_id, position, true)
  }
  /// Moves a geometric pointer with its primary button state; advances no clock or frame.
  pub fn pointer_move(
    &mut self,
    pointer_id: i32,
    position: battlement::PanelPoint,
    pressed: bool,
  ) -> battlement::UiEventDisposition {
    self.client.sample_pointer(pointer_id, position, pressed)
  }
  /// Releases a geometric primary pointer; advances no clock or frame.
  pub fn pointer_up(
    &mut self,
    pointer_id: i32,
    position: battlement::PanelPoint,
  ) -> battlement::UiEventDisposition {
    self.client.sample_pointer(pointer_id, position, false)
  }
  /// Clicks the eligible target after UI/modal/world geometric arbitration.
  pub fn click_at(&mut self, position: battlement::PanelPoint) {
    self.pointer_down(0, position);
    self.pointer_up(0, position);
  }
  /// Observes the current pointer capture owner.
  pub fn pointer_capture(&self, pointer_id: i32) -> Option<ObjectId> {
    self.client.geometric_capture(pointer_id)
  }
  /// Observes host capture loss, including loss after logical destruction.
  pub fn capture_losses(&self) -> &[(i32, ObjectId)] {
    self.client.capture_losses()
  }

  /// Observes current semantic focus without advancing time or frames.
  pub fn focused(&self) -> Option<ObjectId> {
    self.client.focused()
  }
  /// Moves focus using current displayed geometry, without synthesizing a pointer.
  pub fn navigate(&mut self, direction: battlement::NavigationDirection) {
    self.client.navigate(direction);
  }
  /// Activates the current eligible focus owner; advances no time or frame.
  pub fn activate_focused(&mut self) {
    self.client.activate_focused();
  }
  /// Cancels through the focused logical route, including a modal's dismiss behavior.
  pub fn cancel_navigation(&mut self) {
    self.client.cancel_navigation();
  }

  /// Activates a world object through the same coordinate-free route as native Ditto.
  pub fn activate(&mut self, object_id: ObjectId) {
    self.client.activate(object_id);
  }

  /// Performs a semantic primary-pointer click on a world object.
  pub fn click(&mut self, object_id: ObjectId) {
    self.client.click(object_id);
  }

  /// Performs a native-style click on a UI button.
  pub fn click_ui(&mut self, object_id: ObjectId) {
    self.client.ui().click(object_id);
  }

  /// Activates a UI button through the shared keyboard/controller submit route.
  pub fn navigation_submit_ui(&mut self, object_id: ObjectId) {
    self.client.ui().navigation_submit(object_id);
  }

  /// Delivers a previously captured native input without re-resolving its target.
  /// Removed targets are ignored by the engine's current event routing.
  pub fn deliver_ui_event(&mut self, event: battlement::UiEvent) {
    self.client.ui().deliver_event(event);
  }

  /// Delivers native Motion lifecycle observations without advancing host time.
  pub fn deliver_motion_events(&mut self, events: battlement::MotionEventBatch) {
    self.client.submit_motion(events);
  }

  /// Finds a live UI descendant by its authored name.
  #[must_use]
  pub fn find_ui(&self, root: ObjectId, name: &str) -> ObjectId {
    let ui = self.client.ui_world();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
      let element = ui
        .element(id)
        .unwrap_or_else(|| panic!("UI element does not exist: {id}"));
      if element.name() == Some(name) {
        return id;
      }
      pending.extend(element.children());
    }
    panic!("missing UI element named {name}");
  }

  /// Reports whether a native UI element remains presented, including retained exits.
  #[must_use]
  pub fn contains_ui(&self, object_id: ObjectId) -> bool {
    self.client.ui_world().element(object_id).is_some()
  }

  /// Cancels a geometric gesture without clicking or advancing time.
  pub fn pointer_cancel(&mut self, pointer_id: i32) {
    self.client.cancel_pointer(pointer_id);
  }

  /// Returns one live UI element for visible-state inspection.
  #[must_use]
  pub fn ui_element(
    &self,
    object_id: ObjectId,
  ) -> &battlement_fake::battlement_ui_fake::UiElementState {
    self
      .client
      .ui_world()
      .element(object_id)
      .unwrap_or_else(|| panic!("UI element does not exist: {object_id}"))
  }

  /// Returns the currently presented accessibility tree.
  #[must_use]
  pub fn accessibility(&self) -> &battlement::AccessibilitySnapshot {
    self.client.accessibility()
  }

  /// Returns one world object when it is currently presented.
  #[must_use]
  pub fn object(&self, object_id: ObjectId) -> Option<&FakeObject> {
    self.client.world().object(object_id)
  }

  /// Returns one live audio playback when it is currently presented.
  #[must_use]
  pub fn audio(&self, command_id: CommandId) -> Option<&FakeAudio> {
    self.client.world().audio(command_id)
  }

  /// Observes a presented local point without advancing time, work, or frames.
  #[must_use]
  pub fn world_point(&self, object_id: ObjectId, offset: Vector3) -> Vector3 {
    self.client.world().world_point(object_id, offset)
  }

  /// Advances virtual rules and presentation time without rendering a frame.
  pub fn advance_time(&mut self, duration: Duration) {
    self.client.advance_time(duration);
  }

  /// Records one rendered-frame boundary without advancing virtual time.
  pub fn advance_frame(&mut self) {
    self.client.advance_frame();
  }

  /// Advances all finite presentation work and leaves cosmetic loops running.
  pub fn settle(&mut self) {
    self.client.settle();
  }

  /// Returns elapsed virtual presentation time.
  #[must_use]
  pub fn presentation_time(&self) -> Duration {
    self.client.presentation_time()
  }

  /// Returns the number of explicitly recorded rendered frames.
  #[must_use]
  pub fn frame(&self) -> u64 {
    self.client.frame()
  }

  /// Returns audio play occurrences in execution order.
  #[must_use]
  pub fn audio_occurrences(&self) -> &[AudioOccurrence] {
    self.client.audio_occurrences()
  }

  /// Returns temporary particle occurrences in execution order.
  #[must_use]
  pub fn particle_occurrences(&self) -> &[ParticleOccurrence] {
    self.client.particle_occurrences()
  }
}
