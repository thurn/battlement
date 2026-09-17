//! Geometric pointer driving through verified synchronous input.
use crate::{client::FakeClient, pointer_geometry};
use battlement::{
  ClickEvent, KeyModifiers, ObjectId, PanelPoint, PointerBoundaryEvent, PointerButton,
  PointerButtonEvent, PointerCancelEvent, PointerCaptureEvent, PointerCrossingEvent,
  PointerMoveEvent, PointerType, UiEvent, UiEventBody, UiEventDisposition, Vector,
};
use battlement_native::Engine;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Default)]
pub(crate) struct PointerState {
  position: PanelPoint,
  down: bool,
  pressed: Option<ObjectId>,
  hovered: Option<ObjectId>,
  captured: Option<ObjectId>,
  moved: bool,
}

#[derive(Default)]
pub(crate) struct Pointers {
  states: BTreeMap<i32, PointerState>,
  ui: BTreeMap<i32, ObjectId>,
  active: bool,
  losses: Vec<(i32, ObjectId)>,
  pub(super) legacy: BTreeMap<i32, crate::client::pointer_legacy::LegacyPointer>,
}

impl<E: Engine> FakeClient<E> {
  /// Samples a geometric primary pointer without advancing time or frames.
  pub fn sample_pointer(
    &mut self,
    id: i32,
    position: PanelPoint,
    down: bool,
  ) -> UiEventDisposition {
    assert!(
      position.x.is_finite() && position.y.is_finite(),
      "pointer position must be finite"
    );
    self.pointers.active = true;
    self.reconcile_geometric_pointers();
    let initial_losses = self.pointers.losses.len();
    let mut state = self.pointers.states.remove(&id).unwrap_or(PointerState {
      position,
      ..PointerState::default()
    });
    self.update_legacy_drag(id, position);
    let hit = pointer_geometry::pick(&self.world, &self.ui_world, position, self.connect.screen);
    let legacy = hit.is_some_and(|hit| {
      self
        .world
        .object(hit.id)
        .is_some_and(|o| o.world_pointer.is_none())
    });
    let uncaptured = state.captured.is_none() && self.ui_world.pointer_capture(id).is_none();
    if uncaptured && (legacy || (hit.is_none() && self.pointers.legacy.contains_key(&id))) {
      if let Some(old) = state.hovered.take() {
        self.geometric_event(
          old,
          UiEventBody::PointerOut(PointerCrossingEvent {
            related_target_id: hit.map(|h| h.id),
            pointer_id: id,
            position,
            pointer_type: if id == 0 {
              PointerType::Mouse
            } else {
              PointerType::Touch
            },
          }),
          false,
        );
        self.geometric_event(
          old,
          UiEventBody::PointerLeave(PointerBoundaryEvent {
            pointer_id: id,
            position,
            pointer_type: if id == 0 {
              PointerType::Mouse
            } else {
              PointerType::Touch
            },
          }),
          false,
        );
      }
      self.sample_legacy_pointer(id, position, down, state.down, hit);
      state.down = down;
      state.position = position;
      state.pressed = None;
      if id == 0 || down {
        self.pointers.states.insert(id, state);
      }
      return UiEventDisposition::Continue;
    }
    self.cancel_legacy_pointer(id);
    let target = state
      .captured
      .or(self.ui_world.pointer_capture(id))
      .or(hit.map(|h| h.id));
    let delta = Vector::new(
      (position.x - state.position.x) as f32,
      (position.y - state.position.y) as f32,
    );
    let pointer_type = if id == 0 {
      PointerType::Mouse
    } else {
      PointerType::Touch
    };
    state.position = position;
    if state.hovered != target {
      if let Some(old) = state.hovered {
        self.geometric_event(
          old,
          UiEventBody::PointerOut(PointerCrossingEvent {
            related_target_id: target,
            pointer_id: id,
            position,
            pointer_type,
          }),
          false,
        );
        self.geometric_event(
          old,
          UiEventBody::PointerLeave(PointerBoundaryEvent {
            pointer_id: id,
            position,
            pointer_type,
          }),
          false,
        );
      }
      if let Some(target) = target {
        self.geometric_event(
          target,
          UiEventBody::PointerOver(PointerCrossingEvent {
            related_target_id: state.hovered,
            pointer_id: id,
            position,
            pointer_type,
          }),
          false,
        );
        self.geometric_event(
          target,
          UiEventBody::PointerEnter(PointerBoundaryEvent {
            pointer_id: id,
            position,
            pointer_type,
          }),
          false,
        );
      }
    }
    state.hovered = target;
    let mut disposition = UiEventDisposition::Continue;
    if delta != Vector::default() {
      state.moved |= state.captured.is_some() || self.ui_world.pointer_capture(id).is_some();
      if let Some(target) = target {
        disposition = self.geometric_event(
          target,
          UiEventBody::PointerMove(PointerMoveEvent {
            pointer_id: id,
            position,
            delta,
            changed_button: None,
            buttons: u32::from(down),
            pressure: 0.0,
            click_count: 0,
            modifiers: KeyModifiers::default(),
            pointer_type,
          }),
          true,
        );
      }
    }
    let button = PointerButtonEvent {
      pointer_id: id,
      position,
      delta,
      button: PointerButton::Left,
      buttons: u32::from(down),
      pressure: 0.0,
      click_count: 1,
      modifiers: KeyModifiers::default(),
      pointer_type,
    };
    if down && !state.down {
      state.pressed = target;
      state.moved = false;
      if let Some(target) = target {
        disposition = self.geometric_event(target, UiEventBody::PointerDown(button.clone()), true);
        let capture = self
          .world
          .object(target)
          .and_then(|o| o.world_pointer)
          .is_some_and(|p| p.capture_on_press);
        if capture && disposition == UiEventDisposition::Continue && self.pointer_eligible(target) {
          state.captured = Some(target);
          self.geometric_event(
            target,
            UiEventBody::PointerCapture(PointerCaptureEvent { pointer_id: id }),
            false,
          );
        }
      }
    } else if !down && state.down {
      if let Some(target) = target {
        disposition = self.geometric_event(target, UiEventBody::PointerUp(button), true);
        let matched = state.pressed == Some(target) && !state.moved;
        let lost = self.pointers.losses[initial_losses..]
          .iter()
          .any(|(pointer, _)| *pointer == id);
        if matched && !lost && disposition == UiEventDisposition::Continue {
          disposition = self.geometric_event(
            target,
            UiEventBody::Click(ClickEvent::Pointer {
              pointer_id: id,
              position,
              button: PointerButton::Left,
              click_count: 1,
              modifiers: KeyModifiers::default(),
            }),
            true,
          );
        }
      }
      self.lose_capture(id, &mut state);
      state.pressed = None;
    }
    state.down = down;
    if state
      .captured
      .is_some_and(|target| !self.pointer_eligible(target))
    {
      self.lose_capture(id, &mut state);
    }
    if id == 0 || down {
      self.pointers.states.insert(id, state);
    }
    disposition
  }

