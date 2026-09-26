use std::{
  fs,
  os::unix::fs::PermissionsExt,
  path::Path,
  process::{Command, Output},
};

use tempfile::TempDir;

const SUITE: &str = r#"name = "build-fixture"
default_profile = "macos"
[player]
unity_project = "game"
scene = "game/Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"
[profiles.macos]
target = "macos"
display = { width = 640, height = 360, scale = 1.0 }
[[scenarios]]
name = "unused"
motion = "controlled"
[[scenarios.steps]]
advance = { frames = 1 }
"#;

const TOOL: &str = r#"#!/usr/bin/env python3
import os
from pathlib import Path
import sys
args = sys.argv[1:]
tool = Path(sys.argv[0]).name
if tool in ('rustc', 'xcrun', 'xcodebuild', 'odiff') or '--version' in args or '-version' in args:
    print({'unity': '6000.0.56f1', 'cargo': 'cargo 1.98.1', 'rustc': 'rustc 1.98.1', 'xcrun': '15.2', 'xcodebuild': 'Xcode 26.0', 'odiff': 'odiff 4.5.0'}[tool])
    sys.exit(0)
def argument(name):
    return args[args.index(name) + 1]
if tool == 'cargo':
    phase = 'rules'
else:
    phase = 'shell' if argument('-executeMethod').endswith('BuildMacosShell') else 'content'
diagnostic = f'retained {phase} diagnostic'
if os.environ.get('FIXTURE_FAILURE_PHASE') == phase:
    diagnostic = ('error[E0308]: ' if phase == 'rules' else 'error CS0619: ') + diagnostic
    print(diagnostic, file=sys.stderr)
    if tool == 'unity':
        Path(argument('-logFile')).write_text(diagnostic)
    sys.exit(1)
if phase == 'rules':
    artifact = Path(argument('--target-dir')) / argument('--target') / 'release/libbattlement_rules.dylib'
    artifact.parent.mkdir(parents=True, exist_ok=True)
    artifact.write_text('rules artifact')
elif phase == 'shell':
    artifact = Path(os.environ['BATTLEMENT_DITTO_BUILD_PATH']) / 'Contents/MacOS/BattlementDitto'
    artifact.parent.mkdir(parents=True, exist_ok=True)
    artifact.write_text('#!/bin/sh\nexit 0\n')
    artifact.chmod(0o755)
else:
    content = Path(os.environ['BATTLEMENT_DITTO_CONTENT_PATH'])
    content.mkdir(parents=True, exist_ok=True)
    (content / 'settings.json').write_text('{}')
if tool == 'unity':
    Path(argument('-logFile')).write_text(diagnostic)
"#;

struct Fixture {
  root: TempDir,
}

