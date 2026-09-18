//! World attachments and typed visuals owned by ordinary Reactant components.

use battlement::{
  GameObject, GameObjectKind, LocalTransform, ObjectId, ParentScene, PrefabAddress, Quaternion,
  RenderOrder, Vector3,
};
use reactant_core::{
  callback::Callback,
  component::Component,
  context::ContextProvider,
  hooks,
  motion::MotionProps,
  native_host::{NativeHost, ObjectRef},
  prelude::MotionComponent,
  render::{Node, Render},
};

use uuid::Uuid;

use crate::world_adapter::{WorldAdapter, WorldDescription};

pub use crate::world_hit_region::BoxHitRegion;
pub use crate::world_layout::{
  Arc, ArcLayout, Fan, FanLayout, Flex, FlexDirection, FlexLayout, Grid, GridLayout,
  LayoutAlgorithm, LayoutAlignment, LayoutBox, LayoutChild, LayoutDestination, LayoutExtent,
  LayoutItem, LayoutOrientation, LayoutPlacement, LayoutPlane, LayoutScaling, LayoutTarget, Pile,
  PileLayout, WorldLayout,
};
pub use crate::world_object::WorldObject;
pub use crate::world_text::Text;
pub use crate::world_view::{Camera, Light};
pub use crate::world_visuals::{Mesh, Sprite};
pub use reactant_core::local_point::{
  LocalPoint, LocalPointTarget, PointTracking, ResolvedLocalPoint,
};
pub use reactant_core::navigation_handlers::NavigationHandlers;
pub use reactant_core::pointer_handlers::PointerHandlers;

#[derive(Clone, PartialEq)]
struct SceneAttachment(ParentScene);

#[derive(Clone, Default, PartialEq)]
struct MotionPointerInput(bool);

#[derive(Clone, Default, PartialEq)]
struct RustPointerInput(bool);

/// An explicit attachment to a loaded scene or the persistent scene container.
pub struct SceneRoot {
  scene: ParentScene,
  children: Vec<Node>,
}

/// An empty transform group with logical children.
#[derive(Clone)]
pub struct Group {
  pub(crate) kind: GameObjectKind,
  transform: LocalTransform,
  active: bool,
  pub(crate) render_order: Option<RenderOrder>,
  pub(crate) material_instances: Vec<battlement::MaterialInstance>,
  children: Vec<Node>,
  reference: Option<ObjectRef>,
  click: Option<Callback<()>>,
  events: PointerHandlers,
  navigation: NavigationHandlers,
  pointer: battlement::WorldPointerSettings,
  id: Option<Uuid>,
  motion: MotionProps,
}

/// An opaque prepared prefab beneath a Reactant-owned transform.
pub struct Prefab;

impl SceneRoot {
  /// Attaches children to this scene without changing logical ancestry.
  pub fn new(scene: ParentScene) -> Self {
    Self {
      scene,
      children: Vec::new(),
    }
  }

  /// Appends a logical world contribution.
  pub fn child(mut self, child: impl Render) -> Self {
    self.children.push(Node::new(child));
    self
  }
}

impl Component for SceneRoot {
  fn render(&self) -> impl Render {
    ContextProvider::new()
      .context(SceneAttachment(self.scene))
      .child(
        ContextProvider::new()
          .context(MotionPointerInput(false))
          .child(
            ContextProvider::new()
              .context(RustPointerInput(false))
              .child(
                NativeHost::<WorldAdapter>::new(WorldDescription {
                  kind: GameObjectKind::Empty,
                  scene: self.scene,
                  root: true,
                  transform: LocalTransform::default(),
                  active: true,
                  clickable: false,
                  world_pointer: None,
                  render_order: None,
                  material_instances: Vec::new(),
                })
                .child(self.children.clone()),
              ),
          ),
      )
  }
}

impl Group {
  /// Creates an empty group.
  pub fn new() -> Self {
    Self {
      kind: GameObjectKind::Empty,
      transform: LocalTransform::default(),
      active: true,
      render_order: None,
      material_instances: Vec::new(),
      children: Vec::new(),
      reference: None,
      click: None,
      events: PointerHandlers::new(),
      navigation: NavigationHandlers::new(),
      pointer: battlement::WorldPointerSettings::default(),
      id: None,
      motion: MotionProps::new(),
    }
  }

  pub(crate) fn into_object(self, object_id: ObjectId) -> GameObject {
    assert!(
      self.children.is_empty(),
      "static objects cannot contain logical children"
    );
    assert!(
      self.reference.is_none(),
      "static objects cannot attach render refs"
    );
    assert!(
      self.click.is_none() && self.events.is_empty() && self.navigation.is_empty(),
      "static objects cannot attach logical callbacks"
    );
    assert!(
      self.id.is_none(),
      "static object identity comes from its object ID"
    );
    assert!(
      self.motion == MotionProps::new(),
      "static objects cannot own component Motion"
    );
    let mut object = GameObject::new(object_id, self.kind);
    object.local_transform = self.transform;
    object.active = self.active;
    object.render_order = self.render_order;
    object.material_instances = self.material_instances;
    object
  }

