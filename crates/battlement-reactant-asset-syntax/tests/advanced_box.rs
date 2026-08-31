use battlement_reactant_asset_syntax::{DiagnosticCategory, parse};

#[test]
fn accepts_closed_border_catalog_and_component_orderings() {
  for declaration in [
    "border: 2px solid red",
    "border: dotted #1238 0",
    "border: rgb(1, 2, 3) 0.25rem dashed",
    "border: double hsl(120, 100%, 50%) calc(1px + 1em)",
    "border: none",
    "border-width: 1px 2em 3vw 4vmax",
    "border-style: none solid dashed dotted",
    "border-color: red #0f0 rgb(0, 0, 255) transparent",
    "border-top: solid 1px red",
    "border-right: #00f dashed 2vh",
    "border-bottom: 3vmin dotted rebeccapurple",
    "border-left: double transparent 4rem",
  ] {
    request(declaration).unwrap_or_else(|error| panic!("{declaration}: {error:?}"));
  }
}

#[test]
fn border_shorthands_and_expanded_longhands_have_one_identity() {
  let shorthand = request("border: 2px solid red").unwrap();
  let longhands =
    request("border-width: 2px; border-style: solid; border-color: rgb(255, 0, 0)").unwrap();
  let sides = request(
    "border-top: red solid 2px; border-right: 2px solid #f00; border-bottom: solid red 2px; border-left: 2px #ff0000 solid",
  )
  .unwrap();

  assert_eq!(shorthand.identity(), longhands.identity());
  assert_eq!(shorthand.identity(), sides.identity());
}

#[test]
fn one_to_four_value_longhands_expand_by_css_side_order() {
  for (compact, expanded) in [
    ("border-width: 1px", "border-width: 1px 1px 1px 1px"),
    (
      "border-style: solid dashed",
      "border-style: solid dashed solid dashed",
    ),
    (
      "border-color: red green blue",
      "border-color: red green blue green",
    ),
  ] {
    assert_eq!(
      request(compact).unwrap().identity(),
      request(expanded).unwrap().identity(),
      "{compact}"
    );
  }
}

#[test]
fn nonoverlapping_edge_declarations_ignore_statement_order() {
  let first = request(
    "border-top: 1px solid red; border-right: 2px dashed green; border-bottom: 3px dotted blue; border-left: 4px double #1238",
  )
  .unwrap();
  let reordered = request(
    "border-left: 4px double #1238; border-bottom: blue dotted 3px; border-right: green 2px dashed; border-top: red solid 1px",
  )
  .unwrap();

  assert_eq!(first.identity(), reordered.identity());
  assert_eq!(first.paint[0].property, "border-bottom");
  assert_eq!(first.paint.len(), 4);
}

#[test]
fn rejects_every_border_shorthand_overlap_in_either_order() {
  for (first, second) in [
    ("border: 1px solid red", "border-top: 2px dashed blue"),
    ("border-width: 1px", "border-left: 2px solid red"),
    ("border-color: red", "border-bottom: 2px solid blue"),
    ("border-style: solid", "border: 2px dashed blue"),
  ] {
    for source in [format!("{first}; {second}"), format!("{second}; {first}")] {
      let expected_property = source.rsplit_once(';').unwrap().1.trim();
      let expected_property = expected_property.split_once(':').unwrap().0;
      let error = request(&source).unwrap_err();
      assert_eq!(
        error.category,
        DiagnosticCategory::DuplicateStatement,
        "{source}"
      );
      assert_eq!(error.symbol.as_deref(), Some("PANEL"), "{source}");
      assert_eq!(
        error.property.as_deref(),
        Some(expected_property),
        "{source}"
      );
    }
  }
}

#[test]
fn accepts_one_to_four_and_elliptical_corner_radii() {
  for declaration in [
    "border-radius: 1px",
    "border-radius: 1px 2em",
    "border-radius: 1px 2rem 3vw",
    "border-radius: 1px 2vh 3vmin 4vmax",
    "border-radius: 10% / 2px",
    "border-radius: 10% 20% / 1em 2rem 3vw 4vh",
    "border-radius: calc(10% - 1px) min(2em, 3px)",
  ] {
    request(declaration).unwrap_or_else(|error| panic!("{declaration}: {error:?}"));
  }
}

