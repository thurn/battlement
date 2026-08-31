use std::{
  fs,
  path::{Path, PathBuf},
  process::{Command, Output},
};

use serde_json::Value;

#[test]
fn empty_commands_resolve_project_and_remove_only_generated_output() {
  let fixture = Fixture::new();
  fixture.write_generated_output();
  let report = fixture.root.join("generate-report.json");

  let generated = fixture.run_from(
    &fixture.root,
    [
      "reactant",
      "assets",
      "generate",
      "--project",
      "game",
      "--work-report",
      report.to_str().unwrap(),
    ],
  );

  assert!(generated.status.success(), "{}", stderr(&generated));
  assert!(stdout(&generated).contains("browser not started"));
  assert!(!fixture.generated_root().exists());
  assert!(!fixture.generated_meta().exists());
  let report: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
  assert_eq!(report["cargoMetadataRuns"], 2);
  assert_eq!(report["browserLaunches"], 0);
  assert_eq!(report["browserContextsCreated"], 0);
  assert_eq!(report["filesWritten"], 3);

  let nested = fixture.project.join("Assets/Nested/Deeper");
  fs::create_dir_all(&nested).unwrap();
  let checked = fixture.run_from(&nested, ["reactant", "assets", "check"]);
  assert!(checked.status.success(), "{}", stderr(&checked));
}

#[test]
fn check_is_read_only_and_reports_stale_empty_output() {
  let fixture = Fixture::new();
  fixture.write_generated_output();
  let manifest = fixture.generated_root().join("manifest.json");
  let before = fs::read(&manifest).unwrap();
  let report = fixture.root.join("check-report.json");

  let checked = fixture.run_from(
    &fixture.project,
    [
      "reactant",
      "assets",
      "check",
      "--work-report",
      report.to_str().unwrap(),
    ],
  );

  assert!(!checked.status.success());
  assert!(stderr(&checked).contains("assets are stale"));
  assert_eq!(fs::read(manifest).unwrap(), before);
  assert_eq!(
    serde_json::from_slice::<Value>(&fs::read(report).unwrap()).unwrap()["filesWritten"],
    0
  );
}

#[test]
fn selections_reject_non_projects_and_escaped_rules_manifests() {
  let fixture = Fixture::new();
  let outside = fixture.root.join("outside");
  write_rules(&outside);
  let escaped = fixture.run_from(
    &fixture.project,
    [
      "reactant",
      "assets",
      "generate",
      "--manifest-path",
      outside.join("Cargo.toml").to_str().unwrap(),
    ],
  );
  assert!(!escaped.status.success());
  assert!(stderr(&escaped).contains("must be contained by Unity project"));

  let not_project = fixture.run_from(
    &fixture.root,
    ["reactant", "assets", "generate", "--project", "outside"],
  );
  assert!(!not_project.status.success());
  assert!(stderr(&not_project).contains("is not a Unity project"));
}

#[test]
fn asset_command_help_exposes_the_shared_selection_contract() {
  let output = Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
    .args(["reactant", "assets", "generate", "--help"])
    .output()
    .unwrap();
  assert!(output.status.success(), "{}", stderr(&output));
  let help = stdout(&output);
  for option in [
    "--project",
    "--manifest-path",
    "--features",
    "--all-features",
    "--no-default-features",
    "--browser",
    "--work-report",
  ] {
    assert!(help.contains(option), "missing {option} in:\n{help}");
  }
}

#[cfg(target_os = "macos")]
#[test]
fn empty_preview_uses_the_system_opener_without_a_renderer() {
  let fixture = Fixture::new();
  let report = fixture.root.join("preview-report.json");
  let previewed = fixture.run_from(
    &fixture.project,
    [
      "reactant",
      "assets",
      "preview",
      "--work-report",
      report.to_str().unwrap(),
    ],
  );

  assert!(previewed.status.success(), "{}", stderr(&previewed));
  let report: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
  assert_eq!(report["browserLaunches"], 0);
  assert_eq!(report["browserContextsCreated"], 0);
  assert_eq!(report["subprocessesStarted"], 4);
}

