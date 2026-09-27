use battlement::{
  Display as UiDisplay, ObjectId, PanelPoint, PickingMode, Position, Prop, object_id,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{element_ref, event::ReactantEvent, host::ButtonHost, prelude::*, testing::App};
use reactant_testing::Display;
use trox::ls;

const TARGET: ObjectId = object_id!("39190000-0000-4000-8000-000000000011");
const COMPONENT: ObjectId = object_id!("39190000-0000-4000-8000-000000000012");
#[derive(Default)]
struct Model {
  moved: bool,
  hidden: bool,
  gone: bool,
  remove: bool,
  gained: usize,
  lost: usize,
  clicks: usize,
  canceled: usize,
}
struct CapturedUi(bool);
impl Component for CapturedUi {
  fn render(&self) -> impl Render {
    let reference = element_ref::use_element_ref();
    let capture = reference.clone();
    View::new()
      .id(*TARGET.as_uuid())
      .element_ref(reference)
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(300.0)
          .top(300.0)
          .width(100.0)
          .height(100.0)
          .display(if self.0 {
            UiDisplay::None
          } else {
            UiDisplay::Flex
          }),
      )
      .on_pointer_down_event(move |e: ReactantEvent<battlement::PointerButtonEvent>| {
        capture.capture_pointer(e.payload().pointer_id)
      })
      .on_pointer_move_event_with_model(
        |m: &mut Model, e: ReactantEvent<battlement::PointerMoveEvent>| {
          if e.payload().buttons != 0 {
            m.moved = true;
          }
          if e.payload().position.x > 1000.0 {
            if m.remove {
              m.gone = true;
            } else {
              m.hidden = true;
            }
          }
        },
      )
      .on_pointer_capture_event_with_model(
        |m: &mut Model, _: ReactantEvent<battlement::PointerCaptureEvent>| m.gained += 1,
      )
      .on_pointer_capture_out_event_with_model(
        |m: &mut Model, _: ReactantEvent<battlement::PointerCaptureEvent>| m.lost += 1,
      )
      .on_pointer_cancel_event_with_model(
        |m: &mut Model, _: ReactantEvent<battlement::PointerCancelEvent>| m.canceled += 1,
      )
      .on_click(|m: &mut Model| m.clicks += 1)
  }
}
fn parent() -> View {
  View::new().picking_mode(PickingMode::Ignore).style(
    Style::new()
      .position(Position::Absolute)
      .left(0.0)
      .top(0.0)
      .width(1920.0)
      .height(1080.0),
  )
}
#[test]
fn ui_capture_reparents_and_loses_once_without_resuming_after_show_or_removal() {
  let app = App::with_model("pointer/ui", Model::default())
    .root(|m| {
      let target = (!m.gone).then(|| Node::new(CapturedUi(m.hidden).id(*COMPONENT.as_uuid())));
      (
        ButtonHost::new(ls("Show and remove next drag"))
          .style(
            Style::new()
              .position(Position::Absolute)
              .left(0.0)
              .top(0.0)
              .width(180.0)
              .height(40.0),
          )
          .on_click(|m: &mut Model| {
            m.hidden = false;
            m.remove = true;
          }),
        self::parent().child((!m.moved).then(|| target.clone())),
        self::parent().child(m.moved.then_some(target)),
      )
    })
    .document(|mut d| {
      d.element.picking_mode = Prop::Set(PickingMode::Ignore);
      d
    });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("pointer/ui");
  let mut display = Display::connect(app, assets);
  let center = PanelPoint::new(350.0, 350.0);
  display.pointer_down(0, center);
  display.poll();
  assert_eq!(display.pointer_capture(0), Some(TARGET));
  display.pointer_move(0, PanelPoint::new(600.0, 350.0), true);
  display.poll();
  assert_eq!(display.pointer_capture(0), Some(TARGET));
  display.with_engine(|a| {
    assert!(a.model().moved);
    assert_eq!(a.model().gained, 1);
    assert_eq!(a.model().lost, 0);
  });
  display.pointer_cancel(0);
  assert_eq!(display.pointer_capture(0), None);
  display.with_engine(|a| {
    assert_eq!(a.model().canceled, 1);
    assert_eq!(a.model().lost, 1);
  });
  display.pointer_down(0, center);
  display.poll();
  assert_eq!(display.pointer_capture(0), Some(TARGET));
  display.pointer_move(0, PanelPoint::new(1100.0, 350.0), true);
  display.poll();
  assert_eq!(display.pointer_capture(0), None);
  assert_eq!(display.capture_losses(), &[(0, TARGET), (0, TARGET)]);
  display.pointer_up(0, center);
  display.click_at(PanelPoint::new(30.0, 20.0));
  display.poll();
  assert_eq!(display.pointer_capture(0), None);
  display.with_engine(|a| {
    assert_eq!(a.model().lost, 2);
    assert_eq!(a.model().clicks, 0);
  });
  display.pointer_down(0, center);
  display.poll();
  assert_eq!(display.pointer_capture(0), Some(TARGET));
  display.pointer_move(0, PanelPoint::new(1100.0, 350.0), true);
  display.poll();
  assert_eq!(display.pointer_capture(0), None);
  assert_eq!(
    display.capture_losses(),
    &[(0, TARGET), (0, TARGET), (0, TARGET)]
  );
  display.pointer_up(0, center);
  display.with_engine(|a| {
    assert!(a.model().gone);
    assert_eq!(a.model().lost, 2);
    assert_eq!(a.model().clicks, 0);
  });
}
