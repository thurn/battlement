use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

use battlement_tooling::{
  build_cache::{BUILD_LOG_FILE, BuildCache, SOURCE_MANIFEST_FILE},
  build_identity::{CaptureAdapter, NativeInput},
  fingerprint::GeneratedInput,
  macos_build::{
    MacosBuildOutcome, MacosBuildRequest, MacosBuildResult, MacosBuildTools, MacosStartupIdentity,
    STARTUP_IDENTITY_FILE, build_macos_player, player_executable, select_macos_player,
  },
};
use tempfile::TempDir;

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn clean_fixture_builds_launches_and_exactly_reuses_immutable_entry() {
  let fixture = Fixture::new();
  let first = build_macos_player(&fixture.request()).unwrap();
  let MacosBuildResult::Ready { build, outcome } = first else {
    panic!("clean fixture failed")
  };
  assert_eq!(outcome, MacosBuildOutcome::Created);
  assert!(build.path().join(BUILD_LOG_FILE).is_file());
  assert!(build.path().join(SOURCE_MANIFEST_FILE).is_file());
  let startup: MacosStartupIdentity =
    serde_json::from_slice(&fs::read(build.path().join(STARTUP_IDENTITY_FILE)).unwrap()).unwrap();
  assert_eq!(
    startup.build_fingerprint,
    build.metadata().identity.fingerprint
  );
  assert_eq!(
    startup.source_fingerprint,
    build.metadata().identity.source_fingerprint
  );
  assert!(startup.diagnostics);
  assert!(
    Command::new(player_executable(&build).unwrap())
      .status()
      .unwrap()
      .success()
  );
  drop(build);

  let second = build_macos_player(&fixture.request()).unwrap();
  let MacosBuildResult::Ready { build, outcome } = second else {
    panic!("exact fixture reuse failed")
  };
  assert_eq!(outcome, MacosBuildOutcome::Reused);
  assert!(player_executable(&build).unwrap().is_file());
  let transcript = fs::read_to_string(&fixture.transcript).unwrap();
  assert_eq!(transcript.matches("cargo\n").count(), 1);
  assert_eq!(transcript.matches("unity\n").count(), 1);
  assert_eq!(transcript.matches("launch\n").count(), 1);
  assert!(
    !fixture
      .path("repo/game/Assets/Resources/BattlementDittoBuildIdentity.json")
      .exists()
  );
  assert!(
    !fixture
      .path("repo/game/Assets/Plugins/macOS/libbattlement_rules.dylib")
      .exists()
  );
  assert!(
    !fixture
      .path("repo/game/Assets/Plugins/macOS/libbattlement_rules.dylib.meta")
      .exists()
  );
  assert_eq!(
    fs::read_to_string(fixture.path("repo/game/ProjectSettings/ProjectSettings.asset")).unwrap(),
    "settings before build\n"
  );
  assert!(
    !fixture
      .path("repo/game/Assets/AddressableAssetsData")
      .exists()
  );
}

#[test]
fn every_build_input_category_selects_a_distinct_entry() {
  let fixture = Fixture::new();
  let mut fingerprints = vec![fixture.build_fingerprint()];

  fixture.write(
    "repo/rules/src/lib.rs",
    "pub fn rules() { let _changed = true; }\n",
  );
  fingerprints.push(fixture.build_fingerprint());
  fixture.write(
    "repo/game/Assets/Scenes/Game.unity",
    "unity scene changed\n",
  );
  fingerprints.push(fixture.build_fingerprint());
  fixture.write(
    "repo/package/Runtime/Player.cs",
    "public class PlayerChanged {}\n",
  );
  fingerprints.push(fixture.build_fingerprint());
  let mut diagnostics = fixture.request();
  diagnostics.diagnostics = false;
  fingerprints.push(ready_fingerprint(build_macos_player(&diagnostics).unwrap()));
  let mut toolchain = fixture.request();
  toolchain.tools.xcode_version = "Xcode 99.0".to_owned();
  fingerprints.push(ready_fingerprint(build_macos_player(&toolchain).unwrap()));
  let mut generated = fixture.request();
  generated.generated_inputs[0].bytes = b"generated v2\n".to_vec();
  fingerprints.push(ready_fingerprint(build_macos_player(&generated).unwrap()));

  fingerprints.sort();
  fingerprints.dedup();
  assert_eq!(fingerprints.len(), 7);
}

