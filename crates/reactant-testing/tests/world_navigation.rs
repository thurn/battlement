use battlement::{
  ControllerButton, ControllerInputSettings, GameObjectKind, NavigationDirection, ObjectId,
  PanelPoint, ParentScene, PhysicalKey, PickingMode, Prop, Vector3, object_id,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  callback::IntoCallback,
  event::ReactantEvent,
  host::ButtonHost,
  overlay::{Overlay, OverlayHost},
  prelude::*,
  testing::App,
  world,
};
use reactant_testing::Display;
use trox::ls;

const ROOT: ObjectId = object_id!("40200000-0000-4000-8000-000000000010");
const FIRST: ObjectId = object_id!("40200000-0000-4000-8000-000000000001");
const SECOND: ObjectId = object_id!("40200000-0000-4000-8000-000000000002");
#[derive(Default)]
struct Model {
  focused: Option<ObjectId>,
  modal: bool,
  hidden: bool,
  removed: bool,
  activations: usize,
  clicks: Vec<i32>,
  navigation: Vec<(NavigationDirection, battlement::Vector)>,
  core_actions: usize,
}
fn fixture() -> Display<App<Model>> {
  let mut app = App::with_model("navigation/scene", Model::default());
  let target = app.create_portal_target();
  app = app
    .root(move |model| {
      Stack::new().picking_mode(PickingMode::Ignore).child((
        Label::new(ls(format!(
          "Focused: {:?}; activations: {}",
          model.focused, model.activations
        )))
        .name("status")
        .picking_mode(PickingMode::Ignore),
        ButtonHost::new(ls("Remove second"))
          .name("remove")
          .style(Style::new().width(100.0).height(40.0))
          .on_click(|m: &mut Model| m.removed = !m.removed),
        world::SceneRoot::new(ParentScene::PrimaryScene).child(
          [FIRST, SECOND]
            .into_iter()
            .filter(|id| *id != SECOND || !model.removed)
            .map(|id| {
              world::BoxHitRegion::new()
                .id(*id.as_uuid())
                .focusable(true)
                .capture_on_press(true)
                .active(id != SECOND || !model.hidden)
                .size(Vector3::new(1.8, 1.8, 0.2))
                .position(Vector3::new(if id == FIRST { -2.0 } else { 2.0 }, 0.0, 0.0))
                .navigation(
                  world::NavigationHandlers::new()
                    .on_navigate(
                      |m: &mut Model, e: ReactantEvent<battlement::NavigationMoveEvent>| {
                        m.navigation
                          .push((e.payload().direction, e.payload().move_vector));
                      },
                    )
                    .on_focus((move |m: &mut Model| m.focused = Some(id)).into_callback())
                    .on_blur(
                      (move |m: &mut Model| {
                        if m.focused == Some(id) {
                          m.focused = None;
                        }
                      })
                      .into_callback(),
                    )
                    .on_activate(
                      (|m: &mut Model| {
                        m.modal = true;
                        m.activations += 1;
                      })
                      .into_callback(),
                    )
                    .on_cancel((|m: &mut Model| m.hidden = true).into_callback()),
                )
                .events(world::PointerHandlers::new().on_click(
                  |m: &mut Model, e: ReactantEvent<battlement::ClickEvent>| {
                    if let battlement::ClickEvent::Pointer { pointer_id, .. } = e.payload() {
                      m.clicks.push(*pointer_id);
                    }
                  },
                ))
            })
            .collect::<Vec<_>>(),
        ),
        model.modal.then(|| {
          Overlay::modal(target.clone(), ls("World control menu"))
            .on_dismiss(|m: &mut Model| m.modal = false)
            .child(
              ButtonHost::new(ls("Close menu"))
                .name("close")
                .on_click(|m: &mut Model| m.modal = false),
            )
        }),
        OverlayHost::new(target.clone()),
      ))
    })
    .document(|mut doc| {
      doc.root_id = ROOT;
      doc.element.picking_mode = Prop::Set(PickingMode::Ignore);
      doc
    })
    .camera(|mut object| {
      object.local_transform.position = Vector3::new(0.0, 0.0, -10.0);
      if let GameObjectKind::Camera { camera } = &mut object.kind {
        camera.orthographic_size = 5.0;
        camera.projection = battlement::CameraProjection::Orthographic;
      }
      object
    })
    .global_keys([PhysicalKey::ArrowRight, PhysicalKey::Enter])
    .controller_input(ControllerInputSettings::new().buttons([ControllerButton::South]))
    .on_core_action(|model, _| model.core_actions += 1);
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("navigation/scene");
  Display::connect(app, assets)
}

