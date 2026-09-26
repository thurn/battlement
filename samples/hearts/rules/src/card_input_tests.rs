use std::{cell::RefCell, rc::Rc, time::Duration};

use battlement::{
  Connect, ControllerButton, ControllerDirection, ControllerInputSettings, PanelPoint, ParentScene,
  PhysicalKey, PickingMode, PreparedAsset, Prop, ScreenSize, Vector3,
};
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use reactant::{GameStatus, app_context, hooks, overlay::OverlayHost, prelude::*, world};
use reactant_testing::Display;

use crate::{
  HeartsController, HeartsReducer, assets,
  card_controls::CardControls,
  card_input,
  card_table::CardTable,
  controller,
  domain::{HeartsState, Phase, Seat},
  layout_fixture, scene,
};

const TIMEOUT: Duration = Duration::from_secs(10);
type Game = reactant_rules::ReducerGame<HeartsReducer>;
type Current = Rc<RefCell<Option<HeartsController>>>;

#[derive(Clone)]
struct Probe {
  initial: HeartsState,
  current: Current,
  aspect: DisplayStore<f64>,
}

impl Component for Probe {
  fn render(&self) -> impl Render {
    let overlay = reactant::use_portal_target();
    let initial = self.initial.clone();
    let game = controller::use_hearts((), move || initial, Seat::South, false);
    let viewport = app_context::use_viewport_size();
    let aspect = hooks::use_external_store(self.aspect.clone());
    let input = card_input::use_card_input(game.clone(), viewport);
    *self.current.borrow_mut() = Some(game.clone());
    ContextProvider::new().context(input.clone()).child(
      Stack::new().picking_mode(PickingMode::Ignore).child((
        View::new().picking_mode(PickingMode::Ignore).child((
          CardControls(overlay.clone()),
          world::SceneRoot::new(ParentScene::PrimaryScene)
            .child(CardTable::new(&game.view, aspect).inspect(input.inspection())),
        )),
        OverlayHost::new(overlay),
      )),
    )
  }
}

#[test]
fn passing_requires_three_and_duplicate_confirmation_exchanges_once() {
  let (mut display, current) = self::mount(HeartsState::new(43));
  self::ready(&mut display, &current);
  let cards = self::current(&current).view.hands[0].clone();
  assert!(display.semantic_node("Pass three cards").state.disabled);
  for card in &cards[..2] {
    display.click_button(&card_input::name(card.face.unwrap()));
    display.flush();
  }
  assert!(display.semantic_node("Pass three cards").state.disabled);
  display.click_button(&card_input::name(cards[2].face.unwrap()));
  display.flush();
  assert!(!display.semantic_node("Pass three cards").state.disabled);
  display.click_button(&card_input::name(cards[3].face.unwrap()));
  display.flush();
  assert!(
    display
      .accessibility()
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Choose three to pass · 3 / 3"))
  );
  let confirm = display.button_event("Pass three cards");
  display.deliver_ui_event(confirm.clone());
  display.deliver_ui_event(confirm);
  self::ready(&mut display, &current);
  let game = self::current(&current);
  assert_eq!(game.game.accepted().version.revision, 4);
  assert_eq!(
    game.game.accepted().state,
    Rc::new(layout_fixture::playing_state())
  );
  assert!(game.view.pending_pass.is_none());
  assert_eq!(game.view.hands[0].len(), 13);
}

#[test]
fn illegal_cards_are_inspectable_and_selection_then_confirm_admits_one_play() {
  let (mut display, current) = self::mount(layout_fixture::playing_state());
  self::ready(&mut display, &current);
  display.click_button("Two of Spades");
  display.flush();
  assert!(display.semantic_node("Play selected card").state.disabled);
  assert!(
    display
      .accessibility()
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Open the first trick with the two of clubs."))
  );
  let before = display.world().objects().count();
  display.click_button("Inspect selected card");
  display.flush();
  assert!(display.world().objects().count() > before);
  display.click_button("Two of Clubs");
  display.flush();
  assert_eq!(self::current(&current).game.accepted().version.revision, 0);
  display.click_button("Close inspection");
  display.flush();
  assert_eq!(display.world().objects().count(), before);
  display.click_button("Two of Clubs");
  display.flush();
  assert_eq!(self::current(&current).game.accepted().version.revision, 0);
  assert!(!display.semantic_node("Play selected card").state.disabled);
  let stale = display.button_event("Two of Clubs");
  display.deliver_ui_event(stale.clone());
  display.deliver_ui_event(stale.clone());
  self::ready(&mut display, &current);
  assert_eq!(self::current(&current).view.hands[0].len(), 12);
  let revision = self::current(&current).game.accepted().version.revision;
  display.deliver_ui_event(stale);
  display.flush();
  assert_eq!(
    self::current(&current).game.accepted().version.revision,
    revision
  );
}