#[test]
fn compilation_failure_is_terminal_retains_full_log_and_never_launches() {
  let fixture = Fixture::new();
  fixture.write("repo/rules/src/lib.rs", "COMPILATION_FAILURE\n");
  let result = build_macos_player(&fixture.request()).unwrap();
  let MacosBuildResult::Failed(failure) = result else {
    panic!("broken Rust fixture unexpectedly built")
  };
  assert_eq!(failure.phase, "rust");
  assert_eq!(failure.error_ids, ["E0308"]);
  let log = fs::read_to_string(&failure.log_path).unwrap();
  assert!(log.contains("complete compiler prelude"));
  assert!(log.contains("error[E0308]"));
  let transcript = fs::read_to_string(&fixture.transcript).unwrap();
  assert_eq!(transcript.matches("cargo\n").count(), 1);
  assert!(!transcript.contains("unity\n"));
  assert!(!transcript.contains("launch\n"));

  assert!(matches!(
    build_macos_player(&fixture.request()).unwrap(),
    MacosBuildResult::Failed(_)
  ));
}

#[test]
fn no_build_explains_the_nearest_cached_source() {
  let fixture = Fixture::new();
  let previous = ready_fingerprint(build_macos_player(&fixture.request()).unwrap());
  fixture.write(
    "repo/rules/src/lib.rs",
    "pub fn rules() { let _changed = true; }\n",
  );

  let MacosBuildResult::Required {
    identity,
    nearest: Some(nearest),
  } = select_macos_player(&fixture.request(), false).unwrap()
  else {
    panic!("changed fixture did not report a required build")
  };

  assert_ne!(identity.fingerprint, previous);
  assert_eq!(nearest.fingerprint, previous);
  assert_eq!(nearest.changed_inputs, ["source"]);
  assert_eq!(nearest.changed_paths, ["rules/src/lib.rs"]);
  assert!(nearest.added_paths.is_empty());
  assert!(nearest.removed_paths.is_empty());
}

fn ready_fingerprint(result: MacosBuildResult) -> String {
  let MacosBuildResult::Ready { build, .. } = result else {
    panic!("fixture build failed")
  };
  build.metadata().identity.fingerprint.clone()
}

struct Fixture {
  root: TempDir,
  transcript: PathBuf,
  cargo: PathBuf,
  unity: PathBuf,
}

impl Fixture {
  fn new() -> Self {
    let root = TempDir::new().unwrap();
    let fixture = Self {
      transcript: root.path().join("transcript.txt"),
      cargo: root.path().join("tools/cargo"),
      unity: root.path().join("tools/Unity"),
      root,
    };
    fixture.write("repo/Cargo.lock", "version = 4\n");
    fixture.write("repo/game/Assets/Scenes/Game.unity", "unity scene\n");
    fixture.write(
      "repo/game/Packages/manifest.json",
      r#"{"dependencies":{"com.battlement.client":"file:../../package"}}"#,
    );
    fixture.write("repo/game/Packages/packages-lock.json", "{}\n");
    fixture.write(
      "repo/game/ProjectSettings/ProjectVersion.txt",
      "m_EditorVersion: 6000.0.56f1\n",
    );
    fixture.write(
      "repo/game/ProjectSettings/ProjectSettings.asset",
      "settings before build\n",
    );
    fixture.write(
      "repo/package/package.json",
      "{\"name\":\"com.battlement.client\"}\n",
    );
    fixture.write("repo/package/Runtime/Player.cs", "public class Player {}\n");
    fixture.write(
      "repo/rules/Cargo.toml",
      "[package]\nname = \"rules\"\nversion = \"0.1.0\"\n[lib]\nname = \"battlement_rules\"\ncrate-type = [\"cdylib\"]\n[workspace]\n",
    );
    fixture.write("repo/rules/src/lib.rs", "pub fn rules() {}\n");
    fixture.write_executable("tools/cargo", &fixture.cargo_script());
    fixture.write_executable("tools/Unity", &fixture.unity_script());
    fixture
  }

