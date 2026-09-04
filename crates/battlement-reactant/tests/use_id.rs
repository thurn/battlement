mod app_support;

use std::{
  collections::HashSet,
  panic::{self, AssertUnwindSafe},
};

use battlement::{ObjectId, UiDocument};
use battlement_fake::client::FakeClient;
use battlement_reactant::{app::App, hooks, prelude::*};

struct IdOwner(&'static str);

struct IdList;

struct VariableId(u8);

impl Component for IdOwner {
  fn render(&self) -> impl Render {
    let id = use_id();
    let second = use_id();
    let (initial, _) = use_state_with(|| id.clone());
    let (count, set_count) = use_state(0);
    if count == 0 {
      set_count.set(1);
    }
    View::new().child((
      Label::new(trox::ls(id)).name(format!("id-{}", self.0)),
      Label::new(trox::ls(second)).name(format!("second-{}", self.0)),
      Label::new(trox::ls(initial)).name(format!("initial-{}", self.0)),
      Label::new(trox::ls(count.to_string())).name(format!("count-{}", self.0)),
      Button::new(trox::ls("Increment"))
        .name(format!("increment-{}", self.0))
        .on_click(move || set_count.update(|value| value + 1)),
    ))
  }
}

impl Component for IdList {
  fn render(&self) -> impl Render {
    let (reversed, set_reversed) = use_state(false);
    let (visible, set_visible) = use_state(true);
    let names = if reversed { ["b", "a"] } else { ["a", "b"] };
    View::new().child((
      Button::new(trox::ls("Reverse"))
        .name("reverse")
        .on_click(move || set_reversed.update(|value| !value)),
      Button::new(trox::ls("Toggle"))
        .name("toggle")
        .on_click(move || set_visible.update(|value| !value)),
      names
        .into_iter()
        .filter(|name| visible || *name != "a")
        .map(|name| IdOwner(name).key(name))
        .collect::<Vec<_>>(),
    ))
  }
}

impl Component for VariableId {
  fn render(&self) -> impl Render {
    match self.0 {
      0 => {
        let _ = hooks::use_id();
      }
      1 => {
        let _ = hooks::use_state(String::new());
      }
      2 => {}
      3 => {
        let _ = (hooks::use_id(), hooks::use_id());
      }
      _ => {
        let _ = hooks::use_memo(hooks::use_id, ());
      }
    }
    Button::new(trox::ls("Change hooks"))
      .name("change")
      .on_click(|mode: &mut u8| *mode += 1)
  }
}

#[test]
fn ids_survive_retries_updates_keyed_moves_and_reconnect_but_not_remount() {
  let app = App::new("app/content")
    .source_bundle(app_support::source_bundle())
    .ui(IdList);
  let root = app.root_document().root_id;
  let mut client = FakeClient::connect(app, app_support::catalog());
  let a = self::ids(&mut client, root, "a");
  let b = self::ids(&mut client, root, "b");
  assert_eq!(a[0], app_support::text(&mut client, root, "initial-a"));
  assert_eq!(app_support::text(&mut client, root, "count-a"), "1");
  assert_eq!(a.iter().chain(&b).collect::<HashSet<_>>().len(), 4);
  assert!(a.iter().chain(&b).all(|id| !id.is_empty()));

  self::click(&mut client, root, "increment-a");
  assert_eq!(app_support::text(&mut client, root, "count-a"), "2");
  assert_eq!(self::ids(&mut client, root, "a"), a);
  self::click(&mut client, root, "reverse");
  assert_eq!(self::ids(&mut client, root, "a"), a);
  assert_eq!(self::ids(&mut client, root, "b"), b);
  client.reconnect();
  assert_eq!(self::ids(&mut client, root, "a"), a);
  assert_eq!(self::ids(&mut client, root, "b"), b);

  self::click(&mut client, root, "toggle");
  self::click(&mut client, root, "toggle");
  let remounted = self::ids(&mut client, root, "a");
  assert!(remounted.iter().all(|id| !a.contains(id)));
  assert_eq!(self::ids(&mut client, root, "b"), b);
}

#[test]
fn roots_runtimes_and_reset_reconnects_allocate_distinct_ids() {
  let mut allocated = HashSet::new();
  for _ in 0..2 {
    let document = UiDocument::new(ObjectId::new_v4());
    let second_root = document.root_id;
    let app = App::new("app/content")
      .source_bundle(app_support::source_bundle())
      .ui(IdOwner("first"))
      .additional_root(document, |_| IdOwner("second"))
      .reset_on_reconnect();
    let root = app.root_document().root_id;
    let mut client = FakeClient::connect(app, app_support::catalog());
    for _ in 0..2 {
      for (root, name) in [(root, "first"), (second_root, "second")] {
        for id in self::ids(&mut client, root, name) {
          assert!(
            allocated.insert(id),
            "separate mounts must have distinct IDs"
          );
        }
      }
      client.reconnect();
    }
  }
}

#[test]
fn id_hooks_enforce_render_context_kind_and_count() {
  assert!(panic::catch_unwind(hooks::use_id).is_err());
  for next_mode in 1..=3 {
    let app = App::with_model("app/content", 0_u8)
      .source_bundle(app_support::source_bundle())
      .root(move |mode| VariableId(if *mode == 0 { 0 } else { next_mode }));
    let root = app.root_document().root_id;
    let mut client = FakeClient::connect(app, app_support::catalog());
    assert!(
      panic::catch_unwind(AssertUnwindSafe(|| {
        self::click(&mut client, root, "change");
      }))
      .is_err()
    );
  }
  assert!(
    panic::catch_unwind(|| {
      let app = App::new("app/content")
        .source_bundle(app_support::source_bundle())
        .ui(VariableId(4));
      let _ = FakeClient::connect(app, app_support::catalog());
    })
    .is_err()
  );
}

fn ids(client: &mut FakeClient<App>, root: ObjectId, name: &str) -> [String; 2] {
  [
    app_support::text(client, root, &format!("id-{name}")),
    app_support::text(client, root, &format!("second-{name}")),
  ]
}

fn click<G: 'static>(client: &mut FakeClient<App<G>>, root: ObjectId, name: &str) {
  let target = app_support::named(client, root, name);
  client.ui().click(target);
}