#[test]
fn physical_keyboard_and_controller_inputs_follow_native_world_focus_arbitration() {
  let mut display = fixture();
  display.key_down(PhysicalKey::ArrowRight);
  assert_eq!(display.focused(), Some(FIRST));
  display.controller_navigate(0, battlement::ControllerDirection::Right);
  assert_eq!(display.focused(), Some(SECOND));
  display.controller_button_down(0, ControllerButton::South);
  display.with_engine(|app| {
    assert_eq!(app.model().activations, 1);
    assert_eq!(app.model().core_actions, 0);
  });
}
#[test]
fn semantic_world_focus_modal_return_and_removal_do_not_synthesize_clicks() {
  let mut display = fixture();
  display.navigate(NavigationDirection::Right);
  assert_eq!(display.focused(), Some(FIRST));
  display.navigate(NavigationDirection::Right);
  assert_eq!(display.focused(), Some(SECOND));
  display.activate_focused();
  assert_ne!(display.focused(), Some(SECOND));
  display.with_engine(|app| {
    assert!(app.model().modal);
    assert_eq!(app.model().activations, 1);
    assert!(app.model().clicks.is_empty());
  });
  display.cancel_navigation();
  assert_eq!(display.focused(), Some(SECOND));
  display.with_engine(|app| {
    assert!(!app.model().modal);
    assert_eq!(app.model().focused, Some(SECOND));
  });
  display.cancel_navigation();
  assert_eq!(display.focused(), Some(FIRST));
  display.with_engine(|app| assert_eq!(app.model().activations, 1));
  assert_eq!(display.frame(), 0);
  assert_eq!(display.presentation_time(), std::time::Duration::ZERO);
}
#[test]
fn two_touch_ids_capture_independently_and_cancellation_never_activates() {
  let mut display = fixture();
  let first = PanelPoint::new(744.0, 540.0);
  let second = PanelPoint::new(1176.0, 540.0);
  display.pointer_down(11, first);
  display.pointer_down(12, second);
  assert_eq!(display.pointer_capture(11), Some(FIRST));
  assert_eq!(display.pointer_capture(12), Some(SECOND));
  display.pointer_cancel(11);
  assert_eq!(display.pointer_capture(11), None);
  assert_eq!(display.pointer_capture(12), Some(SECOND));
  display.pointer_up(11, first);
  display.pointer_up(12, second);
  display.with_engine(|app| {
    assert_eq!(app.model().clicks, [12]);
    assert_eq!(app.model().activations, 0);
  });
  display.navigate(NavigationDirection::Right);
  display.activate_focused();
  display.pointer_down(21, second);
  assert_eq!(display.pointer_capture(21), None);
  display.pointer_cancel(21);
  display.cancel_navigation();
  display.with_engine(|app| assert_eq!(app.model().activations, 1));
}

#[test]
fn removal_repairs_focus_and_stale_activation_cannot_reach_a_remounted_control() {
  let mut display = fixture();
  display.navigate(NavigationDirection::Right);
  display.navigate(NavigationDirection::Right);
  let remove = display.find_ui(ROOT, "remove");
  display.click_ui(remove);
  assert_eq!(display.focused(), Some(FIRST));
  assert!(display.object(SECOND).is_none());
  display.deliver_ui_event(battlement::UiEvent::click(
    SECOND,
    battlement::ClickEvent::NavigationSubmit,
  ));
  display.click_ui(remove);
  display.navigate(NavigationDirection::Right);
  assert_ne!(display.focused(), Some(SECOND));
  assert_ne!(display.focused(), Some(FIRST));
  display.deliver_ui_event(battlement::UiEvent::click(
    SECOND,
    battlement::ClickEvent::NavigationSubmit,
  ));
  display.with_engine(|app| assert_eq!(app.model().activations, 0));
  display.activate_focused();
  display.with_engine(|app| assert_eq!(app.model().activations, 1));
  display.activate_focused();
  assert!(display.focused().is_some());
  display.with_engine(|app| assert!(!app.model().modal));
}

#[test]
fn semantic_vertical_navigation_preserves_native_callback_coordinates() {
  let mut display = fixture();
  display.navigate(NavigationDirection::Right);
  display.navigate(NavigationDirection::Up);
  display.navigate(NavigationDirection::Down);
  display.with_engine(|app| {
    assert_eq!(
      app.model().navigation,
      vec![
        (NavigationDirection::Up, battlement::Vector::new(0.0, 1.0)),
        (
          NavigationDirection::Down,
          battlement::Vector::new(0.0, -1.0)
        ),
      ]
    )
  });
}