#[test]
fn radius_expansion_and_scalar_spelling_are_canonical() {
  let compact = request("border-radius: 1px 2px 3px").unwrap();
  let expanded = request("border-radius: 1.0px 2e0px 3.00px 2px").unwrap();
  let elliptical = request("border-radius: 1px 2px / 3px 4px").unwrap();
  let elliptical_expanded = request("border-radius: 1px 2px 1px 2px / 3px 4px 3px 4px").unwrap();

  assert_eq!(compact.identity(), expanded.identity());
  assert_eq!(elliptical.identity(), elliptical_expanded.identity());
}

#[test]
fn rejects_border_values_outside_the_closed_grammar() {
  for declaration in [
    "border: solid red",
    "border: 1px solid",
    "border: 1px solid solid red",
    "border: 1px 2px solid red",
    "border: 1px groove red",
    "border: 10% solid red",
    "border: -1px solid red",
    "border: thin solid red",
    "border: 1px solid currentColor",
    "border: 1px solid canvastext",
    "border-width: 1px 2px 3px 4px 5px",
    "border-width: calc(-1px)",
    "border-style: hidden",
    "border-color: currentColor",
    "border-top: none red",
  ] {
    let error = request(declaration).unwrap_err();
    assert_eq!(
      error.category,
      DiagnosticCategory::InvalidValue,
      "{declaration}"
    );
    assert_eq!(error.symbol.as_deref(), Some("PANEL"), "{declaration}");
    assert_eq!(
      error.property.as_deref(),
      declaration.split_once(':').map(|value| value.0),
      "{declaration}"
    );
  }
}

#[test]
fn rejects_malformed_or_redundant_radius_forms() {
  for (declaration, category) in [
    ("border-radius: -1px", DiagnosticCategory::InvalidValue),
    ("border-radius: 1deg", DiagnosticCategory::InvalidValue),
    (
      "border-radius: 1px 2px 3px 4px 5px",
      DiagnosticCategory::InvalidValue,
    ),
    ("border-radius: / 1px", DiagnosticCategory::InvalidValue),
    ("border-radius: 1px /", DiagnosticCategory::InvalidValue),
    (
      "border-radius: 1px / 1px",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "border-radius: 1px 2px / 1px 2px",
      DiagnosticCategory::RedundantDefault,
    ),
  ] {
    let error = request(declaration).unwrap_err();
    assert_eq!(error.category, category, "{declaration}");
    assert_eq!(error.property.as_deref(), Some("border-radius"));
  }
}

#[test]
fn accepts_ordered_outer_and_inset_shadow_layers() {
  for declaration in [
    "box-shadow: 0 2px red",
    "box-shadow: #1238 -1em 2rem 3vw -4vh inset",
    "box-shadow: inset 1vmin 2vmax rgb(1, 2, 3)",
    "box-shadow: 1px 2px red, inset blue -3px 4px 5px 6px",
    "box-shadow: calc(1px + 1em) 0 hsl(120, 100%, 50%)",
  ] {
    request(declaration).unwrap_or_else(|error| panic!("{declaration}: {error:?}"));
  }
}

#[test]
fn shadow_identity_normalizes_values_but_preserves_layer_order() {
  let first = request("box-shadow: 1px 2px #ff0000, inset 3px 4px blue").unwrap();
  let equivalent = request("box-shadow: rgb(255, 0, 0) 1.0px 2e0px, 3px blue 4px inset").unwrap();
  let reordered = request("box-shadow: inset 3px 4px blue, 1px 2px red").unwrap();

  assert_eq!(first.identity(), equivalent.identity());
  assert_ne!(first.identity(), reordered.identity());
}

