use std::{fs, process::Command};

use crate::{
  config,
  selection::{self, Options},
  wire::result::ResultCommand,
};

use super::{baseline_inputs, selection_has_screenshots};

#[test]
fn assertion_only_selection_never_reads_the_baseline_lock() {
  let temporary = tempfile::tempdir().unwrap();
  let repository = temporary.path().join("repository");
  fs::create_dir_all(repository.join("Assets/Scenes")).unwrap();
  fs::create_dir_all(repository.join("ProjectSettings")).unwrap();
  fs::create_dir_all(repository.join("rules/src")).unwrap();
  fs::write(repository.join("Assets/Scenes/Game.unity"), "").unwrap();
  fs::write(
    repository.join("ProjectSettings/ProjectVersion.txt"),
    "m_EditorVersion: 6000.0.56f1\n",
  )
  .unwrap();
  fs::write(
    repository.join("rules/Cargo.toml"),
    "[package]\nname='fixture'\nversion='0.1.0'\n",
  )
  .unwrap();
  fs::write(repository.join("ditto.toml"), SUITE).unwrap();
  fs::write(repository.join("ditto.lock"), "not valid json\n").unwrap();
  assert!(
    Command::new("git")
      .args(["init", "--quiet"])
      .current_dir(&repository)
      .status()
      .unwrap()
      .success()
  );

  let suite = config::load(Some(&repository.join("ditto.toml"))).unwrap();
  let selection = selection::resolve(&suite, &Options::default()).unwrap();
  assert!(!selection_has_screenshots(&selection));
  let baseline = baseline_inputs(&suite, ResultCommand::Run, false).unwrap();
  assert!(baseline.manifest.is_none());
  assert!(baseline.store.is_none());
  assert!(baseline.lock_sha256.is_none());
  assert!(baseline_inputs(&suite, ResultCommand::Run, true).is_err());
}

const SUITE: &str = r#"name = "fixture"
default_profile = "macos-local"

[player]
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[baseline]
kind = "filesystem"
namespace = "fixture"
root = "baselines"

[profiles.macos-local]
target = "macos"
display = { width = 1280, height = 720, scale = 1.0 }

[[scenarios]]
name = "assertion only"

[[scenarios.steps]]
assert = { object = "00000000-0000-0000-0000-000000000001", state = "exists" }
"#;
