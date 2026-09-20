use std::{
  cell::{Cell, RefCell},
  rc::Rc,
  time::Duration,
};

use battlement::{ClickEvent, ObjectId, ParentScene, UiEvent};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{element_ref, hooks, host::ButtonHost, prelude::*, testing::App};
use reactant_testing::Display;
use trox::ls;

#[derive(Clone, Default)]
struct Probe {
  active: Rc<Cell<usize>>,
  renders: Rc<Cell<usize>>,
  reference: Rc<RefCell<Option<ElementRef>>>,
  setter: Rc<RefCell<Option<StateSetter<u32>>>>,
}
struct Card(Probe, bool);
impl Component for Card {
  fn render(&self) -> impl Render {
    self.0.renders.set(self.0.renders.get() + 1);
    let (count, setter) = hooks::use_state(0_u32);
    self.0.setter.replace(Some(setter.clone()));
    let reference = element_ref::use_element_ref();
    self.0.reference.replace(Some(reference.clone()));
    let active = self.0.active.clone();
    hooks::use_effect(
      move || {
        active.set(active.get() + 1);
        move || active.set(active.get() - 1)
      },
      (),
    );
    View::new()
      .name("visual")
      .semantic(
        SemanticProps::new(battlement::SemanticRole::Group).name(SemanticName::text(ls("Card"))),
      )
      .element_ref(reference)
      .animate(StyleTarget::new().opacity(if self.1 { 1.0 } else { 0.0 }))
      .transition(Transition::tween().duration_secs(0.2))
      .exit(
        MotionTarget::new(StyleTarget::new().opacity(0.0))
          .transition(Transition::tween().duration_secs(0.2)),
      )
      .child(
        ButtonHost::new(ls(format!("Count {count}")))
          .name("counter")
          .on_click(setter.update_callback(|count| count + 1)),
      )
  }
}

