use std::{fs, path::PathBuf, process::Command};

use battlement_tooling::fingerprint::{
  CaseSensitivity, FingerprintRequest, GeneratedInput, SourceManifest,
};
use tempfile::TempDir;

#[test]
fn manifest_covers_build_inputs_and_round_trips() {
  let fixture = Fixture::new();
  let manifest = fixture.manifest(CaseSensitivity::Sensitive);

  assert_eq!(
    manifest
      .entries
      .iter()
      .map(|entry| entry.path.as_str())
      .collect::<Vec<_>>(),
    [
      ".cargo/config.toml",
      "@generated/cbindgen/0.27/native.h",
      "Cargo.lock",
      "engine/Cargo.toml",
      "engine/src/lib.rs",
      "game/Assets/player.txt",
      "game/Packages/manifest.json",
      "game/Packages/packages-lock.json",
      "game/ProjectSettings/ProjectVersion.txt",
      "local-unity/Runtime/code.cs",
      "local-unity/package.json",
      "rules/Cargo.toml",
      "rules/src/lib.rs",
    ]
  );
  assert_eq!(
    manifest.fingerprint,
    "952d39094a200daad69dfba90cdc87bc69da054ea0701a58174faec7e8d888c9"
  );
  let retained = fixture.root.path().join("retained/source-manifest.json");
  fs::create_dir_all(retained.parent().unwrap()).unwrap();
  manifest.write(&retained).unwrap();
  assert_eq!(SourceManifest::read(&retained).unwrap(), manifest);
}

#[test]
fn manifest_diff_names_every_relevant_working_tree_change() {
  let mut fixture = Fixture::new();
  let original = fixture.manifest(CaseSensitivity::Sensitive);
  fixture.write("game/Assets/player.txt", "changed player\n");
  fixture.write("rules/src/untracked.rs", "pub struct Untracked;\n");
  fs::remove_file(fixture.path("engine/src/lib.rs")).unwrap();
  fixture.generated = vec![GeneratedInput {
    generator: "cbindgen".to_owned(),
    version: "0.28".to_owned(),
    name: "native.h".to_owned(),
    bytes: b"generated v2\n".to_vec(),
  }];
  let changed = fixture.manifest(CaseSensitivity::Sensitive);

  assert_ne!(changed.fingerprint, original.fingerprint);
  assert_eq!(
    changed.difference(&original),
    battlement_tooling::fingerprint::ManifestDiff {
      added: vec![
        "@generated/cbindgen/0.28/native.h".to_owned(),
        "rules/src/untracked.rs".to_owned(),
      ],
      removed: vec![
        "@generated/cbindgen/0.27/native.h".to_owned(),
        "engine/src/lib.rs".to_owned(),
      ],
      changed: vec!["game/Assets/player.txt".to_owned()],
    }
  );
}

#[test]
fn manifest_tracks_modes_local_dependencies_and_working_bytes() {
  let fixture = Fixture::new();
  let original = fixture.manifest(CaseSensitivity::Sensitive);

  fixture.write("local-unity/Runtime/code.cs", "changed unity package\n");
  fixture.write("engine/src/lib.rs", "pub fn changed() {}\n");
  let changed = fixture.manifest(CaseSensitivity::Sensitive);
  assert_eq!(
    changed.difference(&original).changed,
    ["engine/src/lib.rs", "local-unity/Runtime/code.cs"]
  );

  let before_rewrite = changed.fingerprint.clone();
  let player = fixture.path("game/Assets/player.txt");
  let bytes = fs::read(&player).unwrap();
  fs::write(&player, bytes).unwrap();
  assert_eq!(
    fixture.manifest(CaseSensitivity::Sensitive).fingerprint,
    before_rewrite
  );

  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(&player).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&player, permissions).unwrap();
    let mode_changed = fixture.manifest(CaseSensitivity::Sensitive);
    assert_ne!(mode_changed.fingerprint, before_rewrite);
    assert_eq!(
      mode_changed.difference(&changed).changed,
      ["game/Assets/player.txt"]
    );
  }
}

#[test]
fn normative_cache_changes_are_excluded_but_source_named_build_is_not() {
  let fixture = Fixture::new();
  let original = fixture.manifest(CaseSensitivity::Sensitive);
  fixture.write("rules/target/cache.bin", "new cache bytes\n");
  fixture.write("rules/.ditto/run/result.json", "new result\n");
  assert_eq!(
    fixture.manifest(CaseSensitivity::Sensitive).fingerprint,
    original.fingerprint
  );

  fixture.write("game/Assets/Build/meaningful.asset", "source bytes\n");
  let changed = fixture.manifest(CaseSensitivity::Sensitive);
  assert_eq!(
    changed.difference(&original).added,
    ["game/Assets/Build/meaningful.asset"]
  );
}

#[test]
fn git_identity_and_index_state_do_not_replace_working_bytes() {
  let fixture = Fixture::new();
  fixture.git(&["init"]);
  fixture.git(&["config", "user.email", "ditto@example.invalid"]);
  fixture.git(&["config", "user.name", "Ditto Fixture"]);
  fixture.git(&["add", "."]);
  fixture.git(&["commit", "-m", "fixture"]);

  fixture.write("game/Assets/player.txt", "staged bytes\n");
  fixture.git(&["add", "game/Assets/player.txt"]);
  let staged = fixture.manifest(CaseSensitivity::Sensitive);
  fixture.git(&["commit", "-m", "new identity"]);
  assert_eq!(fixture.manifest(CaseSensitivity::Sensitive), staged);

  fixture.write("game/Assets/player.txt", "unstaged bytes\n");
  let unstaged = fixture.manifest(CaseSensitivity::Sensitive);
  assert_ne!(unstaged.fingerprint, staged.fingerprint);
  fixture.git(&["add", "game/Assets/player.txt"]);
  assert_eq!(fixture.manifest(CaseSensitivity::Sensitive), unstaged);
}

