use crate::{client::FakeClient, transform};
use battlement::{
  CameraProjection, FocusEvent, GameObjectKind, NavigationDirection, NavigationEvent,
  NavigationMoveEvent, ObjectId, PanelPoint, Rect, UiEvent, UiEventBody, UiEventDisposition,
  Vector,
};
use battlement_native::Engine;

#[derive(Default)]
pub(super) struct Navigation {
  active: bool,
  busy: bool,
  focused: Option<ObjectId>,
  invoker: Option<ObjectId>,
  modal: bool,
}

impl<E: Engine> FakeClient<E> {
  /// Observes semantic world/UI focus without advancing time or frames.
  pub fn focused(&self) -> Option<ObjectId> {
    self.navigation.focused.or(self.ui_world.focused())
  }

  /// Moves semantic focus using displayed geometry, without pointer synthesis.
  pub fn navigate(&mut self, direction: NavigationDirection) {
    self.navigation.active = true;
    self.reconcile_navigation();
    if !self.world.input_enabled() {
      return;
    }
    let previous = self.focused();
    if let Some(id) = previous {
      let event = UiEventBody::NavigationMove(NavigationMoveEvent {
        direction,
        move_vector: vector(direction),
      });
      if self.navigation_event(id, event) != UiEventDisposition::Continue {
        return;
      }
    }
    let candidates = self.navigation_targets();
    let next = choose(&candidates, previous, direction);
    self.set_navigation_focus(next);
  }

  /// Activates the currently eligible focus owner through the semantic click route.
  pub fn activate_focused(&mut self) {
    self.reconcile_navigation();
    if !self.world.input_enabled() {
      return;
    }
    if let Some(id) = self.focused() {
      self.navigation_event(
        id,
        UiEventBody::Click(battlement::ClickEvent::NavigationSubmit),
      );
    }
  }

  /// Delivers semantic cancellation without a pointer release or click.
  pub fn cancel_navigation(&mut self) {
    self.navigation.active = true;
    self.reconcile_navigation();
    if !self.world.input_enabled() {
      return;
    }
    if let Some(id) = self.focused() {
      self.navigation_event(id, UiEventBody::NavigationCancel(NavigationEvent {}));
    }
  }

  pub(crate) fn reconcile_navigation(&mut self) {
    if !self.navigation.active || self.navigation.busy {
      return;
    }
    self.navigation.busy = true;
    for _ in 0..16 {
      let before = (self.focused(), self.navigation.modal);
      let modal = self.ui_world.has_modal();
      if modal && !self.navigation.modal {
        self.navigation.invoker = self.navigation.focused;
        self.navigation.modal = true;
        self.set_navigation_focus(None);
      } else if !modal && self.navigation.modal {
        self.navigation.modal = false;
        let return_to = self.navigation.invoker.take();
        let candidates = self.navigation_targets();
        self.set_navigation_focus(
          return_to
            .filter(|id| candidates.iter().any(|v| v.0 == *id))
            .or_else(|| candidates.first().map(|v| v.0)),
        );
      }
      let candidates = self.navigation_targets();
      let current = self.focused();
      let invalid = current.is_some_and(|id| !candidates.iter().any(|v| v.0 == id));
      if invalid || (modal && current.is_none()) {
        self.set_navigation_focus(candidates.first().map(|v| v.0));
      }
      if before == (self.focused(), self.navigation.modal) {
        break;
      }
    }
    self.navigation.busy = false;
  }

  fn navigation_targets(&self) -> Vec<(ObjectId, PanelPoint)> {
    if !self.world.input_enabled() {
      return Vec::new();
    }
    let screen = self.connect.screen;
    let mut ui = Vec::new();
    for object in self.world.objects().filter(|o| o.active_in_hierarchy()) {
      if let GameObjectKind::UiDocument(doc) = object.kind() {
        let viewport = Rect {
          x: 0.0,
          y: 0.0,
          width: f64::from(screen.width),
          height: f64::from(screen.height),
        };
        ui.extend(
          self
            .ui_world
            .focus_targets(doc.root_id(), viewport)
            .into_iter()
            .map(|(id, r)| {
              (
                id,
                PanelPoint {
                  x: r.x + r.width / 2.0,
                  y: r.y + r.height / 2.0,
                },
              )
            }),
        );
      }
    }
    if self.ui_world.has_modal() || self.ui_world.focused().is_some() {
      return ui;
    }
    let mut objects = self
      .world
      .objects()
      .filter(|o| o.active_in_hierarchy())
      .filter(|o| !o.pointer_events().is_empty())
      .filter(|o| o.world_pointer.is_some_and(|s| s.focusable))
      .collect::<Vec<_>>();
    objects.sort_by_key(|o| (o.world_pointer.unwrap().order, o.id()));
    let world = objects
      .into_iter()
      .filter_map(|o| self.focus_position(o.id()).map(|p| (o.id(), p)))
      .collect::<Vec<_>>();
    if world.is_empty() { ui } else { world }
  }

