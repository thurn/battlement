mod app_support;

use std::num::NonZeroU64;

use battlement::{
  CommandBody, DisplayId, ElementGeometry, GameObjectKind, GeometryGeneration,
  GeometryObservationBatch, GeometryObservationResult, GeometryObservationTarget,
  GeometryObservationValue, GeometryValue, ObjectId, Projective2, Rect, Style, ViewportRect,
};
use battlement_fake::client::FakeClient;
use battlement_reactant::{app::App, host::Label, scale_to_fit::ScaleToFit};

#[test]
fn scene_only_app_connects_and_reconnects_without_creating_a_ui_document() {
  let mut client = FakeClient::connect(
    App::new("app/content").source_bundle(app_support::source_bundle()),
    app_support::catalog(),
  );
  for _ in 0..2 {
    client.poll();
    assert_eq!(client.world().objects().count(), 1);
    assert!(
      client
        .world()
        .objects()
        .all(|object| matches!(object.kind(), GameObjectKind::Camera { .. }))
    );
    client.reconnect();
  }
}

#[test]
fn fitted_content_uses_the_area_inside_viewport_decoration_without_remounting() {
  let app = App::new("app/content")
    .source_bundle(app_support::source_bundle())
    .ui(
      ScaleToFit::new(100.0, 200.0)
        .viewport(|view| view.name("viewport"))
        .viewport_style(|style| style.padding(10).border_width(2))
        .canvas(|view| view.name("canvas"))
        .bounds_name("bounds")
        .child(Label::new(trox::assert_localized("Content")).name("content")),
    );
  let root = app.root_document().root_id;
  let mut client = FakeClient::connect(app, app_support::catalog());
  let bounds = app_support::named(&mut client, root, "bounds");
  let content = app_support::named(&mut client, root, "content");
  let viewport = app_support::named(&mut client, root, "viewport");
  let observation = client
    .commands()
    .iter()
    .find_map(|command| {
      if let CommandBody::GeometryObservationUpdate(update) = &command.command.body {
        update.added.first().cloned()
      } else {
        None
      }
    })
    .unwrap();
  let GeometryObservationTarget::UiElement { object_id } = observation.target else {
    panic!("fit must measure its content area");
  };
  assert_ne!(object_id, viewport);
  assert_eq!(client.ui().element(object_id).parent_id(), Some(viewport));
  assert_eq!(
    client.ui().element(object_id).style().padding_left,
    Style::new().padding_left
  );
  assert_eq!(
    client.ui().element(bounds).style().width,
    Style::new().width(0).width
  );
  for (index, (width, height, expected)) in [
    (76.0, 300.0, 76.0),
    (50.0, 300.0, 50.0),
    (500.0, 100.0, 50.0),
    (500.0, 500.0, 100.0),
    (0.0, 100.0, 0.0),
  ]
  .into_iter()
  .enumerate()
  {
    client.submit_geometry(GeometryObservationBatch {
      generation: GeometryGeneration(NonZeroU64::new(index as u64 + 1).unwrap()),
      changed: vec![GeometryObservationValue {
        observation_id: observation.observation_id,
        result: GeometryObservationResult::Current(GeometryValue::Element(ElementGeometry {
          layout: Rect::new(0.0, 0.0, width, height),
          viewport_bound: ViewportRect {
            x: 0.0,
            y: 0.0,
            width,
            height,
            display_id: DisplayId(0),
          },
          viewport_from_local: self::identity(),
          viewport_from_parent: self::identity(),
          panel_id: ObjectId::new_v4(),
        })),
      }],
    });
    client.poll();
    assert_eq!(
      client.ui().element(bounds).style().width,
      Style::new().width(expected).width
    );
    assert_eq!(
      client.ui().element(bounds).style().height,
      Style::new().height(expected * 2.0).height
    );
    assert_eq!(app_support::named(&mut client, root, "content"), content);
  }
}

fn identity() -> Projective2 {
  Projective2 {
    m11: 1.0,
    m12: 0.0,
    m13: 0.0,
    m21: 0.0,
    m22: 1.0,
    m23: 0.0,
    m31: 0.0,
    m32: 0.0,
    m33: 1.0,
  }
}
