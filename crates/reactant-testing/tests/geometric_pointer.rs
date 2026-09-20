use battlement::{
  CameraProjection, GameObjectKind, ObjectId, PanelPoint, ParentScene, PickingMode, Position, Prop,
  UiEventDisposition, Vector3, object_id,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  callback::IntoCallback,
  event::ReactantEvent,
  host::ButtonHost,
  overlay::{Overlay, OverlayHost},
  portal,
  prelude::*,
  testing::App,
  world,
};
use reactant_testing::Display;
use std::time::Duration;
use trox::ls;

const FIRST: ObjectId = object_id!("39100000-0000-4000-8000-000000000001");
const SECOND: ObjectId = object_id!("39100000-0000-4000-8000-000000000002");
const CENTER: PanelPoint = PanelPoint::new(960.0, 540.0);

#[derive(Default)]
struct Model {
  pass: bool,
  layer: bool,
  depth: bool,
  reversed: bool,
  moved: bool,
  hidden: bool,
  destroy: bool,
  gone: bool,
  prevent: bool,
  log: Vec<String>,
}

fn control(name: &'static str, top: f32, action: fn(&mut Model)) -> impl Render {
  ButtonHost::new(ls(name))
    .name(name)
    .style(
      Style::new()
        .position(Position::Absolute)
        .left(10.0)
        .top(top)
        .width(160.0)
        .height(40.0),
    )
    .on_click(action)
}
fn region(id: ObjectId, model: &Model) -> impl Render + use<> {
  let name = if id == FIRST { "first" } else { "second" };
  world::BoxHitRegion::new()
    .id(*id.as_uuid())
    .size(Vector3::new(3.0, 3.0, 0.2))
    .position(Vector3::new(
      0.0,
      0.0,
      if id == FIRST && model.depth {
        -1.0
      } else {
        0.0
      },
    ))
    .capture_on_press(true)
    .interaction_layer(if id == FIRST && model.layer { 1 } else { 0 })
    .events(
      world::PointerHandlers::new()
        .on_pointer_down(
          move |m: &mut Model, e: ReactantEvent<battlement::PointerButtonEvent>| {
            m.log.push(format!("{name}:down"));
            if m.prevent {
              e.prevent_default();
            }
          },
        )
        .on_pointer_move(
          move |m: &mut Model, e: ReactantEvent<battlement::PointerMoveEvent>| {
            if e.payload().buttons != 0 {
              m.moved = true;
              m.log.push(format!("{name}:drag"));
            }
            if e.payload().position.x > 1300.0 {
              if m.destroy {
                m.gone = true;
              } else {
                m.hidden = true;
              }
            }
          },
        )
        .on_pointer_capture_out(
          move |m: &mut Model, _: ReactantEvent<battlement::PointerCaptureEvent>| {
            m.log.push(format!("{name}:lost"));
          },
        )
        .on_click(
          move |m: &mut Model, _: ReactantEvent<battlement::ClickEvent>| {
            m.log.push(format!("{name}:click"));
          },
        ),
    )
}
fn view(model: &Model) -> impl Render + use<> {
  let mut objects = vec![
    Node::new(self::region(FIRST, model)),
    Node::new(self::region(SECOND, model)),
  ];
  if model.reversed {
    objects.reverse();
  }
  let objects = (!model.gone).then(|| {
    world::Group::new()
      .id(*object_id!("39100000-0000-4000-8000-000000003911").as_uuid())
      .active(!model.hidden)
      .child(objects)
  });
  (
    self::control("Pass", 10.0, |m| m.pass = !m.pass),
    self::control("Layer", 60.0, |m| m.layer = !m.layer),
    self::control("Reverse", 110.0, |m| m.reversed = !m.reversed),
    self::control("Show", 160.0, |m| m.hidden = false),
    self::control("Destroy", 210.0, |m| m.destroy = true),
    self::control("Prevent", 260.0, |m| m.prevent = true),
    self::control("Depth", 310.0, |m| m.depth = !m.depth),
    View::new()
      .picking_mode(if model.pass {
        PickingMode::Ignore
      } else {
        PickingMode::Position
      })
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(800.0)
          .top(400.0)
          .width(320.0)
          .height(280.0),
      )
      .on_click(|m: &mut Model| m.log.push("ui:click".into())),
    Label::new(ls(model.log.join(",")))
      .name("log")
      .picking_mode(PickingMode::Ignore)
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(10.0)
          .top(320.0)
          .height(30.0),
      ),
    world::SceneRoot::new(ParentScene::PrimaryScene).child((
      world::Group::new().child((!model.moved).then(|| objects.clone())),
      world::Group::new().child(model.moved.then_some(objects)),
    )),
  )
}
fn fixture() -> Display<App<Model>> {
  let app = App::with_model("pointer/scene", Model::default())
    .root(self::view)
    .document(|mut doc| {
      doc.element.picking_mode = Prop::Set(PickingMode::Ignore);
      doc
    })
    .camera(|mut object| {
      object.local_transform.position = Vector3::new(0.0, 0.0, -10.0);
      if let GameObjectKind::Camera { camera } = &mut object.kind {
        camera.projection = CameraProjection::Orthographic;
        camera.orthographic_size = 5.0;
      }
      object
    });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("pointer/scene");
  Display::connect(app, assets)
}
fn log(display: &mut Display<App<Model>>) -> Vec<String> {
  display.with_engine(|app| app.model().log.clone())
}
#[test]
fn geometry_blocks_passthrough_and_layer_depth_sibling_ties() {
  let mut display = self::fixture();
  display.click_at(CENTER);
  assert_eq!(self::log(&mut display), ["ui:click"]);
  display.click_at(PanelPoint::new(30.0, 30.0));
  display.click_at(CENTER);
  assert!(self::log(&mut display).ends_with(&[
    "second:down".into(),
    "second:click".into(),
    "second:lost".into()
  ]));
  display.click_at(PanelPoint::new(30.0, 130.0));
  display.click_at(CENTER);
  assert!(self::log(&mut display).contains(&"first:click".into()));
  display.click_at(PanelPoint::new(30.0, 130.0));
  display.click_at(PanelPoint::new(30.0, 80.0));
  display.click_at(CENTER);
  assert!(self::log(&mut display).ends_with(&[
    "first:down".into(),
    "first:click".into(),
    "first:lost".into()
  ]));
  assert_eq!(display.presentation_time(), Duration::ZERO);
  assert_eq!(display.frame(), 0);
}
#[test]
fn captured_reparent_hide_show_destroy_and_prevention_are_observable() {
  let mut display = self::fixture();
  display.click_at(PanelPoint::new(30.0, 30.0));
  display.pointer_down(0, CENTER);
  assert_eq!(display.pointer_capture(0), Some(SECOND));
  display.pointer_move(0, PanelPoint::new(1200.0, 540.0), true);
  assert_eq!(display.pointer_capture(0), Some(SECOND));
  let losses = display.capture_losses().len();
  display.pointer_move(0, PanelPoint::new(1400.0, 540.0), true);
  assert_eq!(display.pointer_capture(0), None);
  assert_eq!(display.capture_losses().len(), losses + 1);
  display.pointer_up(0, CENTER);
  display.click_at(PanelPoint::new(30.0, 180.0));
  display.pointer_move(0, CENTER, false);
  assert_eq!(display.pointer_capture(0), None);
  assert_eq!(display.capture_losses().len(), losses + 1);
  display.click_at(PanelPoint::new(30.0, 230.0));
  display.pointer_down(0, CENTER);
  display.pointer_move(0, PanelPoint::new(1400.0, 540.0), true);
  assert_eq!(display.pointer_capture(0), None);
  assert_eq!(display.capture_losses().len(), losses + 2);
  assert!(display.object(SECOND).is_none());
  assert_eq!(
    self::log(&mut display)
      .iter()
      .filter(|s| s.ends_with(":lost"))
      .count(),
    1,
    "destruction detaches handlers before the host reports loss; hidden live targets receive it"
  );
  let mut prevented = self::fixture();
  prevented.click_at(PanelPoint::new(30.0, 30.0));
  prevented.click_at(PanelPoint::new(30.0, 280.0));
  assert_eq!(
    prevented.pointer_down(0, CENTER),
    UiEventDisposition::PreventDefault
  );
  assert_eq!(prevented.pointer_capture(0), None);
}

