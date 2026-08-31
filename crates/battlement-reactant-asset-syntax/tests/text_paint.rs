use battlement_reactant_asset_syntax::{DiagnosticCategory, parse};

#[test]
fn accepts_plain_text_catalog() {
  for declaration in [
    "font-style: normal",
    "font-style: italic",
    "font-style: oblique 12deg",
    "font-weight: 1",
    "font-weight: 1000",
    "font-stretch: 50%",
    "font-stretch: 200%",
    "line-height: 18px",
    "letter-spacing: -1px",
    "word-spacing: 2em",
    "text-align: justify",
    "white-space: pre-wrap",
    "color: rebeccapurple",
    "-webkit-text-stroke: 1px red",
    "text-shadow: 1px 2px 3px #1238",
  ] {
    request(declaration).unwrap_or_else(|error| panic!("{declaration}: {error:?}"));
  }
}

#[test]
fn accepts_only_complete_advanced_text_fill() {
  for fill in [
    "color: transparent; background: linear-gradient(red, blue); background-clip: text",
    "background-clip: text; background: unity-url(\"Assets/fill.png\"); color: #0000",
  ] {
    request(fill).unwrap_or_else(|error| panic!("{fill}: {error:?}"));
  }
}

#[test]
fn rejects_incomplete_advanced_fill_and_invalid_text_values() {
  for (declaration, property, category) in [
    (
      "background: linear-gradient(red, blue)",
      "background",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "background-clip: text",
      "background-clip",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "color: red; background: linear-gradient(red, blue); background-clip: text",
      "color",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "font-weight: bold",
      "font-weight",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "font-weight: 1000.5",
      "font-weight",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "font-stretch: 49%",
      "font-stretch",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "line-height: normal",
      "line-height",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "letter-spacing: normal",
      "letter-spacing",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "text-align: start",
      "text-align",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "white-space: normal",
      "white-space",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "color: black",
      "color",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "-webkit-text-stroke: 0 red",
      "-webkit-text-stroke",
      DiagnosticCategory::RedundantDefault,
    ),
    (
      "text-shadow: inset 1px 2px red",
      "text-shadow",
      DiagnosticCategory::InvalidValue,
    ),
    (
      "text-shadow: 1px 2px 3px 4px red",
      "text-shadow",
      DiagnosticCategory::InvalidValue,
    ),
  ] {
    let error = request(declaration).unwrap_err();
    assert_eq!(error.category, category, "{declaration}");
    assert_eq!(error.property.as_deref(), Some(property), "{declaration}");
  }
}

#[test]
fn requires_exactly_one_string_and_positive_font_size() {
  for (content, size) in [("\"Hello\nworld\"", "12px"), ("\"\"", "calc(10px + 2px)")] {
    request_with(content, size, "color: blue").unwrap();
  }
  for (content, size, property) in [
    ("attr(title)", "12px", "content"),
    ("\"a\" \"b\"", "12px", "content"),
    ("\"a\"", "-1px", "font-size"),
    ("\"a\"", "10%", "font-size"),
  ] {
    assert_eq!(
      request_with(content, size, "color: blue")
        .unwrap_err()
        .property
        .as_deref(),
      Some(property)
    );
  }
}

fn request(
  declarations: &str,
) -> Result<
  battlement_reactant_asset_syntax::AssetRequest,
  battlement_reactant_asset_syntax::Diagnostic,
> {
  request_with("\"Hello\"", "12px", declarations)
}

fn request_with(
  content: &str,
  size: &str,
  declarations: &str,
) -> Result<
  battlement_reactant_asset_syntax::AssetRequest,
  battlement_reactant_asset_syntax::Diagnostic,
> {
  parse(&format!(
    "@text-image LABEL {{ @canvas 100px 30px; @font-file unity(\"Assets/face.ttf\"); content: {content}; font-size: {size}; {declarations}; filter: brightness(1.1); }}"
  ))
}
