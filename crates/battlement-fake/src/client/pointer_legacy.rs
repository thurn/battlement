use crate::{
  client::{FakeClient, PointerInput},
  pointer_geometry::{self, Hit},
};
use battlement::{
  ActionBody, DragMode, DragPayload, ObjectId, PanelPoint, PointerButton, PointerEvent,
  ScreenPosition, Vector3,
};
use battlement_native::Engine;

#[derive(Default)]
pub(super) struct LegacyPointer {
  hovered: Option<Hit>,
  pressed: Option<ObjectId>,
  down: bool,
  drag: Option<Drag>,
}
#[derive(Clone, Copy)]
struct Drag {
  target: ObjectId,
  start: Vector3,
  offset: Vector3,
}

impl<E: Engine> FakeClient<E> {
  pub(super) fn sample_legacy_pointer(
    &mut self,
    id: i32,
    point: PanelPoint,
    down: bool,
    previous_down: bool,
    hit: Option<Hit>,
  ) {
    let mut state = self.pointers.legacy.remove(&id).unwrap_or(LegacyPointer {
      down: previous_down,
      ..LegacyPointer::default()
    });
    if state.hovered.map(|h| h.id) != hit.map(|h| h.id) {
      self.legacy_event(PointerEvent::Exit, state.hovered, id, point);
      self.legacy_event(PointerEvent::Enter, hit, id, point);
      state.hovered = hit;
    }
    if down && !state.down {
      state.pressed = hit.map(|h| h.id);
      self.legacy_event(PointerEvent::Down, hit, id, point);
      if let Some(hit) = hit
        && let Some(mode) = self.world.object(hit.id).and_then(|o| o.drag_mode())
      {
        let start = self.world.world_transform(hit.id).position;
        let pointer = pointer_geometry::plane_point(&self.world, point, self.connect.screen, start);
        let offset = if mode == DragMode::PreserveOffset {
          Vector3::new(
            start.x - pointer.x,
            start.y - pointer.y,
            start.z - pointer.z,
          )
        } else {
          Vector3::ZERO
        };
        self.submit_action(ActionBody::DragStart(DragPayload::new(
          hit.id,
          id,
          self.legacy_screen(point),
          start,
        )));
        state.drag = Some(Drag {
          target: hit.id,
          start,
          offset,
        });
      }
    } else if !down && state.down {
      let dragging = state.drag.take();
      if let Some(drag) = dragging
        && self.legacy_available(drag.target)
      {
        self.submit_action(ActionBody::DragEnd(DragPayload::new(
          drag.target,
          id,
          self.legacy_screen(point),
          self.world.world_transform(drag.target).position,
        )));
      }
      self.legacy_event(PointerEvent::Up, hit, id, point);
      if dragging.is_none() && state.pressed == hit.map(|h| h.id) {
        self.legacy_event(PointerEvent::Click, hit, id, point);
      }
      state.pressed = None;
    }
    state.down = down;
    if id == 0 || down {
      self.pointers.legacy.insert(id, state);
    }
    self.update_legacy_drag(id, point);
  }

  pub(super) fn update_legacy_drag(&mut self, id: i32, point: PanelPoint) {
    let drag = self.pointers.legacy.get(&id).and_then(|s| s.drag);
    if let Some(drag) = drag {
      if !self.legacy_available(drag.target) {
        self.cancel_legacy_pointer(id);
        return;
      }
      let p = pointer_geometry::plane_point(&self.world, point, self.connect.screen, drag.start);
      self.world.set_world_position(
        drag.target,
        Vector3::new(
          p.x + drag.offset.x,
          p.y + drag.offset.y,
          p.z + drag.offset.z,
        ),
      );
    }
  }
  pub(super) fn cancel_legacy_pointer(&mut self, id: i32) {
    if let Some(state) = self.pointers.legacy.remove(&id)
      && let Some(drag) = state.drag
      && self.world.object(drag.target).is_some()
    {
      self.world.set_world_position(drag.target, drag.start);
    }
  }
  fn legacy_event(&mut self, event: PointerEvent, hit: Option<Hit>, id: i32, point: PanelPoint) {
    if let Some(hit) = hit
      && self.legacy_available(hit.id)
    {
      self.send_pointer_event(
        event,
        hit.id,
        PointerInput {
          pointer_id: id,
          screen_position: self.legacy_screen(point),
          world_hit: hit.world.unwrap_or_default(),
          button: PointerButton::Left,
        },
      );
    }
  }
  fn legacy_available(&self, id: ObjectId) -> bool {
    if self.ui_world.has_modal() {
      return false;
    }
    self.world.input_enabled()
      && self
        .world
        .object(id)
        .is_some_and(|o| o.active_in_hierarchy())
  }
  fn legacy_screen(&self, point: PanelPoint) -> ScreenPosition {
    ScreenPosition {
      x: point.x,
      y: f64::from(self.connect.screen.height) - point.y,
    }
  }
}
