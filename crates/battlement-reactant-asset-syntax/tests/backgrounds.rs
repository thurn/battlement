use battlement_reactant_asset_syntax::{DependencyKind, DiagnosticCategory, parse};

#[test]
fn parses_color_and_local_image_backgrounds() {
  for background in [
    "rebeccapurple",
    "unity-url(\"Assets/Textures/panel.png\")",
    "unity-url(\"Assets/Textures/panel.png\") center / cover no-repeat content-box",
    "unity-url(\"Assets/Textures/panel.png\") right 1px bottom 2px no-repeat",
    "unity-url(\"Assets/Textures/panel.png\") 10vw 2em / calc(100% - 4px) 8vmin round border-box padding-box #1238",
  ] {
    parse(&format!(
      "@background PANEL {{ @canvas 20px 10px; background: {background}; box-shadow: 1px 2px red; }}"
    ))
    .unwrap_or_else(|error| panic!("{background}: {error:?}"));
  }
}

#[test]
fn preserves_layer_order_and_normalizes_colors() {
  let first = parse(
    "@background PANEL {
      @canvas 20px 10px;
      background: unity-url(\"Assets/a.png\") no-repeat, unity-url(\"Assets/b.png\") center #f00;
    }",
  )
  .unwrap();
  let reordered = parse(
    "@background PANEL {
      @canvas 20px 10px;
      background: unity-url(\"Assets/b.png\") center, unity-url(\"Assets/a.png\") no-repeat red;
    }",
  )
  .unwrap();
  let equivalent = parse(
    "@background OTHER {
      @canvas 20px 10px;
      background: unity-url(\"Assets/a.png\") no-repeat, unity-url(\"Assets/b.png\") center rgb(255, 0, 0);
    }",
  )
  .unwrap();

  assert_ne!(first.identity(), reordered.identity());
  assert_eq!(first.identity(), equivalent.identity());
}

#[test]
fn exposes_sorted_unique_dependency_references_without_file_access() {
  let request = parse(
    "@background PANEL {
      @canvas 20px 10px;
      background: unity-url(\"Assets/z.png\"), unity-url(\"Assets/a.PNG\") no-repeat, unity-url(\"Assets/z.png\") center;
    }",
  )
  .unwrap();

  assert_eq!(request.dependencies.len(), 2);
  assert_eq!(request.dependencies[0].kind, DependencyKind::Image);
  assert_eq!(request.dependencies[0].path, "Assets/a.PNG");
  assert_eq!(request.dependencies[1].path, "Assets/z.png");
}

#[test]
fn text_requests_expose_their_font_dependency() {
  let request = parse(
    "@text-image LABEL {
      @canvas 20px 10px;
      @font-file unity(\"Assets/Fonts/face.woff2\");
      content: \"Hi\";
      font-size: 8px;
      filter: brightness(1.1);
    }",
  )
  .unwrap();

  assert_eq!(request.dependencies.len(), 1);
  assert_eq!(request.dependencies[0].kind, DependencyKind::Font);
  assert_eq!(request.dependencies[0].path, "Assets/Fonts/face.woff2");
}

#[test]
fn rejects_unsupported_sources_paths_and_layer_forms() {
  for background in [
    "url(\"Assets/a.png\")",
    "unity-url(\"../a.png\")",
    "unity-url(\"/Assets/a.png\")",
    "unity-url(\"Assets/a.jpg\")",
    "unity-url(\"Assets/a.png?x=1\")",
    "paint(red)",
    "red, unity-url(\"Assets/a.png\")",
    "unity-url(\"Assets/a.png\") / cover",
    "unity-url(\"Assets/a.png\") center / cover / contain",
    "unity-url(\"Assets/a.png\") center red blue",
  ] {
    let error = parse(&format!(
      "@background PANEL {{ @canvas 20px 10px; background: {background}; }}"
    ))
    .unwrap_err();
    assert_eq!(
      error.category,
      DiagnosticCategory::InvalidValue,
      "{background}"
    );
    assert_eq!(error.property.as_deref(), Some("background"));
  }
}

#[test]
fn rejects_explicit_background_defaults() {
  for background in [
    "transparent",
    "unity-url(\"Assets/a.png\") repeat",
    "unity-url(\"Assets/a.png\") left top",
    "unity-url(\"Assets/a.png\") 0 0",
    "unity-url(\"Assets/a.png\") center / auto",
    "unity-url(\"Assets/a.png\") padding-box",
    "unity-url(\"Assets/a.png\") content-box border-box",
  ] {
    assert_eq!(
      parse(&format!(
        "@background PANEL {{ @canvas 20px 10px; background: {background}; }}"
      ))
      .unwrap_err()
      .category,
      DiagnosticCategory::RedundantDefault,
      "{background}"
    );
  }
}