#[test]
fn build_failures_report_retained_phase_logs_and_success_still_reuses() {
  let fixture = Fixture::new();
  for phase in ["rules", "shell", "content"] {
    let failed = fixture.build(phase, true);
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert_eq!(failed.status.code(), Some(2), "{stderr}");
    let result: serde_json::Value = serde_json::from_slice(&failed.stdout)
      .unwrap_or_else(|error| panic!("missing build failure JSON: {error}; {stderr}"));
    assert_eq!(result["disposition"], "failed");
    assert_eq!(result["phase"], phase);
    assert_eq!(result["suite"], "build-fixture");
    assert_eq!(result["profile"], "macos");
    assert_eq!(result["build_fingerprint"].as_str().unwrap().len(), 64);
    assert_eq!(result["source_fingerprint"].as_str().unwrap().len(), 64);
    assert!(
      result["message"]
        .as_str()
        .unwrap()
        .contains(&format!("{phase} build exited"))
    );
    let log = result["log_path"].as_str().unwrap();
    assert!(
      fs::read_to_string(log)
        .unwrap()
        .contains(&format!("retained {phase} diagnostic"))
    );
    assert!(stderr.contains(log));
    assert!(stderr.contains(result["message"].as_str().unwrap()));
    assert_eq!(
      serde_json::from_slice::<serde_json::Value>(
        &fs::read(fixture.root.path().join("result.json")).unwrap()
      )
      .unwrap(),
      result,
    );
    if phase == "rules" {
      assert_eq!(result["error_ids"], serde_json::json!(["E0308"]));
    }

    let human = fixture.build(phase, false);
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert_eq!(human.status.code(), Some(2));
    assert!(stderr.contains(&format!("Build phase: {phase}")));
    assert!(stderr.contains("Build fingerprint: "));
    let log = stderr
      .lines()
      .find_map(|line| line.strip_prefix("Build log: "))
      .unwrap();
    assert!(
      fs::read_to_string(log)
        .unwrap()
        .contains(&format!("retained {phase} diagnostic"))
    );
  }
  for disposition in ["created", "reused"] {
    let success = fixture.build("", true);
    assert!(
      success.status.success(),
      "{}",
      String::from_utf8_lossy(&success.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&success.stdout).unwrap();
    assert_eq!(result["disposition"], disposition);
    assert!(Path::new(result["application_path"].as_str().unwrap()).exists());
    assert!(result["components"]["rules"].is_string());
  }
}

impl Fixture {
  fn new() -> Self {
    let fixture = Self {
      root: tempfile::tempdir().unwrap(),
    };
    for (path, contents) in [
      ("repo/contracts/native-abi.json", "{}\n"),
      ("repo/contracts/wire-contract.json", "{}\n"),
      ("repo/game/Assets/Scenes/Game.unity", "scene\n"),
      (
        "repo/game/Packages/manifest.json",
        "{\"dependencies\":{\"com.battlement.client\":\"file:../../package\"}}\n",
      ),
      ("repo/game/Packages/packages-lock.json", "{}\n"),
      (
        "repo/game/ProjectSettings/ProjectVersion.txt",
        "m_EditorVersion: 6000.0.56f1\n",
      ),
      (
        "repo/game/ProjectSettings/ProjectSettings.asset",
        "settings\n",
      ),
      (
        "repo/package/package.json",
        "{\"name\":\"com.battlement.client\"}\n",
      ),
      ("repo/package/Runtime/Player.cs", "public class Player {}\n"),
      (
        "repo/rules/Cargo.toml",
        "[package]\nname='rules'\nversion='0.1.0'\n[lib]\nname='battlement_rules'\ncrate-type=['cdylib']\n[workspace]\n",
      ),
      ("repo/rules/src/lib.rs", "pub fn rules() {}\n"),
      ("repo/Cargo.lock", "version = 4\n"),
      ("repo/ditto.toml", SUITE),
      (
        "repo/scripts/unity_transaction.py",
        include_str!("../../../../scripts/unity_transaction.py"),
      ),
      (
        "repo/scripts/unity_metadata.py",
        include_str!("../../../../scripts/unity_metadata.py"),
      ),
      (
        "repo/scripts/process_priority.py",
        include_str!("../../../../scripts/process_priority.py"),
      ),
    ] {
      let path = fixture.root.path().join(path);
      fs::create_dir_all(path.parent().unwrap()).unwrap();
      fs::write(path, contents).unwrap();
    }
    fs::create_dir(fixture.root.path().join("tools")).unwrap();
    for tool in ["cargo", "rustc", "unity", "xcrun", "xcodebuild", "odiff"] {
      let path = fixture.root.path().join("tools").join(tool);
      fs::write(&path, TOOL).unwrap();
      fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    for arguments in [
      vec!["init", "--quiet"],
      vec!["add", "."],
      vec![
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "--quiet",
        "-m",
        "fixture",
      ],
    ] {
      assert!(
        Command::new("git")
          .args(arguments)
          .current_dir(fixture.root.path().join("repo"))
          .status()
          .unwrap()
          .success()
      );
    }
    fixture
  }

  fn build(&self, phase: &str, json: bool) -> Output {
    let tools = self.root.path().join("tools");
    let mut command = Command::new(env!("CARGO_BIN_EXE_rt"));
    command
      .args([
        "ditto",
        "--config",
        "ditto.toml",
        "build",
        "--profile",
        "macos",
        "--output",
      ])
      .arg(self.root.path().join("result.json"))
      .env(
        "PATH",
        format!("{}:{}", tools.display(), std::env::var("PATH").unwrap()),
      )
      .env("UNITY_EDITOR", tools.join("unity"))
      .env("DITTO_ODIFF_PATH", tools.join("odiff"))
      .env(
        "DITTO_CACHE_ROOT",
        self.root.path().join(format!("cache-{phase}")),
      )
      .env("BATTLEMENT_RESOURCE_SLOTS", self.root.path().join("slots"))
      .env("FIXTURE_FAILURE_PHASE", phase)
      .current_dir(self.root.path().join("repo"));
    if json {
      command.arg("--json");
    }
    command.output().unwrap()
  }
}