  fn request(&self) -> MacosBuildRequest {
    MacosBuildRequest {
      repository: self.path("repo"),
      unity_project: self.path("repo/game"),
      rust_manifest: self.path("repo/rules/Cargo.toml"),
      scene: self.path("repo/game/Assets/Scenes/Game.unity"),
      suite: "fixture".to_owned(),
      diagnostics: true,
      generated_inputs: vec![GeneratedInput {
        generator: "fixture-generator".to_owned(),
        version: "1".to_owned(),
        name: "bindings.cs".to_owned(),
        bytes: b"generated v1\n".to_vec(),
      }],
      native_inputs: vec![NativeInput {
        name: "battlement-native-abi".to_owned(),
        sha256: HASH.to_owned(),
      }],
      capture_adapter: CaptureAdapter {
        name: "unity-async-readback-png".to_owned(),
        version: "1".to_owned(),
      },
      tools: MacosBuildTools {
        unity_editor: self.unity.clone(),
        unity_version: "6000.0.56f1".to_owned(),
        cargo: self.cargo.clone(),
        cargo_version: "cargo 1.85.0".to_owned(),
        rustc_version: "rustc 1.85.0".to_owned(),
        architecture: "arm64".to_owned(),
        xcode_version: "Xcode 16.2".to_owned(),
        sdk_version: "macOS 15.2".to_owned(),
      },
      resource_slots: self.root.path().join("resource-slots"),
      cache: BuildCache::open(self.root.path().join("build-cache"), 1024 * 1024).unwrap(),
    }
  }

  fn build_fingerprint(&self) -> String {
    ready_fingerprint(build_macos_player(&self.request()).unwrap())
  }

  fn path(&self, relative: &str) -> PathBuf {
    self.root.path().join(relative)
  }

  fn write(&self, relative: &str, contents: &str) {
    let path = self.path(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
  }

  fn write_executable(&self, relative: &str, contents: &str) {
    self.write(relative, contents);
    let path = self.path(relative);
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
  }

  fn cargo_script(&self) -> String {
    format!(
      r#"#!/bin/sh
set -eu
printf 'cargo\n' >> '{}'
manifest=''
target=''
target_dir=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifest-path) manifest="$2"; shift 2 ;;
    --target) target="$2"; shift 2 ;;
    --target-dir) target_dir="$2"; shift 2 ;;
    *) shift ;;
  esac
done
printf 'complete compiler prelude\n'
if grep -q COMPILATION_FAILURE "$(dirname "$manifest")/src/lib.rs"; then
  printf 'error[E0308]: fixture compilation failed\n' >&2
  exit 1
fi
mkdir -p "$target_dir/$target/release"
printf 'native rust engine\n' > "$target_dir/$target/release/libbattlement_rules.dylib"
"#,
      self.transcript.display()
    )
  }

  fn unity_script(&self) -> String {
    format!(
      r#"#!/bin/sh
set -eu
printf 'unity\n' >> '{}'
project=''
method=''
log=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -projectPath) project="$2"; shift 2 ;;
    -executeMethod) method="$2"; shift 2 ;;
    -logFile) log="$2"; shift 2 ;;
    *) shift ;;
  esac
done
[ "$method" = 'Battlement.Editor.BattlementDittoBuild.BuildMacos' ]
[ -f "$project/Assets/Plugins/macOS/libbattlement_rules.dylib" ]
[ -f "$project/Assets/Resources/BattlementDittoBuildIdentity.json" ]
printf 'settings changed by Unity\n' > "$project/ProjectSettings/ProjectSettings.asset"
mkdir -p "$project/Assets/AddressableAssetsData"
printf 'generated catalog\n' > "$project/Assets/AddressableAssetsData/catalog.txt"
printf 'complete Unity build log\n' > "$log"
mkdir -p "$BATTLEMENT_DITTO_BUILD_PATH/Contents/MacOS"
printf '#!/bin/sh\nprintf "launch\\n" >> "{}"\n' > "$BATTLEMENT_DITTO_BUILD_PATH/Contents/MacOS/BattlementDitto"
chmod +x "$BATTLEMENT_DITTO_BUILD_PATH/Contents/MacOS/BattlementDitto"
"#,
      self.transcript.display(),
      self.transcript.display()
    )
  }
}
