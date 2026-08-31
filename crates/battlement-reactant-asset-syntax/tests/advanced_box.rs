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
