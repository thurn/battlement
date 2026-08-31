use battlement_reactant_asset_syntax::{
  DiagnosticCategory, NativeSupport, classify_native_support, parse,
};

#[test]
fn classifies_every_native_support_row_with_reactant_replacements() {
  let cases = [
    (box_request("background: red"), "Style::background_color"),
    (
      box_request("background: unity-url(\"Assets/panel.png\")"),
      "Style::background_image or Image::source",
    ),
    (
      box_request(
        "background: unity-url(\"Assets/panel.png\") center / cover no-repeat content-box",
      ),
      "Style::background_image or Image::source",
    ),
    (
      box_request("border: 2px solid red; border-radius: 3px / 4px"),
      "Style::border_*",
    ),
    (box_request("opacity: 0.5"), "Style::opacity"),
    (
      box_request("transform: translate(1px) rotate(2deg) scale(1.2); transform-origin: left top"),
      "Style transform properties",
    ),
    (
      box_request(
        "filter: opacity(0.5) invert(1) grayscale(0.5) sepia(0.5) blur(1px) contrast(1.2) hue-rotate(2deg)",
      ),
      "Style::filter",
    ),
    (
      text_request("color: red; font-style: italic; letter-spacing: 1px; white-space: nowrap"),
      "text Style properties",
    ),
    (
      text_request("text-shadow: 1px 2px red"),
      "Style::text_shadow",
    ),
    (
      text_request("-webkit-text-stroke: 1px red"),
      "Style::unity_text_outline_*",
    ),
    (
      text_request("color: transparent; background: red; background-clip: text"),
      "text Style properties",
    ),
    (
      "@nine-slice FRAME { @canvas 20px 10px; @slices 1px 2px 1px 2px; }".to_owned(),
      "Style::unity_slice_* with a normal texture",
    ),
  ];

  for (source, replacement) in cases {
    let NativeSupport::NativeOnly { replacements } =
      classify_native_support(&source).unwrap_or_else(|error| panic!("{source}: {error:?}"))
    else {
      panic!("expected native-only classification: {source}");
    };
    assert!(
      replacements.contains(&replacement),
      "{source}: {replacements:?}"
    );

    let error = parse(&source).unwrap_err();
    assert_eq!(error.category, DiagnosticCategory::NativeOnly, "{source}");
    assert_eq!(error.symbol.as_deref(), source.split_whitespace().nth(1));
    assert!(
      error
        .replacement
        .as_deref()
        .is_some_and(|value| value.contains(replacement)),
      "{source}: {error:?}"
    );
  }
}

#[test]
fn classifies_every_generator_required_row() {
  let cases = [
    box_request("background: linear-gradient(red, blue)"),
    box_request("background: unity-url(\"Assets/a.png\"), unity-url(\"Assets/b.png\") red"),
    box_request("box-shadow: 1px 2px red"),
    box_request("border: 2px dashed red"),
    box_request("border-style: dotted"),
    box_request("border-bottom: 2px double red"),
    box_request("clip-path: circle(4px)"),
    box_request("clip-path: ellipse(4px 3px)"),
    box_request("clip-path: polygon(0 0, 10px 0, 0 10px)"),
    box_request("clip-path: path(\"M0 0L10 0L0 10Z\")"),
    box_request("clip-path: inset(1px round 2px)"),
    box_request("mask: linear-gradient(white, black) alpha"),
    box_request("background: linear-gradient(red, blue); background-blend-mode: multiply"),
    box_request("filter: brightness(1.1)"),
    box_request("filter: drop-shadow(1px 2px red)"),
    box_request("filter: saturate(1.1)"),
    box_request("transform: skew(2deg)"),
    box_request("transform: skewX(2deg)"),
    box_request("transform: matrix(1, 0, 0.1, 1, 0, 0)"),
    text_request(
      "color: transparent; background: linear-gradient(red, blue); background-clip: text",
    ),
    text_request("text-shadow: 1px 2px red, 2px 3px blue"),
    text_request(
      "color: transparent; background: unity-url(\"Assets/fill.png\"); background-clip: text; -webkit-text-stroke: 1px red",
    ),
  ];

  for source in cases {
    assert_eq!(
      classify_native_support(&source).unwrap_or_else(|error| panic!("{source}: {error:?}")),
      NativeSupport::GeneratorRequired,
      "{source}"
    );
    parse(&source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
  }
}

#[test]
fn one_generator_feature_admits_a_complete_native_composition() {
  let source = box_request(
    "background: red; border: 2px solid blue; opacity: 0.8; transform: rotate(2deg); filter: contrast(1.1); box-shadow: 1px 2px black",
  );

  assert_eq!(
    classify_native_support(&source).unwrap(),
    NativeSupport::GeneratorRequired
  );
  parse(&source).unwrap();
}

fn box_request(declarations: &str) -> String {
  format!("@background PANEL {{ @canvas 20px 10px; {declarations}; }}")
}

fn text_request(declarations: &str) -> String {
  format!(
    "@text-image LABEL {{ @canvas 100px 30px; @font-file unity(\"Assets/face.ttf\"); content: \"Hello\"; font-size: 12px; {declarations}; }}"
  )
}