  fn focus_position(&self, id: ObjectId) -> Option<PanelPoint> {
    let camera_id = self.world.input_camera_id()?;
    let camera = self.world.object(camera_id)?.camera()?;
    let pose = self.world.world_transform(camera_id);
    let point = match self.world.object(id)?.kind() {
      GameObjectKind::BoxHitRegion { region } => self.world.world_point(id, region.center),
      _ => self.world.world_transform(id).position,
    };
    let p = transform::rotate(
      transform::inverse(pose.rotation),
      battlement::Vector3::new(
        point.x - pose.position.x,
        point.y - pose.position.y,
        point.z - pose.position.z,
      ),
    );
    if p.z < camera.near || p.z > camera.far {
      return None;
    }
    let extent = match camera.projection {
      CameraProjection::Orthographic => camera.orthographic_size,
      CameraProjection::Perspective => p.z * (camera.field_of_view.to_radians() / 2.0).tan(),
    };
    let height = f64::from(self.connect.screen.height);
    Some(PanelPoint {
      x: f64::from(self.connect.screen.width) / 2.0 + p.x * height / (2.0 * extent),
      y: height / 2.0 - p.y * height / (2.0 * extent),
    })
  }

  fn set_navigation_focus(&mut self, next: Option<ObjectId>) {
    let old = self.focused();
    if old == next {
      return;
    }
    self.navigation.focused = next.filter(|id| self.world.object(*id).is_some());
    self
      .ui_world
      .set_semantic_focus(next.filter(|id| self.ui_world.element(*id).is_some()));
    if let Some(old) = old {
      let event = FocusEvent {
        related_target_id: next,
        ..FocusEvent::default()
      };
      self.navigation_event(old, UiEventBody::FocusOut(event));
      self.navigation_event(old, UiEventBody::Blur(event));
    }
    if let Some(next) = next {
      let event = FocusEvent {
        related_target_id: old,
        ..FocusEvent::default()
      };
      self.navigation_event(next, UiEventBody::FocusIn(event));
      self.navigation_event(next, UiEventBody::Focus(event));
    }
  }

  fn navigation_event(&mut self, id: ObjectId, body: UiEventBody) -> UiEventDisposition {
    self.submit_ui_event(UiEvent {
      target_id: id,
      cancelable: true,
      default_prevented: false,
      body,
    })
  }
}

fn vector(direction: NavigationDirection) -> Vector {
  match direction {
    NavigationDirection::Left => Vector::new(-1.0, 0.0),
    NavigationDirection::Right => Vector::new(1.0, 0.0),
    NavigationDirection::Up => Vector::new(0.0, 1.0),
    NavigationDirection::Down => Vector::new(0.0, -1.0),
    _ => Vector::default(),
  }
}

fn choose(
  candidates: &[(ObjectId, PanelPoint)],
  current: Option<ObjectId>,
  direction: NavigationDirection,
) -> Option<ObjectId> {
  let index = current.and_then(|id| candidates.iter().position(|v| v.0 == id));
  let Some(index) = index else {
    return candidates.first().map(|v| v.0);
  };
  if matches!(
    direction,
    NavigationDirection::Next | NavigationDirection::Previous
  ) {
    let offset = if direction == NavigationDirection::Next {
      1
    } else {
      candidates.len() - 1
    };
    return Some(candidates[(index + offset) % candidates.len()].0);
  }
  let origin = candidates[index].1;
  let direction = vector(direction);
  candidates
    .iter()
    .enumerate()
    .filter_map(|(i, (id, p))| {
      let dx = p.x - origin.x;
      let dy = origin.y - p.y;
      let forward = dx * f64::from(direction.x) + dy * f64::from(direction.y);
      if i == index || forward <= 0.001 {
        return None;
      }
      let side = (dx * f64::from(direction.y) - dy * f64::from(direction.x)).abs();
      Some((*id, forward + side * 2.0, i))
    })
    .min_by(|a, b| a.1.total_cmp(&b.1).then(a.2.cmp(&b.2)))
    .map(|v| v.0)
    .or(current)
}