  /// Cancels one gesture without generating a click or advancing time.
  pub fn cancel_pointer(&mut self, id: i32) {
    self.cancel_legacy_pointer(id);
    if let Some(mut state) = self.pointers.states.remove(&id) {
      if let Some(target) = state
        .captured
        .or(self.ui_world.pointer_capture(id))
        .or(state.pressed)
      {
        self.geometric_event(
          target,
          UiEventBody::PointerCancel(PointerCancelEvent {
            pointer_id: id,
            position: state.position,
            delta: Vector::default(),
            buttons: 0,
            pressure: 0.0,
            modifiers: KeyModifiers::default(),
            pointer_type: if id == 0 {
              PointerType::Mouse
            } else {
              PointerType::Touch
            },
          }),
          false,
        );
      }
      self.lose_capture(id, &mut state);
    }
    self.ui_world.cancel_pointer_capture(id);
    self.reconcile_ui_captures();
  }

  /// The target currently capturing this geometric pointer.
  pub fn geometric_capture(&self, id: i32) -> Option<ObjectId> {
    self
      .pointers
      .states
      .get(&id)
      .and_then(|p| p.captured)
      .or(self.ui_world.pointer_capture(id))
  }
  /// Native-style capture-loss observations, including removed logical targets.
  pub fn capture_losses(&self) -> &[(i32, ObjectId)] {
    &self.pointers.losses
  }

  pub(crate) fn reconcile_geometric_pointers(&mut self) {
    self.reconcile_ui_captures();
    let losses = self
      .pointers
      .states
      .iter()
      .filter_map(|(id, s)| {
        s.captured
          .filter(|target| !self.pointer_eligible(*target))
          .map(|target| (*id, target))
      })
      .collect::<Vec<_>>();
    for (id, _) in losses {
      let mut state = self.pointers.states.remove(&id).expect("captured pointer");
      self.lose_capture(id, &mut state);
      self.pointers.states.insert(id, state);
    }
  }
  fn reconcile_ui_captures(&mut self) {
    if !self.pointers.active {
      return;
    }
    self.ui_world.reconcile_pointer_captures();
    let current = self.ui_world.pointer_captures().collect::<BTreeMap<_, _>>();
    let previous = std::mem::replace(&mut self.pointers.ui, current.clone());
    for (pointer, target) in &previous {
      if current.get(pointer) != Some(target) {
        if let Some(state) = self.pointers.states.get_mut(pointer) {
          state.pressed = None;
          state.moved = true;
        }
        self.pointers.losses.push((*pointer, *target));
        self.submit_ui_event(UiEvent::new(
          *target,
          false,
          false,
          UiEventBody::PointerCaptureOut(PointerCaptureEvent {
            pointer_id: *pointer,
          }),
        ));
      }
    }
    for (pointer, target) in current {
      if previous.get(&pointer) != Some(&target)
        && self.ui_world.pointer_capture(pointer) == Some(target)
      {
        self.geometric_event(
          target,
          UiEventBody::PointerCapture(PointerCaptureEvent {
            pointer_id: pointer,
          }),
          false,
        );
      }
    }
  }

  fn lose_capture(&mut self, id: i32, state: &mut PointerState) {
    if let Some(target) = state.captured.take() {
      state.pressed = None;
      state.moved = true;
      self.pointers.losses.push((id, target));
      self.submit_ui_event(UiEvent::new(
        target,
        false,
        false,
        UiEventBody::PointerCaptureOut(PointerCaptureEvent { pointer_id: id }),
      ));
    }
  }
  fn pointer_eligible(&self, target: ObjectId) -> bool {
    if !self.world.input_enabled() {
      return false;
    }
    if let Some(object) = self.world.object(target) {
      !self.ui_world.has_modal()
        && object.active_in_hierarchy()
        && !object.pointer_events().is_empty()
    } else {
      self.ui_world.input_eligible(target)
    }
  }
  fn geometric_event(
    &mut self,
    target: ObjectId,
    body: UiEventBody,
    cancelable: bool,
  ) -> UiEventDisposition {
    if !self.pointer_eligible(target) {
      return UiEventDisposition::Continue;
    }
    self.submit_ui_event(UiEvent::new(target, cancelable, false, body))
  }
}