#[test]
fn rejects_invalid_or_external_shadow_values() {
  for (declaration, category) in [
    ("box-shadow: none", DiagnosticCategory::RedundantDefault),
    ("box-shadow: 1px red", DiagnosticCategory::InvalidValue),
    ("box-shadow: 1px 2px", DiagnosticCategory::InvalidValue),
    (
      "box-shadow: 1px 2px currentColor",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "box-shadow: 1px 2px canvastext",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "box-shadow: 1px 2px -3px red",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "box-shadow: 1px 2px 0 red",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "box-shadow: 1px 2px 3px 0 red",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "box-shadow: 1px 2px 3px 4px 5px red",
      DiagnosticCategory::InvalidValue,
    ),
    ("box-shadow: 1% 2px red", DiagnosticCategory::InvalidValue),
    (
      "box-shadow: inset inset 1px 2px red",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "box-shadow: 1px 2px red blue",
      DiagnosticCategory::InvalidValue,
    ),
  ] {
    let error = request(declaration).unwrap_err();
    assert_eq!(error.category, category, "{declaration}");
    assert_eq!(error.symbol.as_deref(), Some("PANEL"));
    assert_eq!(error.property.as_deref(), Some("box-shadow"));
  }
}

#[test]
fn accepts_every_basic_clip_shape() {
  for declaration in [
    "clip-path: inset(1px)",
    "clip-path: inset(-1px 2% 3em 4vw)",
    "clip-path: inset(1px 2px round 3px)",
    "clip-path: inset(1px round 10% 20% / 1em 2rem 3vh 4vmax)",
    "clip-path: circle()",
    "clip-path: circle(20%)",
    "clip-path: circle(farthest-side at left top)",
    "clip-path: circle(calc(25% - 1px) at right 2px bottom 3px)",
    "clip-path: ellipse()",
    "clip-path: ellipse(farthest-side at 25% 75%)",
    "clip-path: ellipse(10px 20% at left bottom)",
    "clip-path: polygon(0% 0%, 100% 0%, 50% 100%)",
    "clip-path: polygon(evenodd, 0px 0px, 10px 0px, 10px 10px, 0px 10px)",
    "clip-path: polygon(calc(10% + 1px) 0, calc(30% + 1px) 0, calc(20% + 1px) 10px)",
  ] {
    request(declaration).unwrap_or_else(|error| panic!("{declaration}: {error:?}"));
  }
}

#[test]
fn clip_identity_expands_inset_shorthands_and_normalizes_scalars() {
  let compact = request("clip-path: inset(1px 2px round 3px / 4px)").unwrap();
  let expanded =
    request("clip-path: inset(1.0px 2e0px 1px 2px round 3px 3px 3px 3px / 4px 4px 4px 4px)")
      .unwrap();
  let polygon = request("clip-path: polygon(0% 0%, 100% 0%, 50% 100%)").unwrap();
  let equivalent = request("clip-path: polygon(0.0% 0e0%, 1e2% 0%, 50.00% 100%)").unwrap();
  let horizontal_first = request("clip-path: circle(20% at left top)").unwrap();
  let vertical_first = request("clip-path: circle(20% at top left)").unwrap();

  assert_eq!(compact.identity(), expanded.identity());
  assert_eq!(polygon.identity(), equivalent.identity());
  assert_eq!(horizontal_first.identity(), vertical_first.identity());
}

#[test]
fn rejects_invalid_degenerate_and_external_clip_shapes() {
  for (declaration, category) in [
    ("clip-path: none", DiagnosticCategory::RedundantDefault),
    (
      "clip-path: url(\"https://example.com/a.svg\")",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "clip-path: rect(1px 2px 3px 4px)",
      DiagnosticCategory::InvalidValue,
    ),
    ("clip-path: circle(-1px)", DiagnosticCategory::InvalidValue),
    (
      "clip-path: circle(closest-side)",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "clip-path: circle(10px at center)",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "clip-path: circle(1px 2px)",
      DiagnosticCategory::InvalidValue,
    ),
    ("clip-path: ellipse(1px)", DiagnosticCategory::InvalidValue),
    (
      "clip-path: ellipse(-1px 2px)",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "clip-path: inset(1px round -2px)",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "clip-path: inset(1px round 2px / 2px)",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "clip-path: polygon(0 0, 1px 1px)",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "clip-path: polygon(0px 0px, 10px 0px, 20px 0px)",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "clip-path: polygon(0.1px 0.3px, 0.2px 0.6px, 0.3px 0.9px)",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "clip-path: polygon(0 0, 0px 0%, 10px 10px)",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "clip-path: polygon(nonzero, 0 0, 10px 0, 0 10px)",
      DiagnosticCategory::RedundantDefault,
    ),
  ] {
    let error = request(declaration).unwrap_err();
    assert_eq!(error.category, category, "{declaration}");
    assert_eq!(error.symbol.as_deref(), Some("PANEL"));
    assert_eq!(error.property.as_deref(), Some("clip-path"));
  }
}