#[test]
fn committed_absence_ends_logical_lifetime_before_the_exit_visual() {
  let probe = Probe::default();
  let card_probe = probe.clone();
  let id = ObjectId::new_v4();
  let app = App::with_model("lifetime/scene", true).root(move |open| {
    (
      ButtonHost::new(ls("Destroy"))
        .name("destroy")
        .on_click(|open: &mut bool| *open = false),
      AnimatePresence::new()
        .child(open.then(|| Card(card_probe.clone(), true).id(*id.as_uuid()).key("card"))),
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("lifetime/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  let visual = display.find_ui(root, "visual");
  let counter = display.find_ui(root, "counter");
  let reference = probe.reference.borrow().as_ref().unwrap().clone();
  display.click_ui(counter);
  assert_eq!(display.ui_element(counter).text(), Some("Count 1"));
  let renders = probe.renders.get();
  display.click_ui(display.find_ui(root, "destroy"));
  display.poll();
  assert_eq!(
    probe.active.get(),
    0,
    "logical subscription survived destruction"
  );
  assert!(!reference.is_attached());
  assert!(
    display
      .with_engine(|app| app.presentation(*id.as_uuid()))
      .is_none()
  );
  assert_eq!(display.ui_element(visual).name(), Some("visual"));
  display.deliver_ui_event(UiEvent::click(counter, ClickEvent::NavigationSubmit));
  probe.setter.borrow().as_ref().unwrap().set(99);
  display.poll();
  assert_eq!(
    probe.renders.get(),
    renders,
    "terminal visual rerendered a component"
  );
  assert_eq!(display.ui_element(counter).text(), Some("Count 1"));
  assert_eq!(display.presentation_time(), Duration::ZERO);
  display.advance_time(Duration::from_millis(100));
  assert_eq!(display.ui_element(visual).name(), Some("visual"));
  assert_eq!(probe.active.get(), 0);
}

#[test]
fn hidden_children_keep_state_refs_and_native_handles_across_rapid_show() {
  let probe = Probe::default();
  let card_probe = probe.clone();
  let id = ObjectId::new_v4();
  let world = ObjectId::new_v4();
  let app = App::with_model("lifetime/scene", true).root(move |shown| {
    (
      ButtonHost::new(ls("Toggle"))
        .name("toggle")
        .on_click(|shown: &mut bool| *shown = !*shown),
      VisibilityScope::new(*shown).child((
        Card(card_probe.clone(), *shown).id(*id.as_uuid()),
        SceneRoot::new(ParentScene::PrimaryScene).child(WorldGroup::new().id(*world.as_uuid())),
      )),
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("lifetime/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  let visual = display.find_ui(root, "visual");
  let counter = display.find_ui(root, "counter");
  let reference = probe.reference.borrow().as_ref().unwrap().clone();
  assert!(
    display
      .accessibility()
      .nodes
      .iter()
      .any(|node| node.object_id == visual)
  );
  display.click_ui(counter);
  display.click_ui(display.find_ui(root, "toggle"));
  assert!(reference.is_attached());
  assert_eq!(probe.active.get(), 1);
  assert!(
    !display
      .accessibility()
      .nodes
      .iter()
      .any(|node| node.object_id == visual)
  );
  assert!(!display.object(world).unwrap().active_in_hierarchy());
  assert_eq!(
    display.ui_element(visual).style().display,
    Prop::Set(battlement::StyleValue::Value(battlement::Display::None))
  );
  display.deliver_ui_event(UiEvent::click(counter, ClickEvent::NavigationSubmit));
  assert_eq!(display.ui_element(counter).text(), Some("Count 1"));
  probe.setter.borrow().as_ref().unwrap().set(7);
  display.poll();
  assert_eq!(display.ui_element(counter).text(), Some("Count 7"));
  assert_eq!(
    display.ui_element(visual).style().display,
    Prop::Set(battlement::StyleValue::Value(battlement::Display::None))
  );
  display.advance_time(Duration::from_millis(50));
  display.click_ui(display.find_ui(root, "toggle"));
  assert!(
    display
      .accessibility()
      .nodes
      .iter()
      .any(|node| node.object_id == visual)
  );
  assert_eq!(display.find_ui(root, "visual"), visual);
  assert_eq!(display.find_ui(root, "counter"), counter);
  assert!(reference.is_attached());
  assert!(display.object(world).unwrap().active_in_hierarchy());
  assert_ne!(
    display.ui_element(visual).style().display,
    Prop::Set(battlement::StyleValue::Value(battlement::Display::None))
  );
  display.click_ui(counter);
  assert_eq!(display.ui_element(counter).text(), Some("Count 8"));
  assert_eq!(probe.active.get(), 1);
  assert_eq!(display.frame(), 0);
  assert_eq!(display.presentation_time(), Duration::from_millis(50));
}

struct HeldCard {
  probe: Probe,
  presence: Rc<RefCell<Option<Presence>>>,
  world: ObjectId,
}
impl Component for HeldCard {
  fn render(&self) -> impl Render {
    self.presence.replace(Some(hooks::use_presence()));
    let reference = element_ref::use_element_ref();
    self.probe.reference.replace(Some(reference.clone()));
    let active = self.probe.active.clone();
    hooks::use_effect(
      move || {
        active.set(active.get() + 1);
        move || active.set(active.get() - 1)
      },
      (),
    );
    (
      View::new().name("held").element_ref(reference),
      SceneRoot::new(ParentScene::PrimaryScene).child(WorldGroup::new().id(*self.world.as_uuid())),
    )
  }
}

#[test]
fn last_shared_visual_use_releases_native_resources_after_logical_cleanup() {
  let probe = Probe::default();
  let presence = Rc::new(RefCell::new(None));
  let card_probe = probe.clone();
  let card_presence = presence.clone();
  let world = ObjectId::new_v4();
  let app = App::with_model("lifetime/scene", true).root(move |open| {
    (
      ButtonHost::new(ls("Destroy"))
        .name("destroy")
        .on_click(|open: &mut bool| *open = false),
      AnimatePresence::new().child(open.then(|| {
        HeldCard {
          probe: card_probe.clone(),
          presence: card_presence.clone(),
          world,
        }
        .key("held")
      })),
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("lifetime/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  let visual = display.find_ui(root, "held");
  let reference = probe.reference.borrow().as_ref().unwrap().clone();
  let owner = presence.borrow().as_ref().unwrap().clone();
  let first = owner.retain_visual();
  let last = first.clone();
  display.click_ui(display.find_ui(root, "destroy"));
  display.poll();
  assert!(!reference.is_attached());
  assert!(!owner.is_present());
  assert_eq!(probe.active.get(), 0);
  assert!(first.native_objects().contains(&visual));
  assert!(first.native_objects().contains(&world));
  drop(first);
  display.click_ui(display.find_ui(root, "destroy"));
  display.poll();
  assert!(display.contains_ui(visual));
  assert!(display.object(world).is_some());
  drop(last);
  display.poll();
  assert!(!display.contains_ui(visual));
  assert!(display.object(world).is_none());
  owner.safe_to_remove();
  display.poll();
  assert_eq!(probe.active.get(), 0);
  assert!(!display.contains_ui(visual));
}

struct RejectedAbsence(bool);
impl Component for RejectedAbsence {
  fn render(&self) -> impl Render {
    if self.0 {
      Err(std::io::Error::other("reject absence"))
    } else {
      Ok(())
    }
  }
}

#[test]
fn rejected_absence_keeps_the_committed_component_mounted() {
  let probe = Probe::default();
  let card_probe = probe.clone();
  let app = App::with_model("lifetime/scene", true).root(move |open| {
    (
      ButtonHost::new(ls("Reject"))
        .name("reject")
        .on_click(|open: &mut bool| *open = false),
      AnimatePresence::new().child(open.then(|| Card(card_probe.clone(), true).key("card"))),
      RejectedAbsence(!*open),
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("lifetime/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  let visual = display.find_ui(root, "visual");
  let reference = probe.reference.borrow().as_ref().unwrap().clone();
  let reject = display.find_ui(root, "reject");
  assert!(
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| display.click_ui(reject))).is_err()
  );
  assert!(reference.is_attached());
  assert_eq!(probe.active.get(), 1);
  assert!(display.contains_ui(visual));
  let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(display)));
}

struct ResettableCard(Probe, Rc<RefCell<Option<Presence>>>, ObjectId);
impl Component for ResettableCard {
  fn render(&self) -> impl Render {
    let (open, set_open) = hooks::use_state(true);
    (
      ButtonHost::new(ls("Destroy"))
        .name("destroy")
        .on_click(move || set_open.set(false)),
      AnimatePresence::new().child(open.then(|| {
        HeldCard {
          probe: self.0.clone(),
          presence: self.1.clone(),
          world: self.2,
        }
        .key("held")
      })),
    )
  }
}

#[test]
fn old_exit_owners_and_input_cannot_affect_a_replacement_session() {
  let probe = Probe::default();
  let presence = Rc::new(RefCell::new(None));
  let app = App::new("lifetime/scene")
    .ui(ResettableCard(
      probe.clone(),
      presence.clone(),
      ObjectId::new_v4(),
    ))
    .reset_on_reconnect();
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("lifetime/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  let old_visual = display.find_ui(root, "held");
  let old_destroy = display.find_ui(root, "destroy");
  let old_ref = probe.reference.borrow().as_ref().unwrap().clone();
  let old_presence = presence.borrow().as_ref().unwrap().clone();
  let held = old_presence.retain_visual();
  display.click_ui(old_destroy);
  display.poll();
  assert_eq!(probe.active.get(), 0);
  assert!(display.contains_ui(old_visual));
  display.reconnect();
  display.poll();
  let replacement = display.find_ui(root, "held");
  assert_ne!(replacement, old_visual);
  assert!(!display.contains_ui(old_visual));
  assert!(!old_ref.is_attached());
  assert!(held.native_objects().is_empty());
  old_presence.safe_to_remove();
  drop(held);
  display.deliver_ui_event(UiEvent::click(old_destroy, ClickEvent::NavigationSubmit));
  display.poll();
  assert!(display.contains_ui(replacement));
  assert_eq!(probe.active.get(), 1);
}

struct NestedCard(Probe, Rc<RefCell<Option<Presence>>>, ObjectId, bool);
impl Component for NestedCard {
  fn render(&self) -> impl Render {
    View::new()
      .name("outer")
      .child(AnimatePresence::new().child(self.3.then(|| {
        HeldCard {
          probe: self.0.clone(),
          presence: self.1.clone(),
          world: self.2,
        }
        .key("held")
      })))
  }
}

#[test]
fn removing_an_outer_boundary_preserves_an_existing_shared_inner_exit() {
  let probe = Probe::default();
  let presence = Rc::new(RefCell::new(None));
  let card_probe = probe.clone();
  let card_presence = presence.clone();
  let world = ObjectId::new_v4();
  let app = App::with_model("lifetime/scene", 0_u32).root(move |stage| {
    (
      ButtonHost::new(ls("Next"))
        .name("next")
        .on_click(|stage: &mut u32| *stage += 1),
      AnimatePresence::new().child((*stage < 2).then(|| {
        NestedCard(
          card_probe.clone(),
          card_presence.clone(),
          world,
          *stage == 0,
        )
        .key("outer")
      })),
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("lifetime/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  let inner = display.find_ui(root, "held");
  let outer = display.find_ui(root, "outer");
  let held = presence.borrow().as_ref().unwrap().retain_visual();
  display.click_ui(display.find_ui(root, "next"));
  display.poll();
  assert_eq!(probe.active.get(), 0);
  display.click_ui(display.find_ui(root, "next"));
  display.poll();
  assert!(display.contains_ui(inner));
  assert!(display.contains_ui(outer));
  assert!(display.object(world).is_some());
  drop(held);
  display.poll();
  assert!(!display.contains_ui(inner));
  assert!(!display.contains_ui(outer));
  assert!(display.object(world).is_none());
}

#[test]
fn a_live_identified_child_moves_out_before_its_old_parent_exits() {
  let probe = Probe::default();
  let card_probe = probe.clone();
  let card = ObjectId::new_v4();
  let app = App::with_model("lifetime/scene", false).root(move |moved| {
    (
      ButtonHost::new(ls("Move"))
        .name("move")
        .on_click(|moved: &mut bool| *moved = true),
      AnimatePresence::new().child((!*moved).then(|| {
        View::new()
          .name("old-parent")
          .animate(StyleTarget::new().opacity(1.0))
          .exit(
            MotionTarget::new(StyleTarget::new().opacity(0.0))
              .transition(Transition::tween().duration_secs(0.2)),
          )
          .child(Card(card_probe.clone(), true).id(*card.as_uuid()))
          .key("parent")
      })),
      View::new()
        .name("destination")
        .child(moved.then(|| Card(card_probe.clone(), true).id(*card.as_uuid()))),
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("lifetime/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  let visual = display.find_ui(root, "visual");
  let counter = display.find_ui(root, "counter");
  let old_parent = display.find_ui(root, "old-parent");
  let reference = probe.reference.borrow().as_ref().unwrap().clone();
  display.click_ui(counter);
  display.click_ui(display.find_ui(root, "move"));
  display.poll();
  assert!(reference.is_attached());
  assert_eq!(probe.active.get(), 1);
  assert!(display.contains_ui(old_parent));
  assert_eq!(
    display.find_ui(display.find_ui(root, "destination"), "visual"),
    visual
  );
  display.click_ui(counter);
  assert_eq!(display.ui_element(counter).text(), Some("Count 2"));
}
