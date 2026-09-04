#![allow(dead_code)]

use battlement::{
  ClickEvent, Command, Connect, KeyModifiers, ObjectId, PanelPoint, PointerButton, ScreenSize,
  UiEvent,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::Engine;

pub fn catalog() -> FakeAssetCatalog {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("app/content");
  assets
}

pub fn connect() -> Connect {
  Connect::new("test", "test", ScreenSize::new(800, 600))
}

pub fn click(id: ObjectId) -> UiEvent {
  UiEvent::click(
    id,
    ClickEvent::pointer(
      0,
      PanelPoint::default(),
      PointerButton::Left,
      1,
      KeyModifiers::default(),
    ),
  )
}

pub fn named<E: Engine<Command = Command>>(
  client: &mut FakeClient<E>,
  root: ObjectId,
  name: &str,
) -> ObjectId {
  let ui = client.ui();
  let mut pending = vec![root];
  while let Some(id) = pending.pop() {
    let element = ui.element(id);
    if element.name() == Some(name) {
      return id;
    }
    pending.extend(element.children());
  }
  panic!("missing element {name}");
}

pub fn text<E: Engine<Command = Command>>(
  client: &mut FakeClient<E>,
  root: ObjectId,
  name: &str,
) -> String {
  let id = self::named(client, root, name);
  client.ui().element(id).text().unwrap().to_owned()
}
