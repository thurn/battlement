use battlement_reactant_asset_syntax::{DependencyKind, DiagnosticCategory, parse};

#[test]
fn accepts_local_and_gradient_masks_with_every_component_family() {
  for mask in [
    "unity-url(\"Assets/Masks/cutout.png\")",
    "unity-url(\"Assets/Masks/cutout.png\") alpha add",
    "linear-gradient(red, transparent) luminance subtract",
    "radial-gradient(circle, white, black) intersect",
    "conic-gradient(red, blue) exclude",
    "unity-url(\"Assets/Masks/a.png\") right 2px bottom 3px / cover no-repeat padding-box alpha",
    "unity-url(\"Assets/Masks/a.png\") center / 10px 20% round space content-box no-clip luminance exclude",
  ] {
    request(mask).unwrap_or_else(|error| panic!("{mask}: {error:?}"));
  }
}

#[test]
fn preserves_layer_order_and_normalizes_layer_components() {
  let first = request(
    "unity-url(\"Assets/Masks/a.png\") top right no-repeat alpha add, linear-gradient(#f00, #00f) luminance exclude",
  )
  .unwrap();
  let equivalent = request(
    "unity-url(\"Assets/Masks/a.png\") right top no-repeat no-repeat add alpha, linear-gradient(rgb(255, 0, 0), blue) exclude luminance",
  )
  .unwrap();
  let reordered = request(
    "linear-gradient(red, blue) luminance exclude, unity-url(\"Assets/Masks/a.png\") right top no-repeat alpha add",
  )
  .unwrap();

  assert_eq!(first.identity(), equivalent.identity());
  assert_ne!(first.identity(), reordered.identity());
}

#[test]
fn exposes_sorted_unique_local_mask_dependencies() {
  let parsed = request(
    "unity-url(\"Assets/Masks/z.png\") alpha, unity-url(\"Assets/Masks/a.PNG\") luminance, unity-url(\"Assets/Masks/z.png\") exclude",
  )
  .unwrap();

  assert_eq!(parsed.dependencies.len(), 2);
  assert_eq!(parsed.dependencies[0].kind, DependencyKind::Image);
  assert_eq!(parsed.dependencies[0].path, "Assets/Masks/a.PNG");
  assert_eq!(parsed.dependencies[1].path, "Assets/Masks/z.png");
}

#[test]
fn rejects_external_missing_and_malformed_mask_sources() {
  for mask in [
    "none",
    "alpha",
    "red",
    "url(\"Assets/Masks/a.png\")",
    "paint(mask)",
    "unity-url(\"../a.png\")",
    "unity-url(\"Assets/a.jpg\")",
    "unity-url(Assets/a.png)",
    "unity-url(\"Assets/a.png\") linear-gradient(red, blue)",
    "unity-url(\"Assets/a.png\") / cover",
    "unity-url(\"Assets/a.png\") center / cover / contain",
  ] {
    let error = request(mask).unwrap_err();
    assert_eq!(error.symbol.as_deref(), Some("PANEL"), "{mask}");
    assert_eq!(error.property.as_deref(), Some("mask"), "{mask}");
    assert!(
      matches!(
        error.category,
        DiagnosticCategory::InvalidValue | DiagnosticCategory::RedundantDefault
      ),
      "{mask}"
    );
  }
}

#[test]
fn rejects_duplicate_unknown_and_redundant_mask_components() {
  for (mask, category) in [
    (
      "unity-url(\"Assets/a.png\") alpha luminance",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "unity-url(\"Assets/a.png\") add exclude",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "unity-url(\"Assets/a.png\") repeat",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "unity-url(\"Assets/a.png\") left top",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "unity-url(\"Assets/a.png\") 0 0",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "unity-url(\"Assets/a.png\") center / auto",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "unity-url(\"Assets/a.png\") border-box",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "unity-url(\"Assets/a.png\") padding-box border-box",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "unity-url(\"Assets/a.png\") no-clip padding-box",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "unity-url(\"Assets/a.png\") mirror",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "unity-url(\"Assets/a.png\") center / 1deg",
      DiagnosticCategory::InvalidValue,
    ),
  ] {
    assert_eq!(request(mask).unwrap_err().category, category, "{mask}");
  }
}

fn request(
  mask: &str,
) -> Result<
  battlement_reactant_asset_syntax::AssetRequest,
  battlement_reactant_asset_syntax::Diagnostic,
> {
  parse(&format!(
    "@background PANEL {{ @canvas 20px 10px; mask: {mask}; }}"
  ))
}