#[test]
fn invalid_drop_and_capture_loss_restore_the_card_and_valid_drop_plays_once() {
  let (mut display, current) = self::mount(layout_fixture::playing_state());
  self::ready(&mut display, &current);
  self::key(&mut display, PhysicalKey::ArrowRight);
  let card = display.semantic_node("Two of Clubs").object_id;
  let original = display.world_point(card, Vector3::ZERO);
  let start = display.world_point(card, Vector3::new(-0.42, 0.55, 0.0));
  display.drag_world(start, Vector3::new(-4.0, 0.0, -1.0));
  display.flush();
  assert_eq!(self::current(&current).game.accepted().version.revision, 0);
  display.pointer_move(0, PanelPoint::new(0.0, 0.0), false);
  display.flush();
  self::same_point(display.world_point(card, Vector3::ZERO), original);
  display.begin_drag_world(start, Vector3::new(0.0, 0.0, 0.0));
  display.flush();
  assert!(display.pointer_capture(0).is_some());
  display.cancel_drag();
  display.flush();
  display.pointer_move(0, PanelPoint::new(0.0, 0.0), false);
  display.flush();
  self::same_point(display.world_point(card, Vector3::ZERO), original);
  assert_eq!(self::current(&current).game.accepted().version.revision, 0);
  display.drag_world(start, Vector3::new(0.0, 0.0, 0.0));
  self::ready(&mut display, &current);
  assert_eq!(self::current(&current).view.hands[0].len(), 12);
  assert_eq!(
    self::current(&current)
      .game
      .accepted()
      .state
      .public_table()
      .history
      .iter()
      .filter(|played| played.seat == Seat::South)
      .count(),
    1
  );
}

#[test]
fn keyboard_and_controller_complete_pass_and_trick_without_pointer() {
  let (mut display, current) = self::mount(HeartsState::new(43));
  self::ready(&mut display, &current);
  self::key(&mut display, PhysicalKey::ArrowRight);
  assert_eq!(
    display.focused(),
    Some(display.semantic_node("Seven of Clubs").object_id)
  );
  self::key(&mut display, PhysicalKey::Enter);
  display.controller_navigate(0, ControllerDirection::Right);
  self::primary(&mut display);
  display.controller_navigate(0, ControllerDirection::Right);
  self::primary(&mut display);
  self::tab_to(&mut display, "Pass three cards");
  self::primary(&mut display);
  self::ready(&mut display, &current);
  assert_eq!(self::current(&current).game.accepted().version.revision, 4);
  self::tab_to(&mut display, "Two of Clubs");
  self::key(&mut display, PhysicalKey::Enter);
  assert!(!display.semantic_node("Play selected card").state.disabled);
  self::key(&mut display, PhysicalKey::Enter);
  self::ready(&mut display, &current);
  assert_eq!(self::current(&current).view.hands[0].len(), 12);
  assert!(
    self::current(&current)
      .game
      .accepted()
      .state
      .public_table()
      .history
      .len()
      >= 4
  );
  assert!(display.focused().is_some());
}

#[test]
fn inspection_traps_navigation_and_restores_invoker_without_clearing_selection() {
  let (mut display, current) = self::mount(layout_fixture::playing_state());
  self::ready(&mut display, &current);
  self::tab_to(&mut display, "Two of Spades");
  self::primary(&mut display);
  self::tab_to(&mut display, "Inspect selected card");
  let invoker = display.focused();
  self::primary(&mut display);
  let close = display.semantic_node("Close inspection").object_id;
  assert_eq!(display.focused(), Some(close));
  display.controller_navigate(0, ControllerDirection::Right);
  assert_eq!(display.focused(), Some(close));
  display.controller_button_down(0, ControllerButton::East);
  display.controller_button_up(0, ControllerButton::East);
  display.flush();
  assert_eq!(display.focused(), invoker);
  assert_eq!(self::current(&current).game.accepted().version.revision, 0);
  self::primary(&mut display);
  assert_eq!(
    display.semantic_node("Card inspection").role,
    battlement::SemanticRole::Dialog
  );
  self::key(&mut display, PhysicalKey::Escape);
  self::key(&mut display, PhysicalKey::Escape);
  assert!(
    display
      .semantic_node("Inspect selected card")
      .state
      .disabled
  );
}