#[test]
fn validates_font_dependency_paths_before_generation() {
  for path in [
    "../face.ttf",
    "/face.ttf",
    "Assets/face.ttc",
    "Assets//face.ttf",
  ] {
    let error = parse(&format!(
      "@text-image LABEL {{
        @canvas 20px 10px;
        @font-file unity(\"{path}\");
        content: \"Hi\";
        font-size: 8px;
      }}"
    ))
    .unwrap_err();
    assert_eq!(
      error.category,
      DiagnosticCategory::InvalidMetadata,
      "{path}"
    );
    assert_eq!(error.property.as_deref(), Some("@font-file"));
  }
}

#[test]
fn parses_every_gradient_family_and_typed_stop_form() {
  for background in [
    "linear-gradient(red, blue)",
    "linear-gradient(to top right, #f00 0% 10%, 25%, rgb(0, 0, 255) 50% 100%)",
    "repeating-linear-gradient(45deg, red 0, blue 2em)",
    "radial-gradient(circle at 25% 75%, red, blue)",
    "radial-gradient(circle 10px at left top, red 1vw, blue 10vmax)",
    "radial-gradient(ellipse 10px 25% at right bottom, red, blue)",
    "radial-gradient(circle at right 1px bottom 2px, red, blue)",
    "repeating-radial-gradient(closest-side at 2rem 3em, red, blue)",
    "conic-gradient(from 45deg at 30% 40%, red 0deg, blue 100%)",
    "repeating-conic-gradient(from calc(0.25turn) at left top, red, 25%, blue 180deg)",
  ] {
    parse(&format!(
      "@background PANEL {{ @canvas 20px 10px; background: {background}; }}"
    ))
    .unwrap_or_else(|error| panic!("{background}: {error:?}"));
  }
}

#[test]
fn gradient_identity_normalizes_stop_colors_and_numbers() {
  let first = parse(
    "@background PANEL { @canvas 20px 10px; background: linear-gradient(45deg, red 0%, blue 100%); }",
  )
  .unwrap();
  let equivalent = parse(
    "@background OTHER { @canvas 20px 10px; background: linear-gradient(45.0deg, #f00 0.0%, rgb(0, 0, 255) 1e2%); }",
  )
  .unwrap();
  let reordered = parse(
    "@background PANEL { @canvas 20px 10px; background: linear-gradient(45deg, blue 100%, red 0%); }",
  )
  .unwrap();

  assert_eq!(first.identity(), equivalent.identity());
  assert_ne!(first.identity(), reordered.identity());
}

#[test]
fn rejects_invalid_gradient_shapes_stops_and_preludes() {
  for background in [
    "linear-gradient(red)",
    "linear-gradient(45deg, red)",
    "linear-gradient(to left right, red, blue)",
    "linear-gradient(red, 25%, 50%, blue)",
    "linear-gradient(25%, red, blue)",
    "linear-gradient(red 10deg, blue)",
    "linear-gradient(red 0% 25% 50%, blue)",
    "linear-gradient(currentColor, blue)",
    "radial-gradient(circle 10px 20px, red, blue)",
    "radial-gradient(ellipse 10px, red, blue)",
    "radial-gradient(square, red, blue)",
    "radial-gradient(circle at left right, red, blue)",
    "conic-gradient(at center from 45deg, red, blue)",
    "conic-gradient(from 45px, red, blue)",
    "conic-gradient(red 10px, blue)",
    "conic-gradient(at top 10px, red, blue)",
  ] {
    assert_eq!(
      parse(&format!(
        "@background PANEL {{ @canvas 20px 10px; background: {background}; }}"
      ))
      .unwrap_err()
      .category,
      DiagnosticCategory::InvalidValue,
      "{background}"
    );
  }
}

#[test]
fn rejects_explicit_gradient_defaults() {
  for background in [
    "linear-gradient(to bottom, red, blue)",
    "linear-gradient(180deg, red, blue)",
    "radial-gradient(ellipse, red, blue)",
    "radial-gradient(farthest-corner, red, blue)",
    "radial-gradient(at center, red, blue)",
    "conic-gradient(from 0deg, red, blue)",
    "conic-gradient(from calc(0turn), red, blue)",
    "conic-gradient(at center center, red, blue)",
  ] {
    assert_eq!(
      parse(&format!(
        "@background PANEL {{ @canvas 20px 10px; background: {background}; }}"
      ))
      .unwrap_err()
      .category,
      DiagnosticCategory::RedundantDefault,
      "{background}"
    );
  }
}