  /// Preserves the compatible native host across logical parents and attachments.
  pub fn id(mut self, id: Uuid) -> Self {
    assert!(!id.is_nil(), "presentation IDs cannot be nil");
    self.id = Some(id);
    self
  }

  /// Sorts visual descendants together relative to the enclosing sorting group.
  pub fn sort_order(mut self, order: i16) -> Self {
    self.render_order = Some(RenderOrder::Group(order));
    self
  }

  /// Adds a logical child; UI contributions use a portal target.
  pub fn child(mut self, child: impl Render) -> Self {
    self.children.push(Node::new(child));
    self
  }
  /// Sets the local transform relative to the physical parent.
  pub fn transform(mut self, transform: LocalTransform) -> Self {
    self.transform = transform;
    self
  }
  /// Sets the local position.
  pub fn position(mut self, position: Vector3) -> Self {
    self.transform.position = position;
    self
  }
  /// Sets the local orientation without changing authored geometry.
  pub fn rotation(mut self, rotation: Quaternion) -> Self {
    self.transform.rotation = rotation;
    self
  }
  /// Sets the local scale.
  pub fn scale(mut self, scale: Vector3) -> Self {
    self.transform.scale = scale;
    self
  }
  /// Controls native activation while preserving logical component state.
  pub fn active(mut self, active: bool) -> Self {
    self.active = active;
    self
  }
  /// Attaches a reference after the host commits.
  pub fn reference(mut self, reference: ObjectRef) -> Self {
    self.reference = Some(reference);
    self
  }
  /// Installs callbacks using the same event payloads and phases as UI.
  pub fn events(mut self, events: PointerHandlers) -> Self {
    self.events = events;
    self
  }
  /// Makes this live world host eligible for keyboard/controller navigation.
  pub fn focusable(mut self, focusable: bool) -> Self {
    self.pointer.focusable = focusable;
    self
  }
  /// Installs semantic activation, focus, and navigation callbacks.
  pub fn navigation(mut self, events: NavigationHandlers) -> Self {
    self.navigation = events;
    self
  }
  /// Higher interaction layers win before visible depth is compared.
  pub fn interaction_layer(mut self, layer: i32) -> Self {
    self.pointer.interaction_layer = layer;
    self
  }
  /// Captures the primary pointer after an unprevented press until release or loss.
  pub fn capture_on_press(mut self, capture: bool) -> Self {
    self.pointer.capture_on_press = capture;
    self
  }
  /// Handles native activation or a descendant portal activation.
  pub fn on_click(mut self, callback: Callback<()>) -> Self {
    self.click = Some(callback);
    self
  }
}

impl Default for Group {
  fn default() -> Self {
    Self::new()
  }
}

impl Prefab {
  /// Creates an opaque prefab declaration with group transform and child builders.
  pub fn at(address: impl Into<PrefabAddress>) -> Group {
    Group {
      kind: GameObjectKind::prefab(address),
      ..Group::new()
    }
  }
}

impl Component for Group {
  fn render(&self) -> impl Render {
    let scene = hooks::use_required_context::<SceneAttachment>();
    let pointer_motion =
      hooks::use_context::<MotionPointerInput>().0 || self.motion.has_pointer_gestures();
    let rust_pointer =
      hooks::use_context::<RustPointerInput>().0 || self.click.is_some() || !self.events.is_empty();
    let mut pointer = self.pointer;
    pointer.forwards_ui_events = rust_pointer;
    let mut host = NativeHost::<WorldAdapter>::new(WorldDescription {
      kind: self.kind.clone(),
      scene: scene.0,
      root: false,
      transform: self.transform,
      active: self.active,
      clickable: self.click.is_some()
        || !self.events.is_empty()
        || [
          self.pointer.focusable,
          pointer_motion,
          matches!(self.kind, GameObjectKind::BoxHitRegion { .. }),
        ]
        .into_iter()
        .any(|enabled| enabled),
      world_pointer: Some(pointer),
      render_order: self.render_order,
      material_instances: self.material_instances.clone(),
    })
    .motion(self.motion.clone())
    .child(self.children.clone())
    .events(self.events.clone())
    .navigation(self.navigation.clone());
    if let Some(id) = self.id {
      host = host.id(id);
    }
    if let Some(reference) = &self.reference {
      host = host.reference(reference.clone());
    }
    if let Some(callback) = &self.click {
      host = host.on_click(callback.clone());
    }
    ContextProvider::new()
      .context(MotionPointerInput(pointer_motion))
      .child(
        ContextProvider::new()
          .context(RustPointerInput(rust_pointer))
          .child(host),
      )
  }
}

impl MotionComponent for Group {
  fn with_motion(mut self, motion: MotionProps) -> Self {
    self.motion = motion;
    self
  }
}