#[test]
fn case_policy_rejects_only_ambiguous_paths() {
  let mut fixture = Fixture::new();
  assert_eq!(
    fixture.manifest(CaseSensitivity::Sensitive),
    fixture.manifest(CaseSensitivity::Insensitive)
  );
  fixture.generated.push(GeneratedInput {
    generator: "cbindgen".to_owned(),
    version: "0.27".to_owned(),
    name: "Native.h".to_owned(),
    bytes: b"other case\n".to_vec(),
  });

  SourceManifest::build(&fixture.request(CaseSensitivity::Sensitive)).unwrap();
  let error = SourceManifest::build(&fixture.request(CaseSensitivity::Insensitive)).unwrap_err();
  assert!(error.to_string().contains("collide by case"));
}

#[cfg(unix)]
#[test]
fn symlinks_are_bounded_and_directory_links_are_rejected() {
  use std::os::unix::fs::symlink;

  let fixture = Fixture::new();
  let internal = fixture.path("game/Assets/internal.txt");
  symlink(fixture.path("engine/src/lib.rs"), &internal).unwrap();
  assert!(
    fixture
      .manifest(CaseSensitivity::Sensitive)
      .entries
      .iter()
      .any(|entry| entry.path == "game/Assets/internal.txt")
  );

  fs::remove_file(&internal).unwrap();
  symlink(fixture.path("engine/src"), &internal).unwrap();
  let directory_error =
    SourceManifest::build(&fixture.request(CaseSensitivity::Sensitive)).unwrap_err();
  assert!(directory_error.to_string().contains("directory symlinks"));

  fs::remove_file(&internal).unwrap();
  let external = TempDir::new().unwrap();
  fs::write(external.path().join("outside.txt"), "outside\n").unwrap();
  symlink(external.path().join("outside.txt"), &internal).unwrap();
  let escape_error =
    SourceManifest::build(&fixture.request(CaseSensitivity::Sensitive)).unwrap_err();
  assert!(escape_error.to_string().contains("symlink escapes"));

  fs::remove_file(&internal).unwrap();
  let linked_package = fixture.path("linked-package");
  symlink(fixture.path("local-unity"), &linked_package).unwrap();
  fixture.write(
    "game/Packages/manifest.json",
    r#"{"dependencies":{"com.example.local":"file:../../linked-package"}}"#,
  );
  let package_error =
    SourceManifest::build(&fixture.request(CaseSensitivity::Sensitive)).unwrap_err();
  assert!(package_error.to_string().contains("directory symlink"));
}

struct Fixture {
  root: TempDir,
  generated: Vec<GeneratedInput>,
}

impl Fixture {
  fn new() -> Self {
    let fixture = Self {
      root: TempDir::new().unwrap(),
      generated: vec![GeneratedInput {
        generator: "cbindgen".to_owned(),
        version: "0.27".to_owned(),
        name: "native.h".to_owned(),
        bytes: b"generated v1\n".to_vec(),
      }],
    };
    fixture.write("Cargo.lock", "version = 4\n");
    fixture.write(".cargo/config.toml", "[build]\nincremental = false\n");
    fixture.write("game/Assets/player.txt", "player\n");
    fixture.write(
      "game/Packages/manifest.json",
      r#"{"dependencies":{"com.example.local":"file:../../local-unity","com.unity.registry":"1.0.0"}}"#,
    );
    fixture.write("game/Packages/packages-lock.json", "{}\n");
    fixture.write(
      "game/ProjectSettings/ProjectVersion.txt",
      "m_EditorVersion: 6000.5.8f1\n",
    );
    fixture.write(
      "local-unity/package.json",
      "{\"name\":\"com.example.local\"}\n",
    );
    fixture.write("local-unity/Runtime/code.cs", "public class Local {}\n");
    fixture.write(
      "rules/Cargo.toml",
      "[package]\nname = \"rules\"\nversion = \"0.1.0\"\n[dependencies]\nengine = { path = \"../engine\" }\n[workspace]\n",
    );
    fixture.write("rules/src/lib.rs", "pub fn rules() {}\n");
    fixture.write(
      "engine/Cargo.toml",
      "[package]\nname = \"engine\"\nversion = \"0.1.0\"\n",
    );
    fixture.write("engine/src/lib.rs", "pub fn engine() {}\n");
    fixture.write("rules/target/cache.bin", "ignored cache\n");
    fixture.write("rules/.ditto/run/result.json", "ignored result\n");
    fixture
  }

  fn request(&self, case_sensitivity: CaseSensitivity) -> FingerprintRequest {
    FingerprintRequest {
      repository: self.root.path().to_owned(),
      unity_project: self.path("game"),
      rust_manifest: self.path("rules/Cargo.toml"),
      generated_inputs: self.generated.clone(),
      case_sensitivity,
    }
  }

  fn manifest(&self, case_sensitivity: CaseSensitivity) -> SourceManifest {
    SourceManifest::build(&self.request(case_sensitivity)).unwrap()
  }

  fn path(&self, relative: &str) -> PathBuf {
    self.root.path().join(relative)
  }

  fn write(&self, relative: &str, contents: &str) {
    let path = self.path(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(self.path(relative), contents).unwrap();
  }

  fn git(&self, arguments: &[&str]) {
    let output = Command::new("git")
      .args(arguments)
      .current_dir(self.root.path())
      .output()
      .unwrap();
    assert!(
      output.status.success(),
      "git failed: {}",
      String::from_utf8_lossy(&output.stderr)
    );
  }
}
