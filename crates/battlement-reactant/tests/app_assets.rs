mod app_support;

use battlement::{
  ActionId, BackgroundSource, BatchStart, CommandBody, IconSource, PreparedAsset, Response,
  ResponseMessage, TextureAddress, UiEventAction, UiFontAddress,
};
use battlement_fake::client::FakeClient;
use battlement_native::Engine;
use battlement_reactant::{app::App, prelude::*};

struct Browser;

impl Component for Browser {
  fn render(&self) -> impl Render {
    let (index, select) = use_state(0_usize);
    let app = use_app();
    let font = ["app/font-zero", "app/font-one", "app/font-two"][index];
    let paint = StyleTarget::new()
      .prepared_texture(format!("app/paint-{index}"))
      .mask("app/mask")
      .shader_material("app/shader");
    View::new()
      .animation(Animation::new(Keyframes::new([paint.clone(), paint])).duration_secs(1.0))
      .child((
        Button::new(trox::ls("Refresh"))
          .name("refresh")
          .on_click(move || app.refresh_snapshot()),
        Label::new(trox::ls(format!("Page {index}")))
          .name("page")
          .style(Style::new().unity_font_definition(UiFontAddress::new(font))),
        Button::new(trox::ls("Next"))
          .name("next")
          .on_click(move || select.update(|old| (old + 1) % 3))
          .icon(IconSource::Texture(TextureAddress::from_static(
            "app/base-icon",
          )))
          .icon_style(Style::new().background_image(BackgroundSource::Texture(
            TextureAddress::from_static("app/icon"),
          ))),
      ))
  }
}

#[test]
fn references_prepare_initial_and_later_assets_without_author_lists() {
  let app = App::new("app/content")
    .source_bundle(app_support::source_bundle())
    .ui(Browser);
  let root = app.root_document().root_id;
  let mut catalog = app_support::catalog();
  catalog.add_texture("app/icon");
  catalog.add_texture("app/base-icon");
  catalog.add_texture("app/mask");
  catalog.add_material("app/shader");
  for index in 0..3 {
    catalog.add_texture(format!("app/paint-{index}"));
  }
  for font in ["app/font-zero", "app/font-one", "app/font-two"] {
    catalog.add_ui_font(font);
  }
  let mut client = FakeClient::connect(app, catalog);
  assert!(
    client
      .world()
      .prepared_assets()
      .contains(&PreparedAsset::ui_font("app/font-zero"))
  );
  assert!(
    client
      .world()
      .prepared_assets()
      .contains(&PreparedAsset::texture("app/icon"))
  );
  assert!(
    !client
      .world()
      .prepared_assets()
      .contains(&PreparedAsset::ui_font("app/font-one"))
  );
  for asset in [
    PreparedAsset::texture("app/paint-0"),
    PreparedAsset::texture("app/mask"),
    PreparedAsset::material("app/shader"),
  ] {
    assert!(client.world().prepared_assets().contains(&asset));
  }
  let next = app_support::named(&mut client, root, "next");
  client.ui().click(next);
  assert_eq!(app_support::text(&mut client, root, "page"), "Page 1");
  assert!(
    client
      .world()
      .prepared_assets()
      .contains(&PreparedAsset::texture("app/paint-1"))
  );
  assert!(
    client
      .world()
      .prepared_assets()
      .contains(&PreparedAsset::ui_font("app/font-one"))
  );
  let refresh = app_support::named(&mut client, root, "refresh");
  client.ui().click(refresh);
  assert!(
    client
      .world()
      .prepared_assets()
      .contains(&PreparedAsset::ui_font("app/font-zero")),
    "refresh retains assets used earlier in this session"
  );
  client.ui().click(next);
  assert_eq!(app_support::text(&mut client, root, "page"), "Page 2");
  assert!(
    client
      .world()
      .prepared_assets()
      .contains(&PreparedAsset::ui_font("app/font-two"))
  );
}

#[test]
fn consecutive_responses_wait_for_preparation_and_keep_prior_dependencies() {
  let mut app = App::new("app/content")
    .source_bundle(app_support::source_bundle())
    .ui(Browser);
  let initial = app.connect(app_support::connect()).unwrap();
  let ResponseMessage::Snapshot(snapshot) = &initial.messages[0] else {
    panic!("snapshot");
  };
  let next = snapshot.ui[0].children[0].children[2].object_id;
  let first = ActionId::new_v4();
  let second = ActionId::new_v4();
  // Submit both actions before the client acknowledges either preparation.
  let first_response = app
    .submit_ui_event(UiEventAction::new(
      first,
      initial.session_id,
      app_support::click(next),
    ))
    .unwrap();
  let second_response = app
    .submit_ui_event(UiEventAction::new(
      second,
      initial.session_id,
      app_support::click(next),
    ))
    .unwrap();
  self::assert_preparation(
    &first_response.response,
    first,
    &["app/font-zero", "app/font-one"],
  );
  self::assert_preparation(
    &second_response.response,
    second,
    &["app/font-zero", "app/font-one", "app/font-two"],
  );
}

fn assert_preparation(response: &Response, action: ActionId, fonts: &[&str]) {
  let ResponseMessage::Batch(batch) = &response.messages[0] else {
    panic!("batch");
  };
  assert_eq!(batch.start, BatchStart::AfterEarlierAssetPreparation);
  assert_eq!(batch.caused_by_action_id, Some(action));
  assert_eq!(batch.groups[0].commands.len(), 1);
  let command = &batch.groups[0].commands[0];
  assert!(command.blocking);
  let CommandBody::AssetsReplaceSet(prepared) = &command.body else {
    panic!("preparation must precede dependent mutations");
  };
  for font in fonts {
    assert!(prepared.assets.contains(&PreparedAsset::ui_font(*font)));
  }
  assert!(
    prepared
      .assets
      .contains(&PreparedAsset::scene("app/content"))
  );
  assert!(response.messages.len() > 1);
}

#[test]
#[should_panic(expected = "generated asset reference is not owned by the linked registry")]
fn automatic_preparation_cannot_claim_unregistered_generated_addresses() {
  let mut app = App::new("app/content")
    .source_bundle(app_support::source_bundle())
    .ui(Image::new().source(battlement::ImageSource::Texture(
      TextureAddress::from_static("battlement-reactant/generated/unregistered.png"),
    )));
  let _ = app.connect(app_support::connect());
}
