use battlement_reactant_asset_syntax::{
  ClipEdge, Compression, DeclarationKind, Diagnostic, DiagnosticCategory, FilterMode, Insets,
  LogicalRect, LogicalSize, WrapMode, parse,
};

#[test]
fn applies_background_defaults_independent_of_statement_order() {
  let first =
    parse("@background PANEL { @canvas 32px 16px; border-radius: 3px; background: red; mask: linear-gradient(red, blue) alpha; }").unwrap();
  let reordered =
    parse("@background PANEL { background: red; @canvas 32px 16px; mask: linear-gradient(red, blue) alpha; border-radius: 3px; }").unwrap();

  assert_eq!(first.metadata, reordered.metadata);
  assert_eq!(first.kind, reordered.kind);
  assert_eq!(
    first
      .paint
      .iter()
      .map(|value| (&value.property, &value.value))
      .collect::<Vec<_>>(),
    reordered
      .paint
      .iter()
      .map(|value| (&value.property, &value.value))
      .collect::<Vec<_>>()
  );
  assert_eq!(first.kind, DeclarationKind::Background);
  assert_eq!(
    first.metadata.canvas,
    LogicalSize {
      width: 32.0,
      height: 16.0
    }
  );
  assert_eq!(
    first.metadata.subject,
    LogicalRect {
      x: 0.0,
      y: 0.0,
      width: 32.0,
      height: 16.0
    }
  );
  assert_eq!(first.metadata.slices, None);
  assert!(first.metadata.allowed_clipping.is_empty());
  assert_eq!(first.metadata.raster_scale, 2);
  assert_eq!(first.metadata.filter_mode, FilterMode::Bilinear);
  assert_eq!(first.metadata.wrap_mode, WrapMode::Clamp);
  assert_eq!(first.metadata.compression, Compression::Lossless);
  assert_eq!(first.metadata.font_file, None);
  assert_eq!(
    first
      .paint
      .iter()
      .map(|value| value.property.as_str())
      .collect::<Vec<_>>(),
    ["background", "border-radius", "mask"]
  );
}

#[test]
fn parses_nine_slice_metadata_and_every_nondefault_option() {
  let request = parse(
    "@nine-slice FRAME {
      @compression lossy-high;
      @allow-clipping top bottom left;
      @subject 1px 2px 18px 15px;
      @wrap-mode repeat;
      @slices 1px 2px 3px 4px;
      @raster-scale 3;
      @filter-mode nearest;
      @canvas 24px 20px;
      background: #fff;
      box-shadow: 1px 2px #000;
    }",
  )
  .unwrap();

  assert_eq!(request.kind, DeclarationKind::NineSlice);
  assert_eq!(
    request.metadata.subject,
    LogicalRect {
      x: 1.0,
      y: 2.0,
      width: 18.0,
      height: 15.0
    }
  );
  assert_eq!(
    request.metadata.slices,
    Some(Insets {
      top: 1.0,
      right: 2.0,
      bottom: 3.0,
      left: 4.0
    })
  );
  assert_eq!(
    request.metadata.allowed_clipping,
    [ClipEdge::Top, ClipEdge::Bottom, ClipEdge::Left]
  );
  assert_eq!(request.metadata.raster_scale, 3);
  assert_eq!(request.metadata.filter_mode, FilterMode::Nearest);
  assert_eq!(request.metadata.wrap_mode, WrapMode::Repeat);
  assert_eq!(request.metadata.compression, Compression::LossyHigh);
}

#[test]
fn parses_text_image_requirements_and_font_path() {
  let request = parse(
    "@text-image HEADING {
      font-size: 18px;
      @canvas 120px 30px;
      @font-file unity(\"Assets/Fonts/Heading.ttf\");
      content: \"Ready\";
      filter: brightness(1.1);
    }",
  )
  .unwrap();

  assert_eq!(request.kind, DeclarationKind::TextImage);
  assert_eq!(
    request.metadata.font_file.as_deref(),
    Some("Assets/Fonts/Heading.ttf")
  );
  assert_eq!(
    request
      .paint
      .iter()
      .map(|value| value.property.as_str())
      .collect::<Vec<_>>(),
    ["content", "filter", "font-size"]
  );
}

#[test]
fn parses_trilinear_filtering_for_minified_assets() {
  let request = parse(
    "@background LABEL { @canvas 120px 30px; @filter-mode trilinear; background: red; box-shadow: 1px 2px red; }",
  )
  .unwrap();

  assert_eq!(request.metadata.filter_mode, FilterMode::Trilinear);
}