#[test]
fn declarations_are_discovered_across_modules_and_reachable_packages() {
  let fixture = Fixture::new();
  let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("battlement-reactant");
  fs::create_dir_all(fixture.project.join("rules/src/nested")).unwrap();
  fs::create_dir_all(fixture.project.join("rules/asset-pack/src")).unwrap();
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [features]\ndefault = []\nart = [\"dep:asset-pack\"]\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\nasset-pack = {{ path = \"asset-pack\", optional = true }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/src/lib.rs"),
    "mod nested;\npub fn rules() {}\n",
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/src/nested/mod.rs"),
    "battlement_reactant::asset_generator::generate! {\n\
       @background PANEL { @canvas 20px 10px; @subject 1px 1px 18px 8px; background: linear-gradient(red, blue); }\n\
     }\n",
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/asset-pack/Cargo.toml"),
    format!(
      "[package]\nname = \"asset-pack\"\nversion = \"0.2.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/asset-pack/src/lib.rs"),
    "battlement_reactant::asset_generator::generate! {\n\
       @nine-slice FRAME { @canvas 30px 18px; @subject 2px 2px 26px 14px; @slices 2px 2px 2px 2px; border: 1px dashed red; }\n\
     }\n",
  )
  .unwrap();

  let output = fixture.run_from(
    &fixture.project,
    ["reactant", "assets", "generate", "--features", "art"],
  );

  assert!(output.status.success(), "{}", stderr(&output));
  assert!(stdout(&output).contains("discovered=2 deduplicated=2"));
}

#[test]
fn git_dependency_declarations_are_discovered_with_portable_coordinates() {
  let fixture = Fixture::new();
  let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("battlement-reactant");
  let git_assets = fixture.root.join("git-assets");
  fs::create_dir_all(git_assets.join("src")).unwrap();
  fs::write(
    git_assets.join("Cargo.toml"),
    format!(
      "[package]\nname = \"git-assets\"\nversion = \"0.3.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(
    git_assets.join("src/lib.rs"),
    "battlement_reactant::asset_generator::generate! {\n\
       @background GIT_PANEL { @canvas 12px 8px; background: linear-gradient(red, blue); }\n\
     }\n",
  )
  .unwrap();
  for arguments in [
    vec!["init", "-q"],
    vec!["add", "."],
    vec![
      "-c",
      "user.name=Fixture",
      "-c",
      "user.email=fixture@example.invalid",
      "commit",
      "-qm",
      "fixture",
    ],
  ] {
    assert!(
      Command::new("git")
        .args(arguments)
        .current_dir(&git_assets)
        .status()
        .unwrap()
        .success()
    );
  }
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\ngit-assets = {{ git = {:?} }}\n",
      format!("file://{}", git_assets.display())
    ),
  )
  .unwrap();

  let output = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);

  assert!(!output.status.success());
  assert!(stderr(&output).contains("assets are stale"));
  assert!(stdout(&output).contains("asset=battlement-reactant/generated/"));
}

#[test]
fn discovery_rejects_indirection_conditionals_and_target_graph_drift() {
  let fixture = Fixture::new();
  let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("battlement-reactant");
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  let cases = [
    (
      "use battlement_reactant::asset_generator;\nasset_generator::generate! { anything }\n",
      "imports or reexports",
    ),
    (
      "fn nested() { battlement_reactant::asset_generator::generate! { anything } }\n",
      "is nested",
    ),
    (
      "#[cfg(any())]\nbattlement_reactant::asset_generator::generate! { anything }\n",
      "conditionally compiled",
    ),
    (
      "macro_rules! wrapped { () => { battlement_reactant::asset_generator::generate! { anything } } }\n",
      "macro wrapper",
    ),
  ];
  for (source, diagnostic) in cases {
    fs::write(fixture.project.join("rules/src/lib.rs"), source).unwrap();
    let output = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains(diagnostic), "{}", stderr(&output));
  }

  fs::create_dir_all(fixture.project.join("rules/host-art/src")).unwrap();
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n\
       [target.'cfg(not(target_arch = \"wasm32\"))'.dependencies]\nhost-art = {{ path = \"host-art\" }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/src/lib.rs"),
    "pub fn rules() {}\n",
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/host-art/Cargo.toml"),
    format!(
      "[package]\nname = \"host-art\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/host-art/src/lib.rs"),
    "pub fn art() {}\n",
  )
  .unwrap();
  let output = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!output.status.success());
  assert!(stderr(&output).contains("reachable declaration packages differ"));
  assert!(stderr(&output).contains("host="));
  assert!(stderr(&output).contains("WebAssembly="));
}

