//! Batteries-included Reactant applications hosted by Battlement.

use std::{cell::RefCell, rc::Rc};

use battlement::{
  CameraClearMode, CameraState, Color, GameObject, GameObjectKind, ObjectId, PanelScaleMode,
  PanelSettings, ParentScene, Scene, SceneAddress, SceneId, ScreenSize, SessionId, UiDocument,
  UiDocumentState,
};

use crate::{
  app_context::{AppQueue, Observations},
  app_delivery::Delivery,
  app_root::AppRoot,
  cooperative_executor::CooperativeExecutor,
  executor::{BoxFuture, SpawnedTask, Spawner},
  portal::PortalTarget,
  render::{Node, Render},
  runtime::Reactant,
};

/// Owns the rendering, sessions, native integration, and lifetime of an application.
pub struct App<G: 'static = ()> {
  pub(crate) model: G,
  pub(crate) runtime: Reactant<G>,
  pub(crate) roots: Vec<AppRoot<G>>,
  pub(crate) scene: Scene,
  pub(crate) camera: GameObject,
  pub(crate) objects: Vec<GameObject>,
  pub(crate) executor: CooperativeExecutor,
  spawner: SharedSpawner,
  pub(crate) observations: Rc<RefCell<Observations>>,
  pub(crate) queue: Rc<RefCell<AppQueue>>,
  pub(crate) delivery: Delivery,
  pub(crate) session: Option<SessionId>,
  pub(crate) reset: bool,
  pub(crate) healthy: bool,
}

impl App {
  /// Creates a full-window application from a scene address and root component.
  pub fn new(scene: impl Into<SceneAddress>, component: impl Render) -> Self {
    let component = Node::new(component);
    Self::with_model(scene, ()).root(move |_| component.clone())
  }
}

impl<G: 'static> App<G> {
  /// Creates an application with game-owned state; select its view with [`Self::root`].
  pub fn with_model(scene: impl Into<SceneAddress>, model: G) -> Self {
    let executor = CooperativeExecutor::default();
    let spawner = SharedSpawner(Rc::new(RefCell::new(Box::new(executor.clone()))));
    let runtime = Reactant::new(spawner.clone());
    runtime.resources.cache.borrow_mut().attributed = true;
    Self {
      model,
      runtime,
      roots: vec![AppRoot::new(|_| ())],
      scene: Scene::new(SceneId::new_v4(), scene),
      camera: GameObject::new(
        ObjectId::new_v4(),
        CameraState::new()
          .clear_mode(CameraClearMode::SolidColor)
          .clear_color(Color::rgb(0.0, 0.0, 0.0)),
      )
      .parent_scene(ParentScene::Persistent),
      objects: Vec::new(),
      executor,
      spawner,
      observations: Rc::new(RefCell::new(Observations {
        application: Default::default(),
        screen: ScreenSize::new(0, 0),
        remount: 0,
      })),
      queue: Rc::new(RefCell::new(AppQueue::default())),
      delivery: Delivery::default(),
      session: None,
      reset: false,
      healthy: true,
    }
  }

  /// Sets the primary document's component factory.
  pub fn root<R: Render>(mut self, view: impl Fn(&G) -> R + 'static) -> Self {
    self.require_configuring();
    self.roots[0].view = Rc::new(move |model| Node::new(view(model)));
    self
  }

  /// Customizes the primary document without creating its identities or native host.
  pub fn document(mut self, configure: impl FnOnce(UiDocument) -> UiDocument) -> Self {
    self.require_configuring();
    self.roots[0].document = configure(self.roots[0].document.clone());
    self
  }

  /// Sets the camera's solid background color.
  pub fn background(mut self, color: Color) -> Self {
    self.require_configuring();
    let GameObjectKind::Camera { camera } = &mut self.camera.kind else {
      unreachable!()
    };
    camera.clear_mode = CameraClearMode::SolidColor;
    camera.clear_color = color;
    self
  }

  /// Customizes the input camera, including its identity and transform.
  pub fn camera(mut self, configure: impl FnOnce(GameObject) -> GameObject) -> Self {
    self.require_configuring();
    self.camera = configure(self.camera.clone());
    assert!(
      matches!(self.camera.kind, GameObjectKind::Camera { .. }),
      "application camera must be a camera"
    );
    self
  }

  /// Adds a game-owned object to each replacement snapshot.
  pub fn object(mut self, object: GameObject) -> Self {
    self.require_configuring();
    self.objects.push(object);
    self
  }

  /// Customizes the primary document's native panel settings.
  pub fn panel(mut self, configure: impl FnOnce(UiDocumentState) -> UiDocumentState) -> Self {
    self.require_configuring();
    self.roots[0].state = configure(self.roots[0].state.clone());
    self
  }

  /// Adds another document root; the application creates and owns its native host.
  pub fn additional_root<R: Render>(
    mut self,
    document: UiDocument,
    view: impl Fn(&G) -> R + 'static,
  ) -> Self {
    self.require_configuring();
    let mut root = AppRoot::new(view);
    root.state = UiDocumentState::new(document.root_id)
      .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantPixelSize));
    root.document = document;
    self.roots.push(root);
    self
  }

  /// Creates a target for portals used by this application's component factories.
  pub fn create_portal_target(&mut self) -> PortalTarget {
    self.require_configuring();
    self.runtime.create_portal_target()
  }

  /// Uses a specialized asynchronous executor instead of the built-in executor.
  pub fn spawner(self, spawner: impl Spawner) -> Self {
    self.require_configuring();
    *self.spawner.0.borrow_mut() = Box::new(spawner);
    self
  }

  /// Remounts component state on each connection while retaining the game model.
  pub fn reset_on_reconnect(mut self) -> Self {
    self.require_configuring();
    self.reset = true;
    self
  }

  /// Returns game-owned state for host-side inspection.
  pub fn model(&self) -> &G {
    &self.model
  }

  /// Returns the generated primary document for inspection and test discovery.
  pub fn root_document(&self) -> &UiDocument {
    &self.roots[0].document
  }

  pub(crate) fn require_configuring(&self) {
    assert!(
      self.session.is_none(),
      "configure applications before connecting"
    );
  }
}

#[derive(Clone)]
struct SharedSpawner(Rc<RefCell<Box<dyn Spawner>>>);

impl Spawner for SharedSpawner {
  fn spawn(&self, task: BoxFuture<'static, ()>) -> SpawnedTask {
    self.0.borrow().spawn(task)
  }
}
