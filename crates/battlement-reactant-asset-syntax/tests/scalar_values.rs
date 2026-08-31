use battlement_reactant_asset_syntax::{
  DiagnosticCategory, canonicalize_value, parse, serialize_value, value_identity,
};

#[test]
fn equivalent_numbers_and_negative_zero_have_one_identity() {
  for equivalent in [["1", "1.0", "1e0", "+1"], ["0", "-0", "0.0", "-0e10"]] {
    let identity = value_identity(equivalent[0]).unwrap();
    for value in equivalent {
      assert_eq!(value_identity(value).unwrap(), identity, "{value}");
    }
  }

  assert_ne!(value_identity("1").unwrap(), value_identity("1px").unwrap());
  assert_ne!(
    value_identity("1px").unwrap(),
    value_identity("1%").unwrap()
  );
}

#[test]
fn serializes_shortest_round_tripping_scalars_and_normalized_names() {
  let cases = [
    ("1.5000", "1.5"),
    ("-0px", "0px"),
    ("25%", "25%"),
    ("0.25turn", "0.25turn"),
    ("10VMIN", "10vmin"),
    ("SOME-KEYWORD", "some-keyword"),
    ("\"hello\"", "\"hello\""),
  ];
  for (source, expected) in cases {
    assert_eq!(serialize_value(source).unwrap(), expected, "{source}");
  }
}

#[test]
fn equivalent_css_colors_canonicalize_to_explicit_rgba() {
  let red = value_identity("red").unwrap();
  for value in [
    "#f00",
    "#ff0000",
    "#ff0000ff",
    "rgb(255, 0, 0)",
    "hsl(0, 100%, 50%)",
  ] {
    assert_eq!(
      value_identity(value).unwrap_or_else(|error| panic!("{value}: {error:?}")),
      red,
      "{value}"
    );
  }
  assert_eq!(
    value_identity("transparent").unwrap(),
    value_identity("#0000").unwrap()
  );
  assert!(
    serialize_value("rebeccapurple")
      .unwrap()
      .starts_with("rgba(")
  );
}

#[test]
fn parses_functions_and_preserves_list_order() {
  for value in [
    "paint(1px 25% navy)",
    "paint(1px, 25%, navy)",
    "outer(inner(1), \"label\")",
    "blur(2px) contrast(1.5)",
  ] {
    assert!(!canonicalize_value(value).unwrap().is_empty(), "{value}");
  }

  assert_ne!(
    value_identity("blur(2px) contrast(1.5)").unwrap(),
    value_identity("contrast(1.5) blur(2px)").unwrap()
  );
  assert_ne!(
    value_identity("1, 2").unwrap(),
    value_identity("2, 1").unwrap()
  );
}

#[test]
fn rejects_nonfinite_and_unknown_dimensions() {
  for value in ["1e999", "10qu"] {
    assert_eq!(
      canonicalize_value(value).unwrap_err().category,
      DiagnosticCategory::InvalidValue,
      "{value}"
    );
  }
}

#[test]
fn accepts_typed_calculations() {
  for value in [
    "calc(10px + 2px)",
    "calc(2 * 10px / 4)",
    "min(1em, 10px)",
    "max(25%, 10px)",
    "clamp(1px, 5%, 10px)",
    "calc((1px + 2px) * 3)",
  ] {
    assert!(!canonicalize_value(value).unwrap().is_empty(), "{value}");
  }
}

#[test]
fn rejects_invalid_typed_arithmetic() {
  for value in [
    "calc(1px + 1deg)",
    "calc(1px * 1px)",
    "calc(1px / 0)",
    "calc(1 / (1 - 1))",
    "calc(1e308 * 1e308)",
    "min(1px, 1deg)",
    "clamp(1px, 2px)",
  ] {
    assert_eq!(
      canonicalize_value(value).unwrap_err().category,
      DiagnosticCategory::InvalidArithmetic,
      "{value}"
    );
  }
}

#[test]
fn request_identity_ignores_spelling_symbol_location_and_statement_order() {
  let first =
    parse("@background FIRST { @canvas 20px 10px; opacity: 0.5; background: rgb(255, 0, 0); }")
      .unwrap();
  let second = parse(
    "\n\n@background SECOND {\n  background: #f00;\n  @canvas 20px 10px;\n  opacity: 5e-1;\n}",
  )
  .unwrap();

  assert_eq!(first.canonical_bytes(), second.canonical_bytes());
  assert_eq!(first.identity(), second.identity());
}

#[test]
fn request_identity_includes_kind_metadata_and_ordered_values() {
  let base =
    parse("@background PANEL { @canvas 20px 10px; transform: translate(1px) rotate(2deg); }")
      .unwrap();
  let reordered =
    parse("@background PANEL { @canvas 20px 10px; transform: rotate(2deg) translate(1px); }")
      .unwrap();
  let resized =
    parse("@background PANEL { @canvas 21px 10px; transform: translate(1px) rotate(2deg); }")
      .unwrap();

  assert_ne!(base.identity(), reordered.identity());
  assert_ne!(base.identity(), resized.identity());
}

#[test]
fn arithmetic_diagnostics_retain_declaration_context() {
  let error = parse(
    "@background PANEL {\n  @canvas 20px 10px;\n  background: red;\n  opacity: calc(1px + 1deg);\n}",
  )
  .unwrap_err();

  assert_eq!(error.category, DiagnosticCategory::InvalidArithmetic);
  assert_eq!(error.symbol.as_deref(), Some("PANEL"));
  assert_eq!(error.property.as_deref(), Some("opacity"));
  assert_eq!(error.span.start_line, 4);
  assert_eq!(error.span.start_column, 3);
}
