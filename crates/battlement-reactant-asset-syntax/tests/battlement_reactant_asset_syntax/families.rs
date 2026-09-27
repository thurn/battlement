use battlement_reactant_asset_syntax::{DeclarationKind, expand_family, parse_envelope};

#[test]
fn family_expansion_reuses_common_statements_and_applies_member_overrides() {
  let sources = expand_family(
    r#"
      @text-image {
        @canvas 80px 24px;
        font-size: 14px;
        color: white;
      }
      READY { content: "Ready"; }
      ALERT { content: "Alert"; color: red; }
    "#,
  )
  .unwrap();

  assert_eq!(sources.len(), 2);
  let ready = parse_envelope(&sources[0]).unwrap();
  let alert = parse_envelope(&sources[1]).unwrap();
  assert_eq!(ready.kind, DeclarationKind::TextImage);
  assert_eq!(ready.symbol, "READY");
  assert_eq!(alert.symbol, "ALERT");
  assert!(sources[0].contains("color: white"));
  assert!(sources[1].contains("color: red"));
  assert!(!sources[1].contains("color: white"));
}

#[test]
fn family_expansion_requires_a_member_and_unique_statements_per_scope() {
  assert!(expand_family("@background { @canvas 10px 10px; }").is_err());
  assert!(
    expand_family(
      "@background { @canvas 10px 10px; @canvas 20px 20px; } PANEL { background: red; }"
    )
    .is_err()
  );
}