#[test]
fn reports_every_required_statement() {
  let cases = [
    ("@background PANEL { background: red; }", "@canvas"),
    ("@nine-slice FRAME { @canvas 10px 10px; }", "@slices"),
    (
      "@text-image LABEL { @canvas 10px 10px; content: \"x\"; font-size: 1px; }",
      "@font-file",
    ),
    (
      "@text-image LABEL { @canvas 10px 10px; @font-file unity(\"Assets/a.ttf\"); font-size: 1px; }",
      "content",
    ),
    (
      "@text-image LABEL { @canvas 10px 10px; @font-file unity(\"Assets/a.ttf\"); content: \"x\"; }",
      "font-size",
    ),
  ];

  for (source, property) in cases {
    assert_diagnostic(source, DiagnosticCategory::MissingStatement, property);
  }
}

#[test]
fn reports_forbidden_metadata_placement() {
  let cases = [
    (
      "@background PANEL { @canvas 10px 10px; @slices 1px 1px 1px 1px; }",
      "@slices",
    ),
    (
      "@background PANEL { @canvas 10px 10px; @font-file unity(\"Assets/a.ttf\"); }",
      "@font-file",
    ),
    (
      "@nine-slice FRAME { @canvas 10px 10px; @slices 1px 1px 1px 1px; @font-file unity(\"Assets/a.ttf\"); }",
      "@font-file",
    ),
    (
      "@text-image LABEL { @canvas 10px 10px; @slices 1px 1px 1px 1px; @font-file unity(\"Assets/a.ttf\"); content: \"x\"; font-size: 1px; }",
      "@slices",
    ),
  ];

  for (source, property) in cases {
    assert_diagnostic(source, DiagnosticCategory::ForbiddenStatement, property);
  }
}

#[test]
fn validates_canvas_and_subject_geometry_boundaries() {
  let invalid = [
    "@background PANEL { @canvas 0px 10px; }",
    "@background PANEL { @canvas 10px -1px; }",
    "@background PANEL { @canvas 10.25px 10px; }",
    "@background PANEL { @canvas 1e100px 10px; }",
  ];
  for source in invalid {
    assert_diagnostic(source, DiagnosticCategory::InvalidGeometry, "@canvas");
  }
  for source in [
    "@background PANEL { @canvas 10 10px; }",
    "@background PANEL { @canvas calc(5px) 10px; }",
  ] {
    assert_diagnostic(source, DiagnosticCategory::InvalidMetadata, "@canvas");
  }

  for source in [
    "@background PANEL { @canvas 10px 10px; @subject -1px 0px 1px 1px; }",
    "@background PANEL { @canvas 10px 10px; @subject 9px 0px 2px 1px; }",
    "@background PANEL { @canvas 10px 10px; @subject 0px 9px 1px 2px; }",
  ] {
    assert_diagnostic(source, DiagnosticCategory::InvalidGeometry, "@subject");
  }

  let boundary = parse(
    "@background PANEL { @canvas 10px 10px; @subject 10px 10px 0px 0px; box-shadow: 1px 2px red; }",
  )
  .unwrap();
  assert_eq!(
    boundary.metadata.subject,
    LogicalRect {
      x: 10.0,
      y: 10.0,
      width: 0.0,
      height: 0.0
    }
  );
}

#[test]
fn validates_slice_geometry_and_effective_scale() {
  for source in [
    "@nine-slice FRAME { @canvas 10px 10px; @slices -1px 1px 1px 1px; }",
    "@nine-slice FRAME { @canvas 10px 10px; @slices 5px 1px 5px 1px; }",
    "@nine-slice FRAME { @canvas 10px 10px; @slices 1px 5px 1px 5px; }",
    "@nine-slice FRAME { @canvas 10px 10px; @slices 0.25px 1px 1px 1px; }",
    "@nine-slice FRAME { @canvas 10px 10px; @slices 0.1px 1px 1px 1px; @raster-scale 3; }",
  ] {
    assert_diagnostic(source, DiagnosticCategory::InvalidGeometry, "@slices");
  }

  parse("@nine-slice FRAME { @canvas 10px 10px; @slices 0.25px 1px 1px 1px; @raster-scale 4; box-shadow: 1px 2px red; }")
    .unwrap();
}

