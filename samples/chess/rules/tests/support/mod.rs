#![allow(dead_code)]

use std::{
  cell::{Cell, RefCell},
  collections::BTreeMap,
  path::{Path, PathBuf},
  rc::Rc,
  time::Duration,
};

use battlement::{
  Connect, GameObjectKind, ObjectId, PhysicalKey, PointerButton, ScreenPosition, ScreenSize,
  Vector3,
};
use battlement_fake::{
  assets::{FakeAssetCatalog, FakePrefab},
  client::{FakeClient, PointerInput},
};
use battlement_rules::{
  Engine as PublicEngine, EngineDependencies, PersistenceBackend, contract, create_engine,
};

pub const DEFAULT: &[u8] = include_bytes!("../fixtures/default.json");
pub const CAPTURE: &[u8] = include_bytes!("../fixtures/capture.json");
pub const KNIGHT_CAPTURE: &[u8] = include_bytes!("../fixtures/knight-capture.json");
pub const CASTLE: &[u8] = include_bytes!("../fixtures/castle.json");
pub const EN_PASSANT: &[u8] = include_bytes!("../fixtures/en-passant.json");
pub const PROMOTION: &[u8] = include_bytes!("../fixtures/promotion.json");
pub const COMPUTER_WIN: &[u8] = include_bytes!("../fixtures/computer-win.json");
pub const AI_TURN: &[u8] = include_bytes!("../fixtures/ai-turn.json");
pub const CHECK: &[u8] = include_bytes!("../fixtures/check.json");
pub const PLAYER_WIN: &[u8] = include_bytes!("../fixtures/player-win.json");
pub const DRAW: &[u8] = include_bytes!("../fixtures/draw.json");

#[derive(Default)]
pub struct MemoryPersistence {
  values: RefCell<BTreeMap<PathBuf, Vec<u8>>>,
  fail_load: Cell<bool>,
  fail_store: Cell<bool>,
  fail_remove: Cell<bool>,
}

pub fn client(
  persistence: Rc<MemoryPersistence>,
  think_time: Duration,
) -> FakeClient<impl PublicEngine> {
  client_with_modules(persistence, think_time, &[])
}

pub fn client_with_modules(
  persistence: Rc<MemoryPersistence>,
  think_time: Duration,
  modules: &[&str],
) -> FakeClient<impl PublicEngine> {
  let mut connect =
    Connect::new("test", "test", ScreenSize::new(1_920, 1_080)).persistent_data_path("memory");
  connect.modules = modules.iter().map(|module| (*module).to_owned()).collect();
  let (mut client, _) = FakeClient::connect_with_clocked(
    move |clock| {
      create_engine(EngineDependencies {
        persistence,
        now: Rc::new(move || clock.now()),
        rng_seed: Some(43),
        think_time,
      })
    },
    catalog(),
    connect,
  );
  for _ in 0..8 {
    client.poll();
    std::thread::sleep(Duration::from_millis(1));
  }
  client
}

pub fn catalog() -> FakeAssetCatalog {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene(contract::CONTENT);
  for address in contract::PIECE_PREFABS {
    catalog.add_prefab(
      address,
      FakePrefab::new()
        .with_material_slots(1)
        .with_pointer_collider(),
    );
  }
  for address in contract::MUSIC_TRACKS {
    catalog.add_audio_clip(address);
  }
  for address in contract::SOUND_EFFECTS {
    catalog.add_audio_clip(address);
  }
  catalog.add_texture(contract::PLAY_BUTTON);
  catalog.add_texture(contract::REFRESH_BUTTON);
  catalog.add_material(contract::LEGAL_SQUARE);
  catalog.add_prefab(
    contract::PIECE_SELECTED_EFFECT,
    FakePrefab::new().with_particle_systems(),
  );
  catalog.add_particle_effect(contract::PIECE_SPAWN_EFFECT);
  catalog.add_particle_effect(contract::CAPTURE_EFFECT);
  catalog
}

pub fn wait_for_marker<E: PublicEngine>(client: &mut FakeClient<E>, marker: &str) {
  for _ in 0..2_000 {
    client.poll();
    if marker_active(client, marker) {
      return;
    }
    std::thread::sleep(Duration::from_millis(1));
  }
  panic!("Unity state did not reach marker {marker:?}");
}

pub fn marker_active<E: PublicEngine>(client: &FakeClient<E>, marker: &str) -> bool {
  let mut pending = vec![contract::ROOT_ID];
  while let Some(id) = pending.pop() {
    let Some(element) = client.ui_world().element(id) else {
      return false;
    };
    if element.name() == Some(marker) {
      return element.text().is_some_and(|text| !text.is_empty());
    }
    pending.extend(element.children());
  }
  false
}

