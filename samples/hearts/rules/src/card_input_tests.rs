use std::{cell::RefCell, rc::Rc, time::Duration};

use battlement::{Connect, ParentScene, PickingMode, PreparedAsset, Prop, ScreenSize, Vector3};
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use reactant::{GameStatus, app_context, prelude::*, world};
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
}

impl Component for Probe {
  fn render(&self) -> impl Render {
    let initial = self.initial.clone();
    let game = controller::use_hearts((), move || initial, Seat::South, false);
    let viewport = app_context::use_viewport_size();
    let input = card_input::use_card_input(game.clone(), viewport);
    *self.current.borrow_mut() = Some(game.clone());
    ContextProvider::new().context(input.clone()).child((
      CardControls,
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        CardTable::new(
          &game.view,
          f64::from(viewport.width) / f64::from(viewport.height),
        )
        .inspect(input.inspection()),
      ),
    ))
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
  let card = display.semantic_node("Two of Clubs").object_id;
  let original = display.world_point(card, Vector3::ZERO);
  let start = display.world_point(card, Vector3::new(-0.42, 0.55, 0.0));
  display.drag_world(start, Vector3::new(-4.0, 0.0, -1.0));
  display.flush();
  assert_eq!(self::current(&current).game.accepted().version.revision, 0);
  self::same_point(display.world_point(card, Vector3::ZERO), original);
  display.begin_drag_world(start, Vector3::new(0.0, 0.0, 0.0));
  display.flush();
  assert!(display.pointer_capture(0).is_some());
  display.cancel_drag();
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
  let current: Current = Rc::default();
  let probe = Probe {
    initial,
    current: current.clone(),
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