#[test]
fn focused_owned_card_survives_reflow_without_exposing_opponent_controls() {
  let aspect = DisplayStore::new(16.0 / 9.0);
  let (mut display, current) =
    self::mount_with_aspect(layout_fixture::playing_state(), aspect.clone());
  self::ready(&mut display, &current);
  self::key(&mut display, PhysicalKey::ArrowRight);
  let focused = display.focused().unwrap();
  self::key(&mut display, PhysicalKey::Enter);
  aspect.set(9.0 / 16.0);
  display.flush();
  display.settle();
  assert_eq!(display.focused(), Some(focused));
  let owned: Vec<_> = self::current(&current).view.hands[0]
    .iter()
    .map(|card| {
      display
        .semantic_node(&card_input::name(card.face.unwrap()))
        .object_id
    })
    .collect();
  for _ in 0..20 {
    self::key(&mut display, PhysicalKey::Tab);
    let id = display.focused().expect("focus survives reflow");
    if display.object(id).is_some() {
      assert!(owned.contains(&id));
    }
  }
  self::tab_to(&mut display, "Two of Clubs");
  self::key(&mut display, PhysicalKey::Enter);
  self::ready(&mut display, &current);
  assert_eq!(self::current(&current).view.hands[0].len(), 12);
}

fn key(display: &mut Display, key: PhysicalKey) {
  display.key_down(key);
  display.key_up(key);
  display.flush();
}

fn primary(display: &mut Display) {
  display.controller_button_down(0, ControllerButton::South);
  display.controller_button_up(0, ControllerButton::South);
  display.flush();
}

fn tab_to(display: &mut Display, name: &str) {
  let target = display.semantic_node(name).object_id;
  for _ in 0..24 {
    if display.focused() == Some(target) {
      return;
    }
    self::key(display, PhysicalKey::Tab);
  }
  panic!(
    "could not navigate to {name}; focus {:?}",
    display.focused()
  );
}

fn same_point(a: Vector3, b: Vector3) {
  assert!(
    (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs() < 0.0001,
    "{a:?} != {b:?}"
  );
}
fn current(current: &Current) -> HeartsController {
  current.borrow().as_ref().unwrap().clone()
}

fn ready(display: &mut Display, current: &Current) {
  for _ in 0..40 {
    display.flush();
    display.settle();
    display.flush();
    let game = self::current(current);
    if game.game.status() == GameStatus::Busy {
      assert!(display.wait_for_game_output::<Game>(TIMEOUT));
      display.poll();
      continue;
    }
    match game.view.table.phase {
      Phase::Passing if game.game.accepted().version.revision < 3 => {}
      Phase::Playing { turn } if turn != Seat::South => {}
      _ => return,
    }
    assert!(reactant::testing::wait_for_task(&game.computer, TIMEOUT));
  }
  panic!("input did not reach a human decision");
}

fn mount(initial: HeartsState) -> (Display, Current) {
  self::mount_with_aspect(initial, DisplayStore::new(16.0 / 9.0))
}

fn mount_with_aspect(initial: HeartsState, aspect: DisplayStore<f64>) -> (Display, Current) {
  let current: Current = Rc::default();
  let probe = Probe {
    initial,
    current: current.clone(),
    aspect,
  };
  let mut assets = FakeAssetCatalog::new();
  for asset in assets::ASSET_CATALOG {
    match asset {
      PreparedAsset::Scene(a) => assets.add_scene(a.clone()),
      PreparedAsset::UiFont(a) => assets.add_ui_font(a.clone()),
      PreparedAsset::Texture(a) => assets.add_texture(a.clone()),
      PreparedAsset::Prefab(a) => assets.add_prefab(a.clone(), FakePrefab::new()),
      PreparedAsset::Material(a) => assets.add_material(a.clone()),
      _ => panic!("unexpected asset"),
    }
  }
  let display = Display::mount_with(
    move || {
      reactant::Application::new(crate::assets::hearts::CONTENT)
        .global_keys([
          PhysicalKey::ArrowRight,
          PhysicalKey::ArrowLeft,
          PhysicalKey::ArrowUp,
          PhysicalKey::ArrowDown,
          PhysicalKey::Enter,
          PhysicalKey::Escape,
          PhysicalKey::Tab,
        ])
        .controller_input(
          ControllerInputSettings::new().buttons([ControllerButton::South, ControllerButton::East]),
        )
        .child(probe.clone())
        .document(|mut doc| {
          doc.element.picking_mode = Prop::Set(PickingMode::Ignore);
          doc
        })
        .camera(|camera| scene::camera().into_object(camera.object_id))
    },
    assets,
    Connect::new("test", "test", ScreenSize::new(1280, 720)),
  );
  (display, current)
}