pub fn find_ui<E: PublicEngine>(client: &FakeClient<E>, name: &str) -> Option<ObjectId> {
  let mut pending = vec![contract::ROOT_ID];
  while let Some(id) = pending.pop() {
    let element = client.ui_world().element(id)?;
    if element.name() == Some(name) {
      return Some(id);
    }
    pending.extend(element.children());
  }
  None
}

pub fn piece<E: PublicEngine>(client: &FakeClient<E>, file: char, rank: u8) -> ObjectId {
  piece_at(client, file, rank).unwrap_or_else(|| panic!("missing presented piece at {file}{rank}"))
}

pub fn piece_at<E: PublicEngine>(client: &FakeClient<E>, file: char, rank: u8) -> Option<ObjectId> {
  let expected = square(file, rank);
  client
    .world()
    .objects()
    .find(|object| {
      matches!(object.kind(), GameObjectKind::BoxHitRegion { .. })
        && client.world().world_point(object.id(), Vector3::ZERO) == expected
    })
    .map(|object| object.id())
}

pub fn highlight<E: PublicEngine>(client: &FakeClient<E>, file: char, rank: u8) -> ObjectId {
  let expected = square(file, rank);
  client
    .world()
    .objects()
    .find(|object| {
      object.active_self()
        && object.local_transform().position.x == expected.x
        && object.local_transform().position.z == expected.z
        && matches!(object.kind(), GameObjectKind::Plane { .. })
    })
    .unwrap_or_else(|| panic!("missing legal target at {file}{rank}"))
    .id()
}

pub fn move_piece<E: PublicEngine>(client: &mut FakeClient<E>, from: (char, u8), to: (char, u8)) {
  let source = piece(client, from.0, from.1);
  let input = drag_input(0);
  client.drag_start(source, input);
  wait_for_marker(client, contract::marker::SELECTED);
  client.drag_end(source, input, square(from.0, from.1));
  client.poll();
  client.poll();
  let target = highlight(client, to.0, to.1);
  client.activate(target);
}

pub fn drag_input(pointer_id: i32) -> PointerInput {
  PointerInput {
    pointer_id,
    screen_position: ScreenPosition::new(960.0, 540.0),
    world_hit: Vector3::ZERO,
    button: PointerButton::Left,
  }
}

pub fn square(file: char, rank: u8) -> Vector3 {
  Vector3::new((file as u8 - b'a') as f64 - 3.5, 0.0, rank as f64 - 4.5)
}

pub fn start_with_keyboard<E: PublicEngine>(client: &mut FakeClient<E>) {
  wait_for_marker(client, contract::marker::TITLE);
  client.key_down(PhysicalKey::Enter);
  client.key_up(PhysicalKey::Enter);
  wait_for_marker(client, contract::marker::INITIAL);
}

pub fn click_play<E: PublicEngine>(client: &mut FakeClient<E>) {
  let play = find_ui(client, "play-chess").expect("Play chess UI action");
  client.ui().click(play);
  wait_for_marker(client, contract::marker::INITIAL);
  for _ in 0..8 {
    client.poll();
    std::thread::sleep(Duration::from_millis(1));
  }
}

impl MemoryPersistence {
  pub fn with_save(bytes: &[u8]) -> Rc<Self> {
    let storage = Rc::new(Self::default());
    storage
      .values
      .borrow_mut()
      .insert(save_path(), bytes.to_vec());
    storage
  }

  pub fn empty() -> Rc<Self> {
    Rc::new(Self::default())
  }

  pub fn fail_load(&self) {
    self.fail_load.set(true);
  }

  pub fn fail_store(&self) {
    self.fail_store.set(true);
  }

  pub fn fail_remove(&self) {
    self.fail_remove.set(true);
  }
}

impl PersistenceBackend for MemoryPersistence {
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
    if self.fail_load.get() {
      return Err("injected load failure".to_owned());
    }
    Ok(self.values.borrow().get(path).cloned())
  }

  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
    if self.fail_store.get() {
      return Err("injected store failure".to_owned());
    }
    self
      .values
      .borrow_mut()
      .insert(path.to_owned(), bytes.to_vec());
    Ok(())
  }

  fn remove(&self, path: &Path) -> Result<(), String> {
    if self.fail_remove.get() {
      return Err("injected remove failure".to_owned());
    }
    self.values.borrow_mut().remove(path);
    Ok(())
  }
}

fn save_path() -> PathBuf {
  PathBuf::from("memory").join(contract::SAVE_FILE_NAME)
}
