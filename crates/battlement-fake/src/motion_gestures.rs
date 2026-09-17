//! Motion layer activation from the host's existing routed pointer and focus events.

use std::collections::{HashMap, HashSet};

use battlement::{MotionLayer, ObjectId, PointerButton, PointerType, UiEvent, UiEventBody};
use battlement_ui_fake::UiWorld;

use crate::world::FakeWorld;

#[derive(Default)]
pub(crate) struct Gestures {
  hovered: HashMap<i32, HashSet<ObjectId>>,
  pressed: HashMap<i32, HashSet<ObjectId>>,
  focused: Option<ObjectId>,
}

impl Gestures {
  pub(crate) fn active(&self, host: ObjectId, layer: MotionLayer) -> bool {
    match layer {
      MotionLayer::Hover => self.hovered.values().any(|route| route.contains(&host)),
      MotionLayer::Tap => self.pressed.values().any(|route| route.contains(&host)),
      MotionLayer::Focus | MotionLayer::FocusVisible => self.focused == Some(host),
      _ => false,
    }
  }

  pub(crate) fn remove(&mut self, host: ObjectId) {
    for pointers in [&mut self.hovered, &mut self.pressed] {
      pointers.retain(|_, route| {
        route.remove(&host);
        !route.is_empty()
      });
    }
    if self.focused == Some(host) {
      self.focused = None;
    }
  }

  pub(crate) fn handle(&mut self, event: &UiEvent, world: &FakeWorld, ui: &UiWorld) {
    match &event.body {
      UiEventBody::PointerOver(value) if value.pointer_type == PointerType::Mouse => {
        update(
          &mut self.hovered,
          value.pointer_id,
          Some(event.target_id),
          world,
          ui,
        );
      }
      UiEventBody::PointerOut(value) if value.pointer_type == PointerType::Mouse => {
        update(
          &mut self.hovered,
          value.pointer_id,
          value.related_target_id,
          world,
          ui,
        );
      }
      UiEventBody::PointerDown(value) if value.button == PointerButton::Left => {
        update(
          &mut self.pressed,
          value.pointer_id,
          Some(event.target_id),
          world,
          ui,
        );
      }
      UiEventBody::PointerUp(value) if value.button == PointerButton::Left => {
        self.pressed.remove(&value.pointer_id);
      }
      UiEventBody::PointerCancel(value) => {
        self.pressed.remove(&value.pointer_id);
        self.hovered.remove(&value.pointer_id);
      }
      UiEventBody::PointerCaptureOut(value) => {
        self.pressed.remove(&value.pointer_id);
      }
      UiEventBody::Focus(_) => self.focused = Some(event.target_id),
      UiEventBody::Blur(_) if self.focused == Some(event.target_id) => self.focused = None,
      _ => {}
    }
  }
}

fn update(
  pointers: &mut HashMap<i32, HashSet<ObjectId>>,
  pointer: i32,
  mut target: Option<ObjectId>,
  world: &FakeWorld,
  ui: &UiWorld,
) {
  let mut route = HashSet::new();
  while let Some(host) = target {
    route.insert(host);
    target = world
      .object(host)
      .and_then(|object| object.parent_id())
      .or_else(|| ui.element(host).and_then(|element| element.parent_id()));
  }
  if route.is_empty() {
    pointers.remove(&pointer);
  } else {
    pointers.insert(pointer, route);
  }
}
