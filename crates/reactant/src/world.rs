//! World attachments and opaque visuals owned by ordinary Reactant components.

use battlement::{GameObjectKind, LocalTransform, ParentScene, PrefabAddress, Vector3};
use reactant_core::{
  callback::Callback,
  component::Component,
  context::ContextProvider,
  hooks,
  native_host::{NativeHost, ObjectRef},
  render::{Node, Render},
};

use uuid::Uuid;

use crate::world_adapter::{WorldAdapter, WorldDescription};

#[derive(Clone, PartialEq)]
struct SceneAttachment(ParentScene);

/// An explicit attachment to a loaded scene or the persistent scene container.
pub struct SceneRoot {
  scene: ParentScene,
  children: Vec<Node>,
}

/// An empty transform group with logical children.
#[derive(Clone)]
pub struct Group {
  kind: GameObjectKind,
  transform: LocalTransform,
  active: bool,
  children: Vec<Node>,
  reference: Option<ObjectRef>,
  click: Option<Callback<()>>,
  id: Option<Uuid>,
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
        NativeHost::<WorldAdapter>::new(WorldDescription {
          kind: GameObjectKind::Empty,
          scene: self.scene,
          root: true,
          transform: LocalTransform::default(),
          active: true,
          clickable: false,
        })
        .child(self.children.clone()),
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
      children: Vec::new(),
      reference: None,
      click: None,
      id: None,
    }
  }

  /// Preserves the compatible native host across logical parents and attachments.
  pub fn id(mut self, id: Uuid) -> Self {
    assert!(!id.is_nil(), "presentation IDs cannot be nil");
    self.id = Some(id);
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
    let mut host = NativeHost::<WorldAdapter>::new(WorldDescription {
      kind: self.kind.clone(),
      scene: scene.0,
      root: false,
      transform: self.transform,
      active: self.active,
      clickable: self.click.is_some(),
    })
    .child(self.children.clone());
    if let Some(id) = self.id {
      host = host.id(id);
    }
    if let Some(reference) = &self.reference {
      host = host.reference(reference.clone());
    }
    if let Some(callback) = &self.click {
      host = host.on_click(callback.clone());
    }
    host
  }
}