#[test]
fn discovery_rejects_renamed_reactant_and_nonportable_path_packages() {
  let fixture = Fixture::new();
  let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("battlement-reactant");
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nreactant = {{ package = \"battlement-reactant\", path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  let renamed = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!renamed.status.success());
  assert!(stderr(&renamed).contains("aliases battlement-reactant as reactant"));

  let outside = fixture.root.join("outside-assets");
  fs::create_dir_all(outside.join("src")).unwrap();
  fs::write(
    outside.join("Cargo.toml"),
    format!(
      "[package]\nname = \"outside-assets\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(outside.join("src/lib.rs"), "pub fn outside() {}\n").unwrap();
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
     [dependencies]\noutside-assets = { path = \"../../outside-assets\" }\n",
  )
  .unwrap();
  let nonportable = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!nonportable.status.success());
  assert!(stderr(&nonportable).contains("outside Unity project"));
  assert!(stderr(&nonportable).contains("no portable coordinate"));
}

#[test]
fn cli_discovery_preserves_shared_syntax_diagnostic_categories() {
  let fixture = Fixture::new();
  let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("battlement-reactant");
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  for (declaration, category) in [
    (
      "@background PANEL { @canvas 10px 10px; @canvas 20px 20px; background: linear-gradient(red, blue); }",
      "duplicate-statement",
    ),
    (
      "@background PANEL { @canvas 0px 10px; background: linear-gradient(red, blue); }",
      "invalid-geometry",
    ),
    (
      "@background PANEL { @canvas 10px 10px; background: red; }",
      "native-only",
    ),
  ] {
    fs::write(
      fixture.project.join("rules/src/lib.rs"),
      format!("battlement_reactant::asset_generator::generate! {{ {declaration} }}\n"),
    )
    .unwrap();
    let output = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains(category), "{}", stderr(&output));
  }
}

#[test]
fn dependency_identities_change_without_changing_public_addresses_and_duplicates_collapse() {
  let fixture = Fixture::new();
  write_asset_manifest(&fixture);
  let textures = fixture.project.join("Assets/Textures");
  fs::create_dir_all(&textures).unwrap();
  fs::copy(
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../samples/ui/Assets/Original/Signal Texture.png"),
    textures.join("panel.png"),
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/src/lib.rs"),
    "battlement_reactant::asset_generator::generate! {\n\
       @background PANEL { @canvas 20px 10px; background: unity-url(\"Assets/Textures/panel.png\"); box-shadow: 1px 2px red; }\n\
     }\n\
     battlement_reactant::asset_generator::generate! {\n\
       @background OTHER { @canvas 20px 10px; background: unity-url(\"Assets/Textures/panel.png\"); box-shadow: 1px 2px red; }\n\
     }\n",
  )
  .unwrap();

  let first = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!first.status.success());
  assert!(stderr(&first).contains("assets are stale"));
  let first_asset = identity_line(&first, "asset=");
  assert!(first_asset.contains("sources=["));
  assert_eq!(field(&first_asset, "guid=").len(), 32);
  let first_address = field(&first_asset, "asset=");
  let first_dependencies = field(&first_asset, "dependencies=");

  fs::copy(
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../samples/ui/Assets/Original/Signal Cursor.png"),
    textures.join("panel.png"),
  )
  .unwrap();
  let changed = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!changed.status.success());
  assert!(stderr(&changed).contains("assets are stale"));
  let changed_asset = identity_line(&changed, "asset=");
  assert_eq!(field(&changed_asset, "asset="), first_address);
  assert_ne!(field(&changed_asset, "dependencies="), first_dependencies);
  assert_eq!(
    stdout(&changed)
      .lines()
      .filter(|line| line.starts_with("directory="))
      .count(),
    3
  );
}

