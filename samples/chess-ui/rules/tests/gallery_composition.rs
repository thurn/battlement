use battlement::{GameObjectKind, ObjectId};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{
  app::App, asset_generator, component::Component, hooks, host::Label, render::Render,
};
use battlement_rules::{gallery::Gallery, review_button::ReviewButton, review_page::ReviewPage};

struct Counter {
  initial: u32,
}

impl Component for Counter {
  fn render(&self) -> impl Render {
    let (value, set_value) = hooks::use_state(self.initial);
    (
      Label::new(value.to_string()).name("counter"),
      ReviewButton::new("Increment")
        .name("increment")
        .on_press(move || set_value.update(|old| old + 1)),
    )
  }
}

#[test]
fn configured_component_values_keep_state_until_their_page_is_selected_again() {
  let gallery = Gallery::new()
    .page(ReviewPage::new("Counter").child(Counter { initial: 7 }))
    .page(ReviewPage::new("Greeting").child(Label::new("Welcome").name("greeting")));
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("gallery/content");
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  let mut client = FakeClient::connect(App::new("gallery/content").ui(gallery), assets);
  client.poll();

  let counter = self::named(&mut client, "counter");
  assert_eq!(client.ui().element(counter).text(), Some("7"));
  let increment = self::named(&mut client, "increment");
  for expected in ["8", "9"] {
    client.ui().click(increment);
    client.poll();
    assert_eq!(client.ui().element(counter).text(), Some(expected));
  }

  let first = self::named(&mut client, "review-page-1");
  client.ui().click(first);
  client.poll();
  assert!(!client.ui().contains(counter));
  let fresh = self::named(&mut client, "counter");
  assert_eq!(client.ui().element(fresh).text(), Some("7"));

  let second = self::named(&mut client, "review-page-2");
  client.ui().click(second);
  client.poll();
  let greeting = self::named(&mut client, "greeting");
  assert_eq!(client.ui().element(greeting).text(), Some("Welcome"));
  assert!(!client.ui().contains(fresh));

  client.ui().click(first);
  client.poll();
  let returned = self::named(&mut client, "counter");
  assert_eq!(client.ui().element(returned).text(), Some("7"));
  let heading = self::named(&mut client, "page-heading");
  assert_eq!(client.ui().focused(), Some(heading));
}

fn named(client: &mut FakeClient<App>, name: &str) -> ObjectId {
  let mut pending = client
    .world()
    .objects()
    .filter_map(|object| match object.kind() {
      GameObjectKind::UiDocument(document) => Some(document.root_id()),
      _ => None,
    })
    .collect::<Vec<_>>();
  let ui = client.ui();
  while let Some(id) = pending.pop() {
    let element = ui.element(id);
    if element.name() == Some(name) {
      return id;
    }
    pending.extend(element.children());
  }
  panic!("missing {name}");
}
