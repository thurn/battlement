use crate::ROOT_ID;
use battlement::{ParentScene, PickingMode, Prop, Vector3};
use reactant::{
  hooks,
  overlay::{Overlay, OverlayHost},
  portal::PortalTarget,
  prelude::*,
  world,
};
use trox::ls;

struct NavigationProof {
  target: PortalTarget,
}
pub(crate) fn app() -> crate::ReactantEngine {
  let mut app = reactant::app::App::new(crate::CONTENT_SCENE);
  let target = app.create_portal_target();
  app = app
    .ui(NavigationProof { target })
    .document(|mut doc| {
      doc.root_id = ROOT_ID;
      doc.element.picking_mode = Prop::Set(PickingMode::Ignore);
      doc
    })
    .camera(|camera| {
      world::Camera::new()
        .orthographic(4.0)
        .clipping(0.1, 50.0)
        .background(Color::rgb(0.035, 0.05, 0.08))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    });
  app
}
impl Component for NavigationProof {
  fn render(&self) -> impl Render {
    let (focused, focus) = hooks::use_state(None::<usize>);
    let (modal, show) = hooks::use_state(false);
    let (removed, remove) = hooks::use_state(false);
    let (activations, activate) = hooks::use_state(0);
    let (captured, capture) = hooks::use_state(Vec::<i32>::new());
    let close = show.clone();
    Stack::new().picking_mode(PickingMode::Ignore).style(Style::new().width(100.pct()).height(100.pct())).child((
      View::new().picking_mode(PickingMode::Ignore).style(Style::new().width(100.pct()).height(100.pct()).padding(24.px()).color(Color::WHITE)).child((
        Heading::new(ls("World keyboard, controller and touch"),1),
        Heading::new(ls(format!("Focused: {} | Activations: {activations}",focused.map(|v|(v+1).to_string()).unwrap_or_else(||"none".into()))),2),
        Label::new(ls("Arrows / D-pad: navigate. Enter / A: activate. Escape / B: close menu; remove focused card.")).picking_mode(PickingMode::Ignore),
        Label::new(ls(format!("Touch captures: {captured:?}"))).picking_mode(PickingMode::Ignore),
        world::SceneRoot::new(ParentScene::PrimaryScene).child((0..2).filter(|index|*index!=1 || !removed).map(|index|{
          let gain=focus.clone();let lose=focus.clone();let show=show.clone();let activate=activate.clone();let remove=remove.clone();
          let capture_on=capture.clone();let capture_off=capture.clone();
          world::Group::new().position(Vector3::new(if index==0 {-2.0}else{2.0},-0.2,0.0)).child((
            world::Sprite::new().texture("reactant/assets/texture").size(2.8,2.6)
              .tint(if focused==Some(index) {battlement::RgbColor::rgb(1.0,0.82,0.15)}else{battlement::RgbColor::rgb(0.20,0.40,0.55)}),
            world::Text::new().font("reactant/world/font").text(format!("CONTROL {}\n{}",index+1,if focused==Some(index){"FOCUSED"}else{"READY"}))
              .size(2.6).color(Color::WHITE).position(Vector3::new(0.0,0.0,-0.2)).layer(3),
            world::BoxHitRegion::new().size(Vector3::new(2.8,2.6,0.2)).focusable(true).capture_on_press(true)
              .navigation(world::NavigationHandlers::new()
                .on_focus(gain.update_callback(move |_|Some(index)))
                .on_blur(lose.update_callback(move |current|if current==Some(index){None}else{current}))
                .on_activate(reactant::callback::IntoCallback::into_callback(move ||{show.set(true);activate.update(|v|v+1);}))
                .on_cancel(remove.update_callback(move |value|value || index==1)))
              .events(world::PointerHandlers::new()
                .on_pointer_capture(move |event:reactant::event::ReactantEvent<battlement::PointerCaptureEvent>| {let id=event.payload().pointer_id;capture_on.update(move |v|{let mut next=v.clone();next.push(id);next});})
                .on_pointer_capture_out(move |event:reactant::event::ReactantEvent<battlement::PointerCaptureEvent>| {let id=event.payload().pointer_id;capture_off.update(move |v|v.iter().copied().filter(|value|*value!=id).collect());})),
          ))
        }).collect::<Vec<_>>()),
        modal.then(||Overlay::modal(self.target.clone(),ls("World menu")).on_dismiss(close.update_callback(|_|false))
          .child(View::new().style(Style::new().width(420.px()).padding(24.px()).background_color(Color::rgb(0.08,0.14,0.22)).color(Color::WHITE))
            .child((Heading::new(ls("World menu"),1),Label::new(ls("Escape / B returns focus to the same world control.")),Button::new(ls("Close menu")).on_press(show.update_callback(|_|false))))))
      )),
      OverlayHost::new(self.target.clone()),
    ))
  }
}
#[cfg(test)]
mod tests {
  use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
  #[test]
  fn navigation_scene_mounts() {
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(crate::CONTENT_SCENE);
    assets.add_texture("reactant/assets/texture");
    assets.add_text_mesh_pro_font("reactant/world/font");
    assets.add_textures(crate::generated_asset_addresses());
    let _ = FakeClient::connect(crate::navigation_proof::app(), assets);
  }
}
