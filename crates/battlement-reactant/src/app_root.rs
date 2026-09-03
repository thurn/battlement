use std::{cell::RefCell, rc::Rc};

use battlement::{
  GameObject, GameObjectKind, LengthUnits, ObjectId, PanelScaleMode, PanelSettings, ParentScene,
  PickingMode, Style, UiDocument, UiDocumentState,
};

use crate::{
  app_context::{self, AppHandle, AppQueue, Observations},
  application,
  key::KeyRenderExt,
  motion_config,
  render::{Node, Render},
  runtime::Reactant,
};

pub(crate) struct AppRoot<G> {
  pub(crate) document: UiDocument,
  pub(crate) state: UiDocumentState,
  pub(crate) view: Rc<dyn Fn(&G) -> Node>,
}

impl<G: 'static> AppRoot<G> {
  pub(crate) fn new<R: Render>(view: impl Fn(&G) -> R + 'static) -> Self {
    let document = UiDocument::new(ObjectId::new_v4())
      .name("app")
      .picking_mode(PickingMode::Ignore)
      .style(Style::new().width(100.pct()).height(100.pct()));
    Self {
      state: UiDocumentState::new(document.root_id)
        .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantLogicalPixelSize)),
      document,
      view: Rc::new(move |model| Node::new(view(model))),
    }
  }

  pub(crate) fn register(
    &self,
    runtime: &mut Reactant<G>,
    observations: &Rc<RefCell<Observations>>,
    queue: &Rc<RefCell<AppQueue>>,
  ) {
    let observations = Rc::clone(observations);
    let queue = Rc::clone(queue);
    let view = Rc::clone(&self.view);
    runtime.register_root(self.document.clone(), move |model| {
      let observed = observations.borrow();
      application::provider(observed.application).child(
        motion_config::preference_provider(observed.reduced_motion).child(
          app_context::VIEWPORT.provider(observed.screen).child(
            app_context::APP
              .provider(AppHandle::new(&queue))
              .child(view(model).key(observed.remount)),
          ),
        ),
      )
    });
  }

  pub(crate) fn object(&self) -> GameObject {
    let mut state = self.state.clone();
    // The native document host and visual root share the configured identity.
    state = state.with_root_id(self.document.root_id);
    GameObject::new(self.document.document_id, GameObjectKind::UiDocument(state))
      .parent_scene(ParentScene::Persistent)
  }
}