#[cfg(unix)]
#[test]
fn dependencies_validate_font_coverage_formats_and_symlink_containment() {
  use std::os::unix::fs::symlink;

  let fixture = Fixture::new();
  write_asset_manifest(&fixture);
  let fonts = fixture.project.join("Assets/Fonts");
  fs::create_dir_all(&fonts).unwrap();
  fs::copy(
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../Assets/TextMesh Pro/Fonts/LiberationSans.ttf"),
    fonts.join("face.ttf"),
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/src/lib.rs"),
    "battlement_reactant::asset_generator::generate! {\n\
       @text-image LABEL { @canvas 80px 24px; @font-file unity(\"Assets/Fonts/face.ttf\"); content: \"Hello\"; font-size: 16px; text-shadow: 1px 2px red, 2px 3px blue; }\n\
     }\n",
  )
  .unwrap();
  let valid = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!valid.status.success());
  assert!(stderr(&valid).contains("assets are stale"));
  assert!(identity_line(&valid, "asset=").contains("Assets/Fonts/face.ttf="));

  fs::write(
    fixture.project.join("rules/src/lib.rs"),
    "battlement_reactant::asset_generator::generate! {\n\
       @text-image LABEL { @canvas 80px 24px; @font-file unity(\"Assets/Fonts/face.ttf\"); content: \"\\u{10FFFF}\"; font-size: 16px; text-shadow: 1px 2px red, 2px 3px blue; }\n\
     }\n",
  )
  .unwrap();
  let uncovered = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!uncovered.status.success());
  assert!(stderr(&uncovered).contains("does not cover authored character U+10FFFF"));

  fs::copy(
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../samples/ui/Assets/Original/Signal Texture.png"),
    fonts.join("face.ttf"),
  )
  .unwrap();
  let mismatched = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!mismatched.status.success());
  assert!(stderr(&mismatched).contains("extension does not match its TrueType format"));

  let outside = fixture.root.join("outside.png");
  fs::copy(
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../samples/ui/Assets/Original/Signal Texture.png"),
    &outside,
  )
  .unwrap();
  symlink(&outside, fixture.project.join("Assets/escape.png")).unwrap();
  fs::write(
    fixture.project.join("rules/src/lib.rs"),
    "battlement_reactant::asset_generator::generate! {\n\
       @background PANEL { @canvas 20px 10px; background: unity-url(\"Assets/escape.png\"); box-shadow: 1px 2px red; }\n\
     }\n",
  )
  .unwrap();
  let escaped = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!escaped.status.success());
  assert!(stderr(&escaped).contains("resolves outside Unity project"));
}

#[test]
fn incremental_generate_reopens_only_changed_sources_and_dependencies() {
  let fixture = Fixture::new();
  write_asset_manifest(&fixture);
  let textures = fixture.project.join("Assets/Textures");
  fs::create_dir_all(&textures).unwrap();
  fs::copy(
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../samples/ui/Assets/Original/Signal Texture.png"),
    textures.join("panel.png"),
  )
  .unwrap();
  let source = fixture.project.join("rules/src/lib.rs");
  let declaration = "battlement_reactant::asset_generator::generate! {\n\
    @background PANEL { @canvas 20px 10px; @subject 3px 2px 12px 4px; background: unity-url(\"Assets/Textures/panel.png\"); box-shadow: 1px 2px red; }\n\
  }\n";
  fs::write(&source, declaration).unwrap();

  let cold_report = fixture.root.join("cold.json");
  let cold = fixture.generate_with_report(&cold_report);
  assert!(cold.status.success(), "{}", stderr(&cold));
  assert_eq!(report(&cold_report)["cargoMetadataRuns"], 2);

  let warm_report = fixture.root.join("warm.json");
  let warm = fixture.generate_with_report(&warm_report);
  assert!(warm.status.success(), "{}", stderr(&warm));
  let warm = report(&warm_report);
  for counter in [
    "cargoMetadataRuns",
    "rustSourceOpens",
    "dependencyFileOpens",
    "generatedPngOpens",
    "browserExecutableOpens",
    "subprocessesStarted",
    "browserLaunches",
    "browserContextsCreated",
    "filesWritten",
  ] {
    assert_eq!(
      warm[counter], 0,
      "unexpected warm work in {counter}: {warm}"
    );
  }

  fs::write(&source, format!("{declaration}\npub fn unrelated() {{}}\n")).unwrap();
  let source_report = fixture.root.join("source.json");
  let source_changed = fixture.generate_with_report(&source_report);
  assert!(
    source_changed.status.success(),
    "{}",
    stderr(&source_changed)
  );
  let source_work = report(&source_report);
  assert_eq!(source_work["cargoMetadataRuns"], 0);
  assert_eq!(source_work["rustSourceOpens"], 1);
  assert_eq!(source_work["dependencyFileOpens"], 0);
  assert_eq!(source_work["filesWritten"], 1);

  fs::copy(
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../samples/ui/Assets/Original/Signal Cursor.png"),
    textures.join("panel.png"),
  )
  .unwrap();
  let dependency_report = fixture.root.join("dependency.json");
  let dependency_changed = fixture.generate_with_report(&dependency_report);
  assert!(
    dependency_changed.status.success(),
    "{}",
    stderr(&dependency_changed)
  );
  let dependency_work = report(&dependency_report);
  assert_eq!(dependency_work["cargoMetadataRuns"], 0);
  assert_eq!(dependency_work["rustSourceOpens"], 0);
  assert_eq!(dependency_work["dependencyFileOpens"], 1);
  assert!(dependency_work["filesWritten"].as_u64().unwrap() > 2);

  let generated_texture = fixture.generated_root().join("textures/cached.png");
  fs::create_dir_all(generated_texture.parent().unwrap()).unwrap();
  fs::copy(
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../samples/ui/Assets/Original/Signal Texture.png"),
    &generated_texture,
  )
  .unwrap();
  let output_report = fixture.root.join("output.json");
  assert!(
    fixture
      .generate_with_report(&output_report)
      .status
      .success()
  );
  let output_work = report(&output_report);
  assert_eq!(output_work["cargoMetadataRuns"], 0);
  assert!(output_work["generatedPngOpens"].as_u64().unwrap() >= 1);
  assert!(output_work["filesWritten"].as_u64().unwrap() >= 1);

  let output_warm_report = fixture.root.join("output-warm.json");
  assert!(
    fixture
      .generate_with_report(&output_warm_report)
      .status
      .success()
  );
  let output_warm = report(&output_warm_report);
  assert_eq!(output_warm["generatedPngOpens"], 0);
  assert_eq!(output_warm["filesWritten"], 0);
}

