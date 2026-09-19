use battlement::{CameraProjection, CameraState, GameObject, ParentScene, Vector3};
use reactant::{
  callback::Callback,
  hooks,
  portal::{self, PortalTarget},
  prelude::*,
};
use trox::ls;

use crate::ROOT_ID;

#[derive(Clone, Default, PartialEq)]
struct SharedCount(u32);

struct MixedCounter(PortalTarget);
struct Visual(Callback<()>);
struct Details(Callback<()>);

pub(crate) fn app() -> crate::ReactantEngine {
  let mut app = reactant::app::App::new(crate::CONTENT_SCENE);
  let target = app.create_portal_target();
  app
    .ui((
      MixedCounter(target.clone()),
      View::new().portal_target(target).style(
        Style::new()
          .width(420.px())
          .padding(32.px())
          .color(Color::WHITE)
          .background_color(Color::rgb(0.06, 0.09, 0.15)),
      ),
    ))
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
    .camera(|camera| {
      GameObject::new(
        camera.object_id,
        CameraState::new()
          .projection(CameraProjection::Orthographic)
          .orthographic_size(5.0),
      )
      .parent_scene(ParentScene::Persistent)
      .position(Vector3::new(0.0, 0.0, -10.0))
    })
}

impl Component for MixedCounter {
  fn render(&self) -> impl Render {
    let (count, set_count) = hooks::use_state(0_u32);
    let (world, set_world) = hooks::use_state(true);
    let (details, set_details) = hooks::use_state(true);
    let increment = set_count.update_callback(|value| value + 1);
    (
      portal::create_portal(
        View::new().child((
          Label::new(ls("One component, two hosts")).style(Style::new().font_size(28.px())),
          Label::new(ls("The portal and prefab share one counter."))
            .style(Style::new().font_size(16.px())),
          Button::new(ls("Toggle world")).on_press(move || set_world.set(!world)),
          Button::new(ls("Toggle details")).on_press(move || set_details.set(!details)),
        )),
        self.0.clone(),
      ),
      ContextProvider::new().context(SharedCount(count)).child(
        SceneRoot::new(ParentScene::PrimaryScene).child(WorldGroup::new().child((
          world.then(|| Visual(increment.clone())),
          details.then(|| portal::create_portal(Details(increment.clone()), self.0.clone())),
        ))),
      ),
    )
  }
}

impl Component for Visual {
  fn render(&self) -> impl Render {
    let count = hooks::use_context::<SharedCount>().0;
    Prefab::at("reactant/mixed-visual")
      .position(Vector3::new(3.0 + f64::from(count) * 0.35, 0.0, 0.0))
      .scale(Vector3::new(1.5, 1.5, 1.5))
      .on_click(self.0.clone())
  }
}

impl Component for Details {
  fn render(&self) -> impl Render {
    let count = hooks::use_context::<SharedCount>().0;
    View::new().child((
      Heading::new(ls(format!("Shared count: {count}")), 2).style(Style::new().font_size(24.px())),
      Button::new(ls("Increment shared count")).on_press(self.0.clone()),
    ))
  }
}
