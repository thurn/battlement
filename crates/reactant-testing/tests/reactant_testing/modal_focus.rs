use battlement::{NavigationDirection, ObjectId, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  element_ref,
  host::ButtonHost,
  overlay::{Overlay, OverlayHost},
  portal::PortalTarget,
  prelude::*,
  testing::App,
};
use reactant_testing::Display;
use trox::ls;

const OPEN: ObjectId = object_id!("6fc40000-0000-4000-8000-000000000001");
const RETURN: ObjectId = object_id!("6fc40000-0000-4000-8000-000000000002");
const OUTER: ObjectId = object_id!("6fc40000-0000-4000-8000-000000000003");
const INNER: ObjectId = object_id!("6fc40000-0000-4000-8000-000000000004");
const CLOSE: ObjectId = object_id!("6fc40000-0000-4000-8000-000000000005");

#[derive(Clone, Copy, Default)]
enum ReturnTarget {
  #[default]
  Explicit,
  Invoker,
  NonSequential,
  Disabled,
  Removed,
}

#[derive(Clone, Default)]
struct Model {
  outer: bool,
  inner: bool,
  unmount: bool,
  removed: bool,
  target: ReturnTarget,
}

struct Modals(Model, PortalTarget);

impl Component for Modals {
  fn render(&self) -> impl Render {
    let return_to = element_ref::use_element_ref();
    let initial = element_ref::use_element_ref();
    let close = element_ref::use_element_ref();
    let mut outer = Overlay::modal(self.1.clone(), ls("Outer"))
      .initial_focus(initial.clone())
      .on_dismiss(|m: &mut Model| {
        m.outer = false;
        m.removed = matches!(m.target, ReturnTarget::Removed);
      });
    if !matches!(self.0.target, ReturnTarget::Invoker) {
      outer = outer.restore_focus(return_to.clone());
    }
    Stack::new().child((
      ButtonHost::new(ls("Open"))
        .id(*OPEN.as_uuid())
        .on_click(|m: &mut Model| m.outer = true),
      (!self.0.removed).then(|| {
        ButtonHost::new(ls("Return"))
          .id(*RETURN.as_uuid())
          .element_ref(return_to.clone())
          .tab_index(if matches!(self.0.target, ReturnTarget::NonSequential) {
            -1
          } else {
            0
          })
          .enabled(!matches!(self.0.target, ReturnTarget::Disabled))
      }),
      self.0.outer.then(|| {
        outer.child((
          ButtonHost::new(ls("Close outer"))
            .id(*OUTER.as_uuid())
            .element_ref(initial)
            .on_click(|m: &mut Model| m.outer = false),
          ButtonHost::new(ls("Open inner"))
            .id(*INNER.as_uuid())
            .on_click(|m: &mut Model| m.inner = true),
          self.0.inner.then(|| {
            Overlay::modal(self.1.clone(), ls("Inner"))
              .initial_focus(close.clone())
              .restore_focus(return_to.clone())
              .on_dismiss(|m: &mut Model| {
                m.inner = false;
                if m.unmount {
                  m.outer = false;
                }
              })
              .child(
                ButtonHost::new(ls("Close inner"))
                  .id(*CLOSE.as_uuid())
                  .element_ref(close),
              )
          }),
        ))
      }),
      OverlayHost::new(self.1.clone()),
    ))
  }
}

fn fixture(target: ReturnTarget, unmount: bool) -> Display<App<Model>> {
  let mut app = App::with_model(
    "modal/focus",
    Model {
      target,
      unmount,
      ..Model::default()
    },
  );
  let portal = app.create_portal_target();
  app = app.root(move |model| Modals(model.clone(), portal.clone()));
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("modal/focus");
  Display::connect(app, assets)
}

#[test]
fn nested_modals_restore_their_last_focus_before_the_application_target() {
  for target in [
    ReturnTarget::Explicit,
    ReturnTarget::Invoker,
    ReturnTarget::NonSequential,
    ReturnTarget::Disabled,
    ReturnTarget::Removed,
  ] {
    let mut display = fixture(target, false);
    display.navigate(NavigationDirection::Next);
    assert_eq!(display.focused(), Some(OPEN));
    display.activate_focused();
    assert_eq!(display.focused(), Some(OUTER));
    display.navigate(NavigationDirection::Next);
    assert_eq!(display.focused(), Some(INNER));
    display.activate_focused();
    assert_eq!(display.focused(), Some(CLOSE));
    display.cancel_navigation();
    assert_eq!(display.focused(), Some(INNER));
    display.cancel_navigation();
    assert_eq!(
      display.focused(),
      Some(
        if matches!(target, ReturnTarget::Explicit | ReturnTarget::NonSequential) {
          RETURN
        } else {
          OPEN
        }
      )
    );
    assert_eq!(display.frame(), 0);
  }
}

#[test]
fn unmounting_the_modal_tree_restores_an_eligible_application_target() {
  let mut display = fixture(ReturnTarget::Explicit, true);
  display.navigate(NavigationDirection::Next);
  display.activate_focused();
  display.navigate(NavigationDirection::Next);
  display.activate_focused();
  display.cancel_navigation();
  assert_eq!(display.focused(), Some(RETURN));
  assert!(!display.ui().contains(CLOSE));
  assert!(!display.ui().contains(OUTER));
}

#[test]
fn explicit_restore_applies_without_keyboard_navigation() {
  let mut display = fixture(ReturnTarget::Explicit, false);
  assert_eq!(display.focused(), None);
  display.click_ui(OPEN);
  assert_eq!(display.focused(), Some(OUTER));
  display.click_ui(OUTER);
  assert_eq!(display.focused(), Some(RETURN));
}