#[test]
fn graph_inputs_and_corrupt_state_fall_back_to_full_resolution() {
  let fixture = Fixture::new();
  let first_report = fixture.root.join("first.json");
  assert!(fixture.generate_with_report(&first_report).status.success());
  let state_directory = fixture
    .project
    .join("Library/BattlementReactant/asset-generator-state");
  let default_index = fs::read_dir(&state_directory)
    .unwrap()
    .next()
    .unwrap()
    .unwrap()
    .path();
  let state: Value = serde_json::from_slice(&fs::read(&default_index).unwrap()).unwrap();
  assert_eq!(state["schema"], "battlement-reactant-asset-index-v1");
  assert!(
    state["graph"]["inputs"]
      .as_array()
      .unwrap()
      .iter()
      .any(|input| {
        input["path"]
          .as_str()
          .unwrap()
          .ends_with("/.cargo/config.toml")
          && input["fingerprint"].is_null()
      })
  );

  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    "[package]\nname = \"fixture-rules\"\nversion = \"0.1.1\"\nedition = \"2024\"\n\
     [features]\ncache = []\n",
  )
  .unwrap();
  let manifest_report = fixture.root.join("manifest.json");
  assert!(
    fixture
      .generate_with_report(&manifest_report)
      .status
      .success()
  );
  assert_eq!(report(&manifest_report)["cargoMetadataRuns"], 2);

  fs::create_dir_all(fixture.project.join("rules/.cargo")).unwrap();
  fs::write(
    fixture.project.join("rules/.cargo/config.toml"),
    "[net]\noffline = true\n",
  )
  .unwrap();
  let config_report = fixture.root.join("config.json");
  assert!(
    fixture
      .generate_with_report(&config_report)
      .status
      .success()
  );
  assert_eq!(report(&config_report)["cargoMetadataRuns"], 2);

  let lockfile = fixture.project.join("rules/Cargo.lock");
  let mut lock = fs::read_to_string(&lockfile).unwrap();
  lock.push_str("\n# fingerprint probe\n");
  fs::write(lockfile, lock).unwrap();
  let lock_report = fixture.root.join("lock.json");
  assert!(fixture.generate_with_report(&lock_report).status.success());
  assert_eq!(report(&lock_report)["cargoMetadataRuns"], 2);

  fs::write(&default_index, "{\"unknown\":true}\n").unwrap();
  let corrupt_report = fixture.root.join("corrupt.json");
  assert!(
    fixture
      .generate_with_report(&corrupt_report)
      .status
      .success()
  );
  assert_eq!(report(&corrupt_report)["cargoMetadataRuns"], 2);

  let environment_report = fixture.root.join("environment.json");
  let environment_changed = Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
    .args([
      "reactant",
      "assets",
      "generate",
      "--work-report",
      environment_report.to_str().unwrap(),
    ])
    .env("CARGO_NET_OFFLINE", "true")
    .current_dir(&fixture.project)
    .output()
    .unwrap();
  assert!(
    environment_changed.status.success(),
    "{}",
    stderr(&environment_changed)
  );
  assert_eq!(report(&environment_report)["cargoMetadataRuns"], 2);

  let feature_report = fixture.root.join("feature.json");
  let feature_changed = fixture.run_from(
    &fixture.project,
    [
      "reactant",
      "assets",
      "generate",
      "--features",
      "cache",
      "--work-report",
      feature_report.to_str().unwrap(),
    ],
  );
  assert!(
    feature_changed.status.success(),
    "{}",
    stderr(&feature_changed)
  );
  assert_eq!(report(&feature_report)["cargoMetadataRuns"], 2);
  assert_eq!(fs::read_dir(&state_directory).unwrap().count(), 2);

  let before = fs::read(&default_index).unwrap();
  fs::write(
    fixture.project.join("rules/src/lib.rs"),
    "pub fn empty() {}\npub fn unrelated() {}\n",
  )
  .unwrap();
  let check_report = fixture.root.join("readonly-check.json");
  let checked = fixture.run_from(
    &fixture.project,
    [
      "reactant",
      "assets",
      "check",
      "--work-report",
      check_report.to_str().unwrap(),
    ],
  );
  assert!(checked.status.success(), "{}", stderr(&checked));
  assert_eq!(report(&check_report)["filesWritten"], 0);
  assert_eq!(fs::read(default_index).unwrap(), before);
}

