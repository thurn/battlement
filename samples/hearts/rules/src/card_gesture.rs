use battlement::{ClickEvent, PanelPoint, PointerButton, PointerType, Quaternion, Vector3};
use reactant::{GameVersion, app_context, event::ReactantEvent, hooks, world};

use crate::{card_input::CardInput, projection::CardToken, scene};

#[derive(Clone)]
struct Gesture {
  pointer: i32,
  origin: PanelPoint,
  slop: f64,
  moved: bool,
  version: GameVersion,
}

pub(crate) fn use_gesture(
  token: CardToken,
  destination: Option<world::LayoutDestination>,
  input: Option<CardInput>,
) -> (Vector3, world::PointerHandlers) {
  let viewport = app_context::use_viewport_size();
  let gesture = hooks::use_ref(None::<Gesture>);
  let suppress_click = hooks::use_ref(false);
  let (offset, set_offset) = hooks::use_state(Vector3::ZERO);
  let enabled = input
    .as_ref()
    .is_some_and(|input| input.inspection().is_none() && input.owns(token));
  let reset = gesture.clone();
  let reset_offset = set_offset.clone();
  hooks::use_effect(
    move || {
      reset.replace(None);
      reset_offset.set(Vector3::ZERO);
    },
    (enabled, viewport),
  );
  let down = gesture.clone();
  let down_input = input.clone();
  let down_suppression = suppress_click.clone();
  let moved = gesture.clone();
  let move_offset = set_offset.clone();
  let move_suppression = suppress_click.clone();
  let up = gesture.clone();
  let up_offset = set_offset.clone();
  let lost = gesture.clone();
  let lost_offset = set_offset.clone();
  let handlers = world::PointerHandlers::new()
    .on_pointer_down(
      move |event: ReactantEvent<battlement::PointerButtonEvent>| {
        if event.payload().button != PointerButton::Left || !enabled {
          return;
        }
        if down.get().is_some() {
          return;
        }
        let input = down_input.as_ref().expect("enabled card input");
        if !input.table_enabled() {
          return;
        }
        down_suppression.replace(false);
        down.replace(Some(Gesture {
          pointer: event.payload().pointer_id,
          origin: event.payload().position,
          slop: if event.payload().pointer_type == PointerType::Touch {
            8.0
          } else {
            3.0
          },
          moved: false,
          version: input.version(),
        }));
      },
    )
    .on_pointer_move(move |event: ReactantEvent<battlement::PointerMoveEvent>| {
      let Some(mut drag) = moved.get() else {
        return;
      };
      if drag.pointer != event.payload().pointer_id {
        return;
      }
      let dx = event.payload().position.x - drag.origin.x;
      let dy = event.payload().position.y - drag.origin.y;
      drag.moved |= dx.hypot(dy) > drag.slop;
      if drag.moved {
        move_suppression.replace(true);
        if let Some(target) = destination
          .as_ref()
          .and_then(world::LayoutDestination::latest)
        {
          let world = scene::table_drag_delta(dx, dy, viewport.height);
          let q = target.transform.rotation;
          let local = self::rotate(Quaternion::new(-q.x, -q.y, -q.z, q.w), world);
          let scale = target.transform.scale;
          move_offset.set(Vector3::new(
            local.x / scale.x,
            local.y / scale.y,
            local.z / scale.z,
          ));
        }
      }
      moved.replace(Some(drag));
    })
    .on_pointer_up(
      move |event: ReactantEvent<battlement::PointerButtonEvent>| {
        if event.payload().button != PointerButton::Left {
          return;
        }
        let Some(drag) = up.get() else {
          return;
        };
        if drag.pointer != event.payload().pointer_id {
          return;
        }
        up.replace(None);
        up_offset.set(Vector3::ZERO);
        if let Some(input) = &input
          && drag.moved
          && drag.version == input.version()
        {
          input.drop_card(token, event.payload().position);
        }
      },
    )
    .on_pointer_capture_out(
      move |event: ReactantEvent<battlement::PointerCaptureEvent>| {
        if lost
          .get()
          .is_some_and(|drag| drag.pointer == event.payload().pointer_id)
        {
          lost.replace(None);
          lost_offset.set(Vector3::ZERO);
        }
      },
    )
    .on_pointer_cancel(
      move |event: ReactantEvent<battlement::PointerCancelEvent>| {
        if gesture
          .get()
          .is_some_and(|drag| drag.pointer == event.payload().pointer_id)
        {
          gesture.replace(None);
          set_offset.set(Vector3::ZERO);
        }
      },
    )
    .on_click_capture(move |event: ReactantEvent<ClickEvent>| {
      if !matches!(event.payload(), ClickEvent::NavigationSubmit) && suppress_click.get() {
        event.prevent_default();
      }
    });
  (if enabled { offset } else { Vector3::ZERO }, handlers)
}

fn rotate(q: Quaternion, v: Vector3) -> Vector3 {
  let t = Vector3::new(
    2.0 * (q.y * v.z - q.z * v.y),
    2.0 * (q.z * v.x - q.x * v.z),
    2.0 * (q.x * v.y - q.y * v.x),
  );
  Vector3::new(
    v.x + q.w * t.x + q.y * t.z - q.z * t.y,
    v.y + q.w * t.y + q.z * t.x - q.x * t.z,
    v.z + q.w * t.z + q.x * t.y - q.y * t.x,
  )
}
