use crate::{transform, world::FakeWorld};
use battlement::{
  CameraProjection, GameObjectKind, ObjectId, PanelPoint, PanelRenderMode, Rect, ScreenSize,
  Vector3,
};
use battlement_ui_fake::UiWorld;

#[derive(Clone, Copy)]
pub(crate) struct Hit {
  pub id: ObjectId,
  pub world: Option<Vector3>,
}

pub(crate) fn pick(
  world: &FakeWorld,
  ui: &UiWorld,
  point: PanelPoint,
  screen: ScreenSize,
) -> Option<Hit> {
  let viewport = Rect {
    x: 0.0,
    y: 0.0,
    width: f64::from(screen.width),
    height: f64::from(screen.height),
  };
  let mut documents = world
    .objects()
    .filter_map(|object| {
      if !object.active_in_hierarchy() {
        return None;
      }
      match object.kind() {
        GameObjectKind::UiDocument(doc)
          if doc.panel_settings.render_mode == PanelRenderMode::ScreenSpaceOverlay =>
        {
          Some(doc)
        }
        _ => None,
      }
    })
    .collect::<Vec<_>>();
  documents.sort_by_key(|doc| doc.sorting_order);
  for doc in documents.into_iter().rev() {
    if let Some(target) = ui.pick(doc.root_id(), point, viewport) {
      return Some(Hit {
        id: target,
        world: None,
      });
    }
  }
  if ui.has_modal() {
    return None;
  }
  let (origin, direction, far_clip) = ray(world, point, screen)?;
  world
    .objects()
    .filter_map(|object| {
      if !object.active_in_hierarchy()
        || (object.pointer_events().is_empty() && object.drag_mode().is_none())
      {
        return None;
      }
      let GameObjectKind::BoxHitRegion { region } = object.kind() else {
        return None;
      };
      let local_origin = inverse_point(world, object.id(), origin)?;
      let next = inverse_point(
        world,
        object.id(),
        Vector3::new(
          origin.x + direction.x,
          origin.y + direction.y,
          origin.z + direction.z,
        ),
      )?;
      let dir = [
        next.x - local_origin.x,
        next.y - local_origin.y,
        next.z - local_origin.z,
      ];
      let start = [
        local_origin.x - region.center.x,
        local_origin.y - region.center.y,
        local_origin.z - region.center.z,
      ];
      let half = [
        region.size.x / 2.0,
        region.size.y / 2.0,
        region.size.z / 2.0,
      ];
      let mut near = 0.0_f64;
      let mut far = far_clip;
      for axis in 0..3 {
        if dir[axis].abs() < 1e-12 {
          if start[axis].abs() > half[axis] {
            return None;
          }
        } else {
          let a = (-half[axis] - start[axis]) / dir[axis];
          let b = (half[axis] - start[axis]) / dir[axis];
          near = near.max(a.min(b));
          far = far.min(a.max(b));
        }
      }
      if near > far {
        return None;
      }
      let settings = object.world_pointer.unwrap_or_default();
      Some((
        object.id(),
        settings.interaction_layer,
        near,
        settings.order,
      ))
    })
    .min_by(|a, b| b.1.cmp(&a.1).then(a.2.total_cmp(&b.2)).then(b.3.cmp(&a.3)))
    .map(|v| Hit {
      id: v.0,
      world: Some(Vector3::new(
        origin.x + direction.x * v.2,
        origin.y + direction.y * v.2,
        origin.z + direction.z * v.2,
      )),
    })
}

fn inverse_point(world: &FakeWorld, id: ObjectId, point: Vector3) -> Option<Vector3> {
  let object = world.object(id)?;
  let point = if let Some(parent) = object.parent_id() {
    inverse_point(world, parent, point)?
  } else {
    point
  };
  let local = object.local_transform();
  if local.scale.x == 0.0 || local.scale.y == 0.0 || local.scale.z == 0.0 {
    return None;
  }
  let point = transform::rotate(
    transform::inverse(local.rotation),
    Vector3::new(
      point.x - local.position.x,
      point.y - local.position.y,
      point.z - local.position.z,
    ),
  );
  Some(Vector3::new(
    point.x / local.scale.x,
    point.y / local.scale.y,
    point.z / local.scale.z,
  ))
}

fn ray(
  world: &FakeWorld,
  point: PanelPoint,
  screen: ScreenSize,
) -> Option<(Vector3, Vector3, f64)> {
  let camera_id = world.input_camera_id()?;
  let camera = world.object(camera_id)?.camera()?;
  let pose = world.world_transform(camera_id);
  let x = 2.0 * point.x / f64::from(screen.width) - 1.0;
  let y = 1.0 - 2.0 * point.y / f64::from(screen.height);
  let aspect = f64::from(screen.width) / f64::from(screen.height);
  let (origin, direction) = match camera.projection {
    CameraProjection::Orthographic => (
      world.world_point(
        camera_id,
        Vector3::new(
          x * camera.orthographic_size * aspect,
          y * camera.orthographic_size,
          camera.near,
        ),
      ),
      transform::rotate(pose.rotation, Vector3::new(0.0, 0.0, 1.0)),
    ),
    CameraProjection::Perspective => (
      world.world_point(
        camera_id,
        Vector3::new(
          x * aspect * (camera.field_of_view.to_radians() / 2.0).tan() * camera.near,
          y * (camera.field_of_view.to_radians() / 2.0).tan() * camera.near,
          camera.near,
        ),
      ),
      transform::rotate(
        pose.rotation,
        Vector3::new(
          x * aspect * (camera.field_of_view.to_radians() / 2.0).tan(),
          y * (camera.field_of_view.to_radians() / 2.0).tan(),
          1.0,
        ),
      ),
    ),
  };
  Some((origin, direction, camera.far - camera.near))
}

pub(crate) fn plane_point(
  world: &FakeWorld,
  point: PanelPoint,
  screen: ScreenSize,
  start: Vector3,
) -> Vector3 {
  let (origin, direction, _) = ray(world, point, screen).expect("drag requires an input camera");
  let camera = world.world_transform(world.input_camera_id().unwrap());
  let facing = transform::rotate(camera.rotation, Vector3::new(0.0, 0.0, 1.0));
  let axis = if facing.x.abs() >= facing.y.abs() && facing.x.abs() >= facing.z.abs() {
    0
  } else if facing.y.abs() >= facing.z.abs() {
    1
  } else {
    2
  };
  let distance = ([start.x - origin.x, start.y - origin.y, start.z - origin.z][axis])
    / [direction.x, direction.y, direction.z][axis];
  assert!(
    distance.is_finite() && distance >= 0.0,
    "pointer ray must intersect drag plane"
  );
  Vector3::new(
    origin.x + direction.x * distance,
    origin.y + direction.y * distance,
    origin.z + direction.z * distance,
  )
}