struct Fixture {
  _temporary: tempfile::TempDir,
  root: PathBuf,
  project: PathBuf,
}

impl Fixture {
  fn new() -> Self {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().to_owned();
    let project = root.join("game");
    fs::create_dir_all(project.join("Assets")).unwrap();
    fs::create_dir_all(project.join("Packages")).unwrap();
    fs::create_dir_all(project.join("ProjectSettings")).unwrap();
    fs::write(project.join("Packages/manifest.json"), "{}\n").unwrap();
    fs::write(
      project.join("ProjectSettings/ProjectVersion.txt"),
      "m_EditorVersion: fixture\n",
    )
    .unwrap();
    write_rules(&project.join("rules"));
    Self {
      _temporary: temporary,
      root,
      project,
    }
  }

  fn run_from<I, S>(&self, current: &Path, arguments: I) -> Output
  where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
  {
    Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
      .args(arguments)
      .current_dir(current)
      .output()
      .unwrap()
  }

  fn generated_root(&self) -> PathBuf {
    self.project.join("Assets/Generated/BattlementReactant")
  }

  fn generated_meta(&self) -> PathBuf {
    self
      .project
      .join("Assets/Generated/BattlementReactant.meta")
  }

  fn generate_with_report(&self, report: &Path) -> Output {
    self.run_from(
      &self.project,
      [
        "reactant",
        "assets",
        "generate",
        "--work-report",
        report.to_str().unwrap(),
      ],
    )
  }

  fn write_generated_output(&self) {
    fs::create_dir_all(self.generated_root().join("Resources")).unwrap();
    fs::write(self.generated_root().join("manifest.json"), "manifest\n").unwrap();
    fs::write(
      self
        .generated_root()
        .join("Resources/BattlementReactantAssetCatalog.json"),
      "catalog\n",
    )
    .unwrap();
    fs::write(self.generated_meta(), "metadata\n").unwrap();
  }
}

fn write_rules(directory: &Path) {
  fs::create_dir_all(directory.join("src")).unwrap();
  fs::write(
    directory.join("Cargo.toml"),
    "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
  )
  .unwrap();
  fs::write(directory.join("src/lib.rs"), "pub fn empty() {}\n").unwrap();
}

fn write_asset_manifest(fixture: &Fixture) {
  let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("battlement-reactant");
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
}

fn identity_line(output: &Output, prefix: &str) -> String {
  stdout(output)
    .lines()
    .find(|line| line.starts_with(prefix))
    .unwrap()
    .to_owned()
}

fn field<'a>(line: &'a str, prefix: &str) -> &'a str {
  line
    .split_whitespace()
    .find_map(|field| field.strip_prefix(prefix))
    .unwrap()
}

fn report(path: &Path) -> Value {
  serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn stdout(output: &Output) -> String {
  String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
  String::from_utf8_lossy(&output.stderr).into_owned()
}
