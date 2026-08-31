use battlement_reactant_asset_syntax::{DependencyKind, DiagnosticCategory, parse};

#[test]
fn parses_color_and_local_image_backgrounds() {
  for background in [
    "rebeccapurple",
    "unity-url(\"Assets/Textures/panel.png\")",
    "unity-url(\"Assets/Textures/panel.png\") center / cover no-repeat content-box",
    "unity-url(\"Assets/Textures/panel.png\") 10vw 2em / calc(100% - 4px) 8vmin round border-box padding-box #1238",
  ] {
    parse(&format!(
      "@background PANEL {{ @canvas 20px 10px; background: {background}; }}"
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
