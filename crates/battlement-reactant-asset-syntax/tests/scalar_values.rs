use battlement_reactant_asset_syntax::{
  DiagnosticCategory, canonicalize_value, serialize_value, value_identity,
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