#[test]
fn validates_clipping_edges_as_an_ordered_set() {
  for value in ["right top", "top top", "left bottom", "middle"] {
    let source = format!("@background PANEL {{ @canvas 10px 10px; @allow-clipping {value}; }}");
    let category = if value == "middle" {
      DiagnosticCategory::InvalidMetadata
    } else {
      DiagnosticCategory::InvalidClippingOrder
    };
    assert_diagnostic(&source, category, "@allow-clipping");
  }

  for value in ["top", "right bottom", "top right bottom left"] {
    parse(&format!(
      "@background PANEL {{ @canvas 10px 10px; @allow-clipping {value}; box-shadow: 1px 2px red; }}"
    ))
    .unwrap();
  }
}

#[test]
fn validates_metadata_keywords_and_ranges() {
  let invalid = [
    ("@raster-scale 0", "@raster-scale"),
    ("@raster-scale 9", "@raster-scale"),
    ("@raster-scale 1.5", "@raster-scale"),
    ("@filter-mode linear", "@filter-mode"),
    ("@wrap-mode mirror", "@wrap-mode"),
    ("@compression fast", "@compression"),
  ];
  for (statement, property) in invalid {
    assert_diagnostic(
      &format!("@background PANEL {{ @canvas 10px 10px; {statement}; }}"),
      DiagnosticCategory::InvalidMetadata,
      property,
    );
  }

  for statement in [
    "@raster-scale 2",
    "@filter-mode bilinear",
    "@wrap-mode clamp",
    "@compression lossless",
    "@subject 0px 0px 10px 10px",
  ] {
    assert_diagnostic(
      &format!("@background PANEL {{ @canvas 10px 10px; {statement}; }}"),
      DiagnosticCategory::RedundantDefault,
      statement.split_whitespace().next().unwrap(),
    );
  }

  for scale in [1, 3, 4, 5, 6, 7, 8] {
    parse(&format!(
      "@background PANEL {{ @canvas 14px 10px; @raster-scale {scale}; box-shadow: 1px 2px red; }}"
    ))
    .unwrap();
  }
}

#[test]
fn validates_font_file_form_and_closed_statement_catalog() {
  for value in [
    "\"Assets/a.ttf\"",
    "url(\"Assets/a.ttf\")",
    "unity(\"\")",
    "unity(\"a\", \"b\")",
  ] {
    assert_diagnostic(
      &format!(
        "@text-image LABEL {{ @canvas 10px 10px; @font-file {value}; content: \"x\"; font-size: 1px; }}"
      ),
      DiagnosticCategory::InvalidMetadata,
      "@font-file",
    );
  }

  assert_diagnostic(
    "@background PANEL { @canvas 10px 10px; @mystery yes; }",
    DiagnosticCategory::UnknownStatement,
    "@mystery",
  );
  assert_diagnostic(
    "@background PANEL { @canvas 10px 10px; color: red; }",
    DiagnosticCategory::UnknownStatement,
    "color",
  );
  assert_diagnostic(
    "@text-image LABEL { @canvas 10px 10px; @font-file unity(\"a.ttf\"); content: \"x\"; font-size: 1px; border: none; }",
    DiagnosticCategory::UnknownStatement,
    "border",
  );
}

#[test]
fn metadata_diagnostics_include_stable_symbol_property_and_span() {
  let diagnostic =
    parse("\n@background PANEL {\n  @canvas 10px 10px;\n  @raster-scale 2;\n}").unwrap_err();

  assert_eq!(diagnostic.category.code(), "redundant-default");
  assert_eq!(diagnostic.symbol.as_deref(), Some("PANEL"));
  assert_eq!(diagnostic.property.as_deref(), Some("@raster-scale"));
  assert_eq!(diagnostic.span.start_line, 4);
  assert_eq!(diagnostic.span.start_column, 3);
}

fn assert_diagnostic(source: &str, category: DiagnosticCategory, property: &str) -> Diagnostic {
  let diagnostic = parse(source).unwrap_err();
  assert_eq!(diagnostic.category, category, "{source}");
  assert_eq!(
    diagnostic.symbol.as_deref(),
    Some(source.split_whitespace().nth(1).unwrap())
  );
  assert_eq!(diagnostic.property.as_deref(), Some(property), "{source}");
  assert!(diagnostic.span.start_line > 0);
  assert!(diagnostic.span.start_column > 0);
  diagnostic
}
