use std::{fs, process::Command};

const REPRODUCTION: &str = r#"
fn prompt() {
  let action = opaque(input_labels::action(0));
  stylo!(text(txa("Press a key for {action}", tx_args![action], "Binding capture.")));
}
"#;

#[test]
fn hidden_opaque_arguments_fail_preflight_with_source_and_supported_form() {
  let directory = tempfile::tempdir().unwrap();
  let source = directory.path().join("binding.rs");
  fs::write(&source, REPRODUCTION).unwrap();
  let output = Command::new(env!("CARGO_BIN_EXE_trox-argument-check"))
    .arg(&source)
    .output()
    .unwrap();
  assert!(!output.status.success());
  let diagnostic = String::from_utf8(output.stderr).unwrap();
  assert!(diagnostic.contains("binding.rs:4:"), "{diagnostic}");
  assert!(
    diagnostic.contains("opaque argument `action` is hidden"),
    "{diagnostic}"
  );
  assert!(
    diagnostic.contains("tx_args![action => opaque(localized_value)]"),
    "{diagnostic}"
  );
}

#[test]
fn aliases_named_arguments_and_reassignment_cannot_conceal_opaque_values() {
  self::expect_fail(
    r#"
fn prompt() {
  let action = trox::opaque(label());
  let alias = action.clone();
  let mut delayed;
  delayed = alias;
  let _ = trox::tx_args![name => delayed];
}
"#,
  );
}

#[test]
fn qualified_and_parenthesized_inline_markers_are_not_extractable() {
  self::expect_fail(
    r#"
fn prompt() {
  let _ = tx_args![action => trox::opaque(label())];
}
"#,
  );
  self::expect_fail(
    r#"
fn prompt() {
  let _ = tx_args![action => (opaque(label()))];
}
"#,
  );
}

#[test]
fn scalar_arguments_and_inline_markers_preserve_scope_and_ignore_comments_and_strings() {
  self::expect_pass(
    r####"
fn prompt() {
  // let action = opaque(label()); tx_args![action];
  let documentation = r###"let action = opaque(label()); tx_args![action]"###;
  let action = opaque(label());
  { let action = "Left"; let _ = tx_args![action]; }
  let closure = |action: String| tx_args![action];
  fn numeric(action: u32) { let _ = tx_args![action]; }
  let _ = tx_args![action => opaque(label()), count => 2, label => "Left"];
  let rate = 60;
  let _ = tx_args![rate, text => documentation];
}
"####,
  );
}

fn expect_fail(source: &str) {
  let directory = tempfile::tempdir().unwrap();
  let path = directory.path().join("source.rs");
  fs::write(&path, source).unwrap();
  assert!(
    !Command::new(env!("CARGO_BIN_EXE_trox-argument-check"))
      .arg(path)
      .status()
      .unwrap()
      .success()
  );
}

fn expect_pass(source: &str) {
  let directory = tempfile::tempdir().unwrap();
  let path = directory.path().join("source.rs");
  fs::write(&path, source).unwrap();
  assert!(
    Command::new(env!("CARGO_BIN_EXE_trox-argument-check"))
      .arg(path)
      .status()
      .unwrap()
      .success()
  );
}