#[test]
fn nested_modals_block_world_until_the_last_scope_closes() {
  let mut app = App::with_model("pointer/scene", (true, true, Vec::<String>::new()));
  let target = app.create_portal_target();
  app = app
    .root(move |m| {
      Stack::new().picking_mode(PickingMode::Ignore).child((
        world::SceneRoot::new(ParentScene::PrimaryScene).child(
          world::BoxHitRegion::new()
            .size(Vector3::new(3.0, 3.0, 0.2))
            .on_click(
              (|m: &mut (bool, bool, Vec<String>)| m.2.push("world".into())).into_callback(),
            ),
        ),
        m.0.then(|| {
          Overlay::modal(target.clone(), ls("outer")).child((
            ButtonHost::new(ls("Close outer"))
              .style(
                Style::new()
                  .position(Position::Absolute)
                  .left(800.0)
                  .top(400.0)
                  .width(320.0)
                  .height(280.0),
              )
              .on_click(|m: &mut (bool, bool, Vec<String>)| {
                m.0 = false;
                m.2.push("outer".into());
              }),
            m.1.then(|| {
              Overlay::modal(target.clone(), ls("inner")).child(
                ButtonHost::new(ls("Close inner"))
                  .style(
                    Style::new()
                      .position(Position::Absolute)
                      .left(800.0)
                      .top(400.0)
                      .width(320.0)
                      .height(280.0),
                  )
                  .on_click(|m: &mut (bool, bool, Vec<String>)| {
                    m.1 = false;
                    m.2.push("inner".into());
                  }),
              )
            }),
          ))
        }),
        OverlayHost::new(target.clone()),
      ))
    })
    .document(|mut doc| {
      doc.element.picking_mode = Prop::Set(PickingMode::Ignore);
      doc
    })
    .camera(|mut object| {
      object.local_transform.position = Vector3::new(0.0, 0.0, -10.0);
      if let GameObjectKind::Camera { camera } = &mut object.kind {
        camera.projection = CameraProjection::Orthographic;
        camera.orthographic_size = 5.0;
      }
      object
    });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("pointer/scene");
  let mut display = Display::connect(app, assets);
  display.click_at(CENTER);
  assert_eq!(display.with_engine(|a| a.model().2.clone()), ["inner"]);
  display.click_at(CENTER);
  assert_eq!(
    display.with_engine(|a| a.model().2.clone()),
    ["inner", "outer"]
  );
  display.click_at(CENTER);
  assert_eq!(
    display.with_engine(|a| a.model().2.clone()),
    ["inner", "outer", "world"]
  );
}

