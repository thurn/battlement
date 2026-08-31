use battlement_reactant_asset_syntax::{DiagnosticCategory, parse};

#[test]
fn accepts_every_filter_and_two_dimensional_transform_family() {
  for declaration in [
    "filter: blur(2px) brightness(1.2) contrast(80%) drop-shadow(1px 2px 3px red) grayscale(50%) hue-rotate(45deg) invert(1) opacity(0.5) saturate(2) sepia(25%)",
    "transform: translate(1px, 20%) rotate(45deg) scale(2, 0.5) skew(1deg, 2deg) skewX(3deg) skewY(4deg) matrix(1, 2, 3, 4, 5, 6)",
    "transform-origin: left 2px bottom 3px",
  ] {
    request(declaration).unwrap_or_else(|error| panic!("{declaration}: {error:?}"));
  }
}

#[test]
fn preserves_effect_order_and_normalizes_origins() {
  let first = request("filter: blur(1px) invert(1); transform: translate(1px) rotate(2deg); transform-origin: top left").unwrap();
  let equivalent = request("transform-origin: left top; transform: translate(1.0px) rotate(2.0deg); filter: blur(1.0px) invert(1.0)").unwrap();
  let reordered = request("filter: invert(1) blur(1px); transform: rotate(2deg) translate(1px); transform-origin: left top").unwrap();

  assert_eq!(first.identity(), equivalent.identity());
  assert_ne!(first.identity(), reordered.identity());
}

#[test]
fn rejects_unsupported_invalid_and_default_effects() {
  for (declaration, category) in [
    ("filter: none", DiagnosticCategory::RedundantDefault),
    ("filter: blur(0)", DiagnosticCategory::RedundantDefault),
    (
      "filter: brightness(1)",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "filter: grayscale(0%)",
      DiagnosticCategory::RedundantDefault,
    ),
    ("filter: blur(-1px)", DiagnosticCategory::InvalidValue),
    ("filter: grayscale(2)", DiagnosticCategory::InvalidValue),
    (
      "filter: drop-shadow(inset 1px 2px red)",
      DiagnosticCategory::InvalidValue,
    ),
    ("filter: url(\"x\")", DiagnosticCategory::InvalidValue),
    ("transform: none", DiagnosticCategory::RedundantDefault),
    (
      "transform: translate(0)",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "transform: rotate(0deg)",
      DiagnosticCategory::RedundantDefault,
    ),
    ("transform: scale(1)", DiagnosticCategory::RedundantDefault),
    (
      "transform: matrix(1, 0, 0, 1, 0, 0)",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "transform: perspective(1px)",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "transform: translate3d(1px, 2px, 3px)",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "transform-origin: center",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "transform-origin: left right",
      DiagnosticCategory::InvalidValue,
    ),
  ] {
    assert_eq!(
      request(declaration).unwrap_err().category,
      category,
      "{declaration}"
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
    "@background PANEL {{ @canvas 20px 10px; {declarations}; box-shadow: 1px 2px red; }}"
  ))
}
