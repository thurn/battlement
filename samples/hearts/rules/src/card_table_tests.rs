use std::{collections::BTreeMap, collections::BTreeSet};

use battlement::{GameObjectKind, ObjectId, ParentScene, PreparedAsset};
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use reactant::{Application, prelude::*, world};
use reactant_testing::Display;

use crate::{
  assets, card_assets,
  card_table::CardTable,
  domain::Seat,
  layout_fixture,
  projection::{CardToken, HumanView, VisibleCard},
  scene,
};

#[derive(Clone, PartialEq)]
struct Input {
  view: HumanView,
  aspect: f64,
  inspection: Option<VisibleCard>,
}

#[derive(Clone)]
struct Probe(DisplayStore<Input>);

impl Component for Probe {
  fn render(&self) -> impl Render {
    let input = use_external_store(self.0.clone());
    world::SceneRoot::new(ParentScene::PrimaryScene)
      .child(CardTable::new(&input.view, input.aspect).inspect(input.inspection))
  }
}

#[test]
fn card_hosts_and_hits_survive_pass_reveal_trick_collection_and_reflow() {
  let views = layout_fixture::views();
  let initial = Input {
    view: views[0].clone(),
    aspect: 16.0 / 9.0,
    inspection: None,
  };
  let store = DisplayStore::new(initial.clone());
  let mut display = self::mount(store.clone());
  let tokens: Vec<_> = initial
    .view
    .hands
    .iter()
    .flatten()
    .map(|card| card.token)
    .collect();
  let stable: BTreeMap<_, _> = tokens
    .iter()
    .map(|&token| (token, self::stable_hosts(&mut display, token)))
    .collect();
  assert!(stable.values().all(|hosts| hosts.len() == 3));
  for view in &views {
    for aspect in [16.0 / 9.0, 9.0 / 16.0] {
      store.set(Input {
        view: view.clone(),
        aspect,
        inspection: None,
      });
      self::settle(&mut display);
      for &token in &tokens {
        assert_eq!(self::stable_hosts(&mut display, token), stable[&token]);
      }
      self::assert_private_visuals(&mut display, view);
      assert_eq!(
        display
          .world()
          .objects()
          .filter(|object| matches!(object.kind(), GameObjectKind::BoxHitRegion { .. }))
          .count(),
        52
      );
    }
  }
  assert_eq!(views[5].trick.len(), 4);
  assert_eq!(views[6].captured.iter().map(Vec::len).sum::<usize>(), 4);
}

#[test]
fn inspection_uses_new_hosts_without_replacing_the_original_card() {
  let view = layout_fixture::views().remove(0);
  let inspected = view.hands[Seat::South.index()][0];
  let initial = Input {
    view,
    aspect: 16.0 / 9.0,
    inspection: None,
  };
  let store = DisplayStore::new(initial.clone());
  let mut display = self::mount(store.clone());
  let original = self::stable_hosts(&mut display, inspected.token);
  let before: BTreeSet<_> = display
    .world()
    .objects()
    .map(|object| object.id())
    .collect();
  store.set(Input {
    inspection: Some(inspected),
    ..initial.clone()
  });
  self::settle(&mut display);
  assert_eq!(self::stable_hosts(&mut display, inspected.token), original);
  let added: Vec<_> = display
    .world()
    .objects()
    .filter(|object| !before.contains(&object.id()))
    .collect();
  assert_eq!(
    added
      .iter()
      .filter(|object| matches!(object.kind(), GameObjectKind::BoxHitRegion { .. }))
      .count(),
    1
  );
  assert!(added.iter().any(|object| matches!(object.kind(), GameObjectKind::Prefab { address, .. } if *address == card_assets::model(inspected.face.unwrap()))));
  store.set(initial);
  self::settle(&mut display);
  assert_eq!(
    display
      .world()
      .objects()
      .map(|object| object.id())
      .collect::<BTreeSet<_>>(),
    before
  );
}

fn stable_hosts(display: &mut Display, token: CardToken) -> Vec<ObjectId> {
  display
    .presentation(token.id())
    .expect("live card destination")
    .native_objects
    .into_iter()
    .filter(|id| {
      matches!(
        display
          .world()
          .object(*id)
          .expect("live native host")
          .kind(),
        GameObjectKind::Empty | GameObjectKind::BoxHitRegion { .. }
      )
    })
    .collect()
}

fn assert_private_visuals(display: &mut Display, view: &HumanView) {
  for card in view
    .hands
    .iter()
    .flatten()
    .chain(view.trick.iter().map(|(_, card)| card))
    .chain(view.captured.iter().flatten())
  {
    let hosts = display
      .presentation(card.token.id())
      .unwrap()
      .native_objects;
    let mut visuals = 0;
    for id in hosts {
      let object = display.world().object(id).unwrap();
      match object.kind() {
        GameObjectKind::Empty | GameObjectKind::BoxHitRegion { .. } => {}
        GameObjectKind::Image { image } => {
          assert!(card.face.is_none());
          assert_eq!(image.texture, assets::hearts::cards::BACK);
          assert!(object.material(0).is_none());
          visuals += 1;
        }
        GameObjectKind::Prefab { address, .. } => {
          assert_eq!(
            *address,
            card_assets::model(card.face.expect("hidden card must not carry a face prefab"))
          );
          visuals += 1;
        }
        kind => panic!("unexpected card payload {kind:?}"),
      }
    }
    assert_eq!(visuals, 1);
  }
}

fn settle(display: &mut Display) {
  display.flush();
  display.settle();
  display.flush();
}

fn mount(store: DisplayStore<Input>) -> Display {
  let mut catalog = FakeAssetCatalog::new();
  for asset in assets::ASSET_CATALOG {
    match asset {
      PreparedAsset::Scene(address) => catalog.add_scene(address.clone()),
      PreparedAsset::UiFont(address) => catalog.add_ui_font(address.clone()),
      PreparedAsset::Texture(address) => catalog.add_texture(address.clone()),
      PreparedAsset::Prefab(address) => catalog.add_prefab(address.clone(), FakePrefab::new()),
      PreparedAsset::Material(address) => catalog.add_material(address.clone()),
      _ => panic!("unexpected Hearts asset: {asset:?}"),
    }
  }
  let mut display = Display::mount(
    move || {
      Application::new(assets::hearts::CONTENT)
        .child(Probe(store.clone()))
        .camera(|camera| scene::camera().into_object(camera.object_id))
    },
    catalog,
  );
  self::settle(&mut display);
  display
}