#[test]
fn accepts_every_typed_absolute_clip_path_command() {
  for declaration in [
    "clip-path: path(\"M 0 0 L 10 10 H 20 V 30 C 1 2 3 4 5 6 Q 7 8 9 10 A 5 6 45 0 1 20 20 Z\")",
    "clip-path: path(\"M0 0 10 10 20 20Z\")",
    "clip-path: path(\"M.5.6L1e1-2e1L+30,+40\")",
    "clip-path: path(\"M0 0L1 2 3 4H5 6V7 8C1 2 3 4 5 6 7 8 9 10 11 12Q1 2 3 4 5 6 7 8A1 2 3 0 1 4 5 6 7 8 1 0 9 10\")",
  ] {
    request(declaration).unwrap_or_else(|error| panic!("{declaration}: {error:?}"));
  }
}

#[test]
fn path_identity_normalizes_numbers_separators_and_implicit_lines() {
  let compact = request("clip-path: path(\"M0,0 10,10L20-20Z\")").unwrap();
  let expanded = request("clip-path: path(\"M 0.0 0e0 L 1e1 10.00 L 20 -20 Z\")").unwrap();
  let reordered = request("clip-path: path(\"M0 0L20-20L10 10Z\")").unwrap();

  assert_eq!(compact.identity(), expanded.identity());
  assert_ne!(compact.identity(), reordered.identity());
}

#[test]
fn rejects_malformed_relative_and_unsupported_clip_paths() {
  for declaration in [
    "clip-path: path(\"\")",
    "clip-path: path(\"M0 0\")",
    "clip-path: path(\"M0 0Z\")",
    "clip-path: path(\"L0 0\")",
    "clip-path: path(\"m0 0l1 1\")",
    "clip-path: path(\"M0 0S1 2 3 4\")",
    "clip-path: path(\"M0 0T1 2\")",
    "clip-path: path(\"M0 0L\")",
    "clip-path: path(\"M0 0L1\")",
    "clip-path: path(\"M0 0Z1\")",
    "clip-path: path(\"M,0 0L1 1\")",
    "clip-path: path(\"M0 0L1,,2\")",
    "clip-path: path(\"M0 0L1e 2\")",
    "clip-path: path(\"M0 0L1e309 2\")",
    "clip-path: path(\"M0 0A-1 2 0 0 1 3 4\")",
    "clip-path: path(\"M0 0A1 2 0 2 1 3 4\")",
    "clip-path: path(\"M0 0A1 2 0 0 -1 3 4\")",
    "clip-path: path(M0 0L1 1)",
    "clip-path: path(\"M0 0\", \"L1 1\")",
  ] {
    let error = request(declaration).unwrap_err();
    assert_eq!(
      error.category,
      DiagnosticCategory::InvalidValue,
      "{declaration}"
    );
    assert_eq!(error.symbol.as_deref(), Some("PANEL"));
    assert_eq!(error.property.as_deref(), Some("clip-path"));
  }
}

fn request(
  declarations: &str,
) -> Result<
  battlement_reactant_asset_syntax::AssetRequest,
  battlement_reactant_asset_syntax::Diagnostic,
> {
  parse(&format!(
    "@background PANEL {{ @canvas 20px 10px; {declarations}; }}"
  ))
}
