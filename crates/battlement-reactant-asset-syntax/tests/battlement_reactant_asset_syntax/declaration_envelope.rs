use battlement_reactant_asset_syntax::{
  DeclarationKind, DiagnosticCategory, StatementName, parse_envelope,
};

#[test]
fn parses_every_declaration_envelope_and_normalizes_statement_names() {
  let cases = [
    (
      "@background PANEL { @canvas 64px 32px; BACKGROUND: linear-gradient(red, blue); }",
      "PANEL",
      DeclarationKind::Background,
      vec!["@canvas", "background"],
    ),
    (
      "@Nine-Slice FRAME { /* geometry */ @SLICES 2px 3px 4px 5px; @canvas 20px 20px; }",
      "FRAME",
      DeclarationKind::NineSlice,
      vec!["@slices", "@canvas"],
    ),
    (
      "@text-image Heading { content: \"Hello; world\"; @font-file unity(\"Assets/Face.ttf\"); font-size: calc(10px + 2px); @canvas 100px 30px; }",
      "Heading",
      DeclarationKind::TextImage,
      vec!["content", "@font-file", "font-size", "@canvas"],
    ),
  ];

  for (source, symbol, kind, names) in cases {
    let parsed = parse_envelope(source).unwrap();
    assert_eq!(parsed.symbol, symbol);
    assert_eq!(parsed.kind, kind);
    assert_eq!(
      parsed
        .statements
        .iter()
        .map(|statement| match &statement.name {
          StatementName::Metadata(name) => format!("@{name}"),
          StatementName::Property(name) => name.clone(),
        })
        .collect::<Vec<_>>(),
      names
    );
  }
}

#[test]
fn rejects_invalid_static_names_and_outer_declarations() {
  for source in [
    "@background r#TYPE { @canvas 1px 1px; }",
    "@background match { @canvas 1px 1px; }",
    "@background 4EVER { @canvas 1px 1px; }",
  ] {
    assert_eq!(
      parse_envelope(source).unwrap_err().category,
      DiagnosticCategory::InvalidIdentifier
    );
  }
  for source in [
    "background PANEL { @canvas 1px 1px; }",
    "@sprite PANEL { @canvas 1px 1px; }",
    "@background PANEL { @canvas 1px 1px; } trailing",
  ] {
    assert_eq!(
      parse_envelope(source).unwrap_err().category,
      DiagnosticCategory::InvalidDeclaration
    );
  }
  assert_eq!(
    parse_envelope("@background PANEL { @canvas 1px 1px;")
      .unwrap_err()
      .category,
    DiagnosticCategory::InvalidSyntax
  );
}

#[test]
fn statement_uniqueness_is_ascii_case_insensitive() {
  let duplicates = [
    (
      "@background PANEL { @canvas 1px 1px; @CANVAS 2px 2px; }",
      "@canvas",
    ),
    (
      "@background PANEL { background: red; BACKGROUND: blue; }",
      "background",
    ),
  ];
  for (source, property) in duplicates {
    let diagnostic = parse_envelope(source).unwrap_err();
    assert_eq!(diagnostic.category, DiagnosticCategory::DuplicateStatement);
    assert_eq!(diagnostic.symbol.as_deref(), Some("PANEL"));
    assert_eq!(diagnostic.property.as_deref(), Some(property));
  }
}

#[test]
fn diagnostics_retain_symbol_property_and_source_location() {
  let diagnostic =
    parse_envelope("\n@background PANEL {\n  @canvas 1px 1px;\n  @canvas 2px 2px;\n}").unwrap_err();

  assert_eq!(diagnostic.category.code(), "duplicate-statement");
  assert_eq!(diagnostic.symbol.as_deref(), Some("PANEL"));
  assert_eq!(diagnostic.property.as_deref(), Some("@canvas"));
  assert_eq!(diagnostic.span.start_line, 4);
  assert_eq!(diagnostic.span.start_column, 3);
  assert!(
    diagnostic
      .to_string()
      .contains("duplicate-statement in PANEL at @canvas (4:3)")
  );
}

#[test]
fn statements_require_names_values_and_terminators() {
  for source in [
    "@background PANEL { @canvas; }",
    "@background PANEL { background:; }",
    "@background PANEL { background: red }",
    "@background PANEL { div .child: red; }",
  ] {
    assert_eq!(
      parse_envelope(source).unwrap_err().category,
      DiagnosticCategory::InvalidDeclaration
    );
  }
}