#[test]
fn geometric_portal_dispatch_uses_logical_world_ancestry_and_returns_prevention() {
  let mut app = App::with_model("pointer/scene", Vec::<String>::new());
  let target = app.create_portal_target();
  app = app.root(move |_| {
    (
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .events(
            world::PointerHandlers::new()
              .on_pointer_down_capture(
                |m: &mut Vec<String>, _: ReactantEvent<battlement::PointerButtonEvent>| {
                  m.push("world capture".into())
                },
              )
              .on_pointer_down(
                |m: &mut Vec<String>, _: ReactantEvent<battlement::PointerButtonEvent>| {
                  m.push("world bubble".into())
                },
              ),
          )
          .child(portal::create_portal(
            View::new()
              .style(
                Style::new()
                  .position(Position::Absolute)
                  .left(800.0)
                  .top(400.0)
                  .width(320.0)
                  .height(280.0),
              )
              .on_pointer_down_capture_event_with_model(
                |m: &mut Vec<String>, _: ReactantEvent<battlement::PointerButtonEvent>| {
                  m.push("target capture".into())
                },
              )
              .on_pointer_down_event_with_model(
                |m: &mut Vec<String>, e: ReactantEvent<battlement::PointerButtonEvent>| {
                  m.push("target".into());
                  e.prevent_default();
                },
              ),
            target.clone(),
          )),
      ),
      View::new()
        .portal_target(target.clone())
        .picking_mode(PickingMode::Ignore)
        .on_pointer_down(|m: &mut Vec<String>| m.push("physical parent must not receive".into())),
    )
  });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("pointer/scene");
  let mut display = Display::connect(app, assets);
  assert_eq!(
    display.pointer_down(0, CENTER),
    UiEventDisposition::PreventDefault
  );
  assert_eq!(
    display.with_engine(|a| a.model().clone()),
    ["world capture", "target capture", "target", "world bubble"]
  );
}

#[test]
fn visible_depth_and_cancel_are_geometric_without_implicit_frames() {
  let mut display = self::fixture();
  display.click_at(PanelPoint::new(30.0, 30.0));
  display.click_at(PanelPoint::new(30.0, 330.0));
  display.pointer_down(0, CENTER);
  assert_eq!(display.pointer_capture(0), Some(FIRST));
  display.pointer_cancel(0);
  assert_eq!(display.pointer_capture(0), None);
  assert_eq!(display.capture_losses(), &[(0, FIRST)]);
  display.pointer_up(0, CENTER);
  assert!(
    !self::log(&mut display)
      .iter()
      .any(|s| s.ends_with(":click"))
  );
  assert_eq!(display.frame(), 0);
  assert_eq!(display.presentation_time(), Duration::ZERO);
}
