use battlement_reactant_asset_syntax::{DiagnosticCategory, parse};

#[test]
fn accepts_every_nondefault_internal_background_blend_mode() {
  for mode in [
    "multiply",
    "screen",
    "overlay",
    "darken",
    "lighten",
    "color-dodge",
    "color-burn",
    "hard-light",
    "soft-light",
    "difference",
    "exclusion",
    "hue",
    "saturation",
    "color",
    "luminosity",
  ] {
    request(&format!(
      "background: linear-gradient(red, blue), linear-gradient(white, black); background-blend-mode: {mode}"
    ))
    .unwrap_or_else(|error| panic!("{mode}: {error:?}"));
  }
}

#[test]
fn accepts_one_mode_or_exactly_one_mode_per_background_layer() {
  let repeated = request(
    "background: linear-gradient(red, blue), linear-gradient(white, black), linear-gradient(green, yellow); background-blend-mode: multiply",
  )
  .unwrap();
  let expanded = request(
    "background-blend-mode: multiply, multiply, multiply; background: linear-gradient(red, blue), linear-gradient(white, black), linear-gradient(green, yellow)",
  )
  .unwrap();
  let ordered = request(
    "background: linear-gradient(red, blue), linear-gradient(white, black), linear-gradient(green, yellow); background-blend-mode: screen, overlay, luminosity",
  )
  .unwrap();

  assert_eq!(repeated.identity(), expanded.identity());
  assert_ne!(repeated.identity(), ordered.identity());
}

#[test]
fn rejects_default_unknown_external_and_mismatched_blending() {
  for (declarations, property, category) in [
    (
      "background: linear-gradient(red, blue); background-blend-mode: normal",
      "background-blend-mode",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "background: linear-gradient(red, blue), linear-gradient(white, black); background-blend-mode: multiply, normal",
      "background-blend-mode",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "background: linear-gradient(red, blue); background-blend-mode: plus-lighter",
      "background-blend-mode",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "background-blend-mode: multiply",
      "background-blend-mode",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "background: linear-gradient(red, blue), linear-gradient(white, black), linear-gradient(green, yellow); background-blend-mode: multiply, screen",
      "background-blend-mode",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "background: red; mix-blend-mode: multiply",
      "mix-blend-mode",
      DiagnosticCategory::UnknownStatement,
    ),
    (
      "background: red; backdrop-filter: blur(2px)",
      "backdrop-filter",
      DiagnosticCategory::UnknownStatement,
    ),
  ] {
    let error = request(declarations).unwrap_err();
    assert_eq!(error.category, category, "{declarations}");
    assert_eq!(error.symbol.as_deref(), Some("PANEL"));
    assert_eq!(error.property.as_deref(), Some(property));
  }
}

#[test]
fn accepts_closed_opacity_range_and_isolation() {
  for declaration in [
    "opacity: 0",
    "opacity: 0.5",
    "opacity: calc(1 / 4)",
    "isolation: isolate",
  ] {
    request(declaration).unwrap_or_else(|error| panic!("{declaration}: {error:?}"));
  }
}

#[test]
fn normalizes_compositing_values_and_declaration_order() {
  let first = request("opacity: 0.5; isolation: isolate").unwrap();
  let equivalent = request("isolation: isolate; opacity: 5e-1").unwrap();
  let different = request("opacity: 0.25; isolation: isolate").unwrap();

  assert_eq!(first.identity(), equivalent.identity());
  assert_ne!(first.identity(), different.identity());
}

#[test]
fn rejects_invalid_or_explicit_default_opacity_and_isolation() {
  for (declaration, category) in [
    ("opacity: 1", DiagnosticCategory::RedundantDefault),
    ("opacity: calc(2 / 2)", DiagnosticCategory::RedundantDefault),
    ("opacity: -0.1", DiagnosticCategory::InvalidValue),
    ("opacity: 1.1", DiagnosticCategory::InvalidValue),
    ("opacity: 50%", DiagnosticCategory::InvalidValue),
    ("opacity: calc(1px)", DiagnosticCategory::InvalidValue),
    ("opacity: none", DiagnosticCategory::InvalidValue),
    ("isolation: auto", DiagnosticCategory::RedundantDefault),
    ("isolation: normal", DiagnosticCategory::InvalidValue),
    (
      "isolation: isolate isolate",
      DiagnosticCategory::InvalidValue,
    ),
  ] {
    let error = request(declaration).unwrap_err();
    assert_eq!(error.category, category, "{declaration}");
    assert_eq!(error.symbol.as_deref(), Some("PANEL"));
    assert_eq!(
      error.property.as_deref(),
      declaration.split_once(':').map(|value| value.0)
    );
  }
}

fn request(
  declarations: &str,
) -> Result<
  battlement_reactant_asset_syntax::AssetRequest,
  battlement_reactant_asset_syntax::Diagnostic,
> {
  parse(&format!(
    "@background PANEL {{ @canvas 20px 10px; mask: linear-gradient(red, blue) alpha; {declarations}; }}"
  ))
}
