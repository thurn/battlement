use battlement_ditto::cli::{CleanCommand, Command, StorageCommand, parse_from};
use std::{
  fs,
  io::Write,
  os::unix::fs::PermissionsExt,
  path::Path,
  process::{Command as ProcessCommand, Stdio},
};

#[test]
fn core_command_matrix_parses_complete_options() {
  assert!(matches!(
    parse_from(["ditto", "build", "--profile", "macos", "--json"])
      .unwrap()
      .command,
    Command::Build(options) if options.profile.as_deref() == Some("macos") && options.json
  ));
  let run = parse_from([
    "ditto",
    "--config",
    "suite.toml",
    "run",
    "menu*",
    "--scenario",
    "settings",
    "--exclude",
    "slow*",
    "--profile",
    "macos-local",
    "--allow-empty",
    "--update",
    "--bail=3",
    "--no-build",
    "--json",
    "--output",
    "result.json",
    "--review",
  ])
  .unwrap();
  let Command::Run(run) = run.command else {
    panic!("run command was not parsed")
  };
  assert_eq!(run.selection.includes, ["menu*", "settings"]);
  assert_eq!(run.selection.excludes, ["slow*"]);
  assert_eq!(run.selection.profile.as_deref(), Some("macos-local"));
  assert!(run.selection.allow_empty && run.update && run.no_build && run.json && run.review);
  assert_eq!(run.bail_after, Some(3));
  assert_eq!(run.output.unwrap().to_str(), Some("result.json"));

  let capture = parse_from([
    "ditto",
    "capture",
    "--fragment=-",
    "--bail",
    "--no-build",
    "--json",
    "--output=capture.json",
    "--review",
  ])
  .unwrap();
  let Command::Capture(capture) = capture.command else {
    panic!("capture command was not parsed")
  };
  assert_eq!(capture.fragment.unwrap().to_str(), Some("-"));
  assert_eq!(capture.bail_after, Some(1));
  assert!(capture.no_build && capture.json && capture.review);

  let Command::Run(watch) = parse_from(["ditto", "run", "-w"]).unwrap().command else {
    panic!("watch run was not parsed")
  };
  assert!(watch.watch);
  let Command::Capture(watch) =
    parse_from(["ditto", "capture", "--fragment=cycle.toml", "--watch"])
      .unwrap()
      .command
  else {
    panic!("watch capture was not parsed")
  };
  assert!(watch.watch);

  assert!(matches!(
    parse_from(["ditto", "review", "39e15c94-f631-454e-86a0-2659299d1637"])
      .unwrap()
      .command,
    Command::Review(options)
      if options.run.as_deref() == Some("39e15c94-f631-454e-86a0-2659299d1637")
  ));

  assert!(matches!(
    parse_from([
      "ditto",
      "gallery",
      "--profile",
      "macos",
      "--port",
      "48123",
      "--no-open"
    ])
    .unwrap()
    .command,
    Command::Gallery(options)
      if options.profile.as_deref() == Some("macos")
        && options.port == Some(48123)
        && options.no_open
  ));

  assert!(matches!(
    parse_from(["ditto", "fetch", "--all"]).unwrap().command,
    Command::Fetch(options) if options.all
  ));
  assert!(matches!(
    parse_from(["ditto", "list", "menu*"]).unwrap().command,
    Command::List(options) if options.includes == ["menu*"]
  ));
  assert!(matches!(
    parse_from(["ditto", "doctor", "--profile", "ios"]).unwrap().command,
    Command::Doctor(options) if options.profile.as_deref() == Some("ios")
  ));
  assert!(matches!(
    parse_from(["ditto", "clean", "runs", "--global"])
      .unwrap()
      .command,
    Command::Clean(CleanCommand::Runs { global: true })
  ));
  assert!(matches!(
    parse_from(["ditto", "clean", "builds"]).unwrap().command,
    Command::Clean(CleanCommand::Builds { global: false })
  ));
  assert!(matches!(
    parse_from(["ditto", "clean", "baselines"]).unwrap().command,
    Command::Clean(CleanCommand::Baselines)
  ));
  assert!(matches!(
    parse_from(["ditto", "clean", "storage", "--apply"])
      .unwrap()
      .command,
    Command::Clean(CleanCommand::Storage { apply: true })
  ));
  assert!(matches!(
    parse_from(["ditto", "storage", "publish"]).unwrap().command,
    Command::Storage(StorageCommand::Publish)
  ));
}

#[test]
fn unavailable_and_ambiguous_forms_are_rejected() {
  for arguments in [
    vec!["ditto", "run", "--watch", "--update"],
    vec!["ditto", "capture", "--update"],
    vec!["ditto", "fetch", "--all", "menu*"],
    vec!["ditto", "fetch", "--all", "--profile", "macos"],
    vec!["ditto", "clean", "baselines", "--global"],
  ] {
    assert!(parse_from(arguments.clone()).is_err(), "{arguments:?}");
  }
}

#[test]
fn parser_output_uses_the_correct_process_stream_and_exit_code() {
  let mut stdout = Vec::new();
  let mut stderr = Vec::new();
  assert_eq!(
    battlement_ditto::process_from(["ditto", "--help"], &mut stdout, &mut stderr),
    0
  );
  assert!(String::from_utf8(stdout).unwrap().contains("Usage: ditto"));
  assert!(stderr.is_empty());

  let mut stdout = Vec::new();
  let mut stderr = Vec::new();
  assert_eq!(
    battlement_ditto::process_from(["ditto", "unknown"], &mut stdout, &mut stderr),
    2
  );
  assert!(stdout.is_empty());
  assert!(
    String::from_utf8(stderr)
      .unwrap()
      .contains("unrecognized subcommand")
  );
}

#[test]
fn capture_json_is_baseline_neutral_and_keeps_prose_on_stderr() {
  let temporary = tempfile::tempdir().unwrap();
  let repository = temporary.path().join("repository");
  let baseline = temporary.path().join("baseline-store");
  let cache = temporary.path().join("cache");
  fs::create_dir_all(repository.join("Assets/Scenes")).unwrap();
  fs::create_dir_all(repository.join("ProjectSettings")).unwrap();
  fs::create_dir_all(repository.join("rules/src")).unwrap();
  fs::create_dir_all(&baseline).unwrap();
  fs::write(baseline.join("sentinel"), "unchanged").unwrap();
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
  fs::write(
    repository.join("ditto.toml"),
    SUITE.replace("$BASELINE", &baseline.to_string_lossy()),
  )
  .unwrap();
  assert!(
    ProcessCommand::new("git")
      .args(["init", "--quiet"])
      .current_dir(&repository)
      .status()
      .unwrap()
      .success()
  );
  let output_path = temporary.path().join("copy.json");
  let output = ProcessCommand::new(env!("CARGO_BIN_EXE_ditto"))
    .args([
      "capture",
      "--profile",
      "ios",
      "--json",
      "--output",
      output_path.to_str().unwrap(),
    ])
    .env("DITTO_CACHE_ROOT", &cache)
    .current_dir(&repository)
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "{}",
    String::from_utf8_lossy(&output.stderr)
  );
  let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(result["command"], "capture");
  assert_eq!(result["lock_sha256"], serde_json::Value::Null);
  assert!(result["baseline_writes"].as_array().unwrap().is_empty());
  assert_eq!(result["scenarios"][0]["status"], "skipped");
  let stored: serde_json::Value = serde_json::from_slice(&fs::read(&output_path).unwrap()).unwrap();
  assert_eq!(stored, result);
  assert_eq!(
    output.stdout.iter().filter(|byte| **byte == b'\n').count(),
    1
  );
  let stderr = String::from_utf8(output.stderr).unwrap();
  assert!(stderr.contains("DITTO_RUN_DIR="));
  assert!(stderr.contains("DITTO_SELECTED=1"));
  assert!(stderr.contains("DITTO_STATUS=passed"));
  assert!(stderr.contains("DITTO_EXIT_CODE=0"));
  assert!(stderr.contains("DITTO_RESULT="));
  assert_eq!(
    fs::read_to_string(baseline.join("sentinel")).unwrap(),
    "unchanged"
  );
  assert_eq!(fs::read_dir(&baseline).unwrap().count(), 1);
}

#[test]
fn runnable_no_build_command_returns_a_durable_machine_failure() {
  let temporary = tempfile::tempdir().unwrap();
  let repository = temporary.path().join("repository");
  let cache = temporary.path().join("cache");
  let tools = temporary.path().join("tools");
  fs::create_dir_all(repository.join("Assets/Scenes")).unwrap();
  fs::create_dir_all(repository.join("ProjectSettings")).unwrap();
  fs::create_dir_all(repository.join("rules/src")).unwrap();
  fs::create_dir_all(&tools).unwrap();
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
  fs::write(repository.join("rules/src/lib.rs"), "pub fn fixture() {}\n").unwrap();
  fs::write(repository.join("ditto.toml"), RUNNABLE_SUITE).unwrap();
  executable(&tools.join("unity"), "#!/bin/sh\necho 6000.0.56f1\n");
  executable(&tools.join("cargo"), "#!/bin/sh\necho cargo 1.94.0\n");
  executable(&tools.join("rustc"), "#!/bin/sh\necho rustc 1.94.0\n");
  executable(&tools.join("xcrun"), "#!/bin/sh\necho xcrun 26.0\n");
  executable(&tools.join("xcodebuild"), "#!/bin/sh\necho 'Xcode 26.0'\n");
  executable(&tools.join("odiff"), "#!/bin/sh\necho odiff 4.5.0\n");
  assert!(
    ProcessCommand::new("git")
      .args(["init", "--quiet"])
      .current_dir(&repository)
      .status()
      .unwrap()
      .success()
  );

  let output = ProcessCommand::new(env!("CARGO_BIN_EXE_ditto"))
    .args(["run", "--no-build", "--json"])
    .env(
      "PATH",
      format!(
        "{}:{}",
        tools.display(),
        std::env::var("PATH").unwrap_or_default()
      ),
    )
    .env("UNITY_EDITOR", tools.join("unity"))
    .env("DITTO_ODIFF_PATH", tools.join("odiff"))
    .env("DITTO_CACHE_ROOT", &cache)
    .env(
      "BATTLEMENT_RESOURCE_SLOTS",
      temporary.path().join("resource-slots"),
    )
    .current_dir(&repository)
    .output()
    .unwrap();

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    output.stdout.iter().filter(|byte| **byte == b'\n').count(),
    1,
    "stdout={} stderr={}",
    String::from_utf8_lossy(&output.stdout),
    String::from_utf8_lossy(&output.stderr)
  );
  let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(result["status"], "infrastructure-error");
  assert_eq!(result["exit_code"], 2);
  assert_eq!(result["build"]["disposition"], "required-by-no-build");
  assert_eq!(result["scenarios"][0]["status"], "not-run");
  assert!(
    result["errors"][0]["message"]
      .as_str()
      .unwrap()
      .contains(result["build"]["fingerprint"].as_str().unwrap())
  );
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("DITTO_PHASE=discovery"));
  assert!(stderr.contains("DITTO_BUILD=required-by-no-build"));
  assert!(stderr.contains("DITTO_STATUS=infrastructure-error"));
  assert!(stderr.contains("DITTO_RESULT="));
  assert!(!stderr.contains("platform execution adapter"));
}

#[test]
fn file_and_standard_input_fragments_produce_complete_handoffs() {
  let temporary = tempfile::tempdir().unwrap();
  let repository = temporary.path().join("repository");
  let baseline = temporary.path().join("baseline-store");
  let cache = temporary.path().join("cache");
  fs::create_dir_all(repository.join("Assets/Scenes")).unwrap();
  fs::create_dir_all(repository.join("ProjectSettings")).unwrap();
  fs::create_dir_all(repository.join("rules/src")).unwrap();
  fs::create_dir_all(&baseline).unwrap();
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
  fs::write(
    repository.join("ditto.toml"),
    SUITE.replace("$BASELINE", &baseline.to_string_lossy()),
  )
  .unwrap();
  assert!(
    ProcessCommand::new("git")
      .args(["init", "--quiet"])
      .current_dir(&repository)
      .status()
      .unwrap()
      .success()
  );
  let fragment_path = temporary.path().join("fragment.toml");
  fs::write(&fragment_path, FRAGMENT).unwrap();

  let file = ProcessCommand::new(env!("CARGO_BIN_EXE_ditto"))
    .args([
      "capture",
      "--fragment",
      fragment_path.to_str().unwrap(),
      "--json",
    ])
    .env("DITTO_CACHE_ROOT", &cache)
    .current_dir(&repository)
    .output()
    .unwrap();
  assert!(
    file.status.success(),
    "{}",
    String::from_utf8_lossy(&file.stderr)
  );
  let file_result: serde_json::Value = serde_json::from_slice(&file.stdout).unwrap();
  assert_eq!(file_result["scenarios"][0]["name"], "fragment hover");
  assert!(String::from_utf8_lossy(&file.stderr).contains("DITTO_RESULT="));

  let mut stdin = ProcessCommand::new(env!("CARGO_BIN_EXE_ditto"))
    .args(["capture", "--fragment=-", "--json"])
    .env("DITTO_CACHE_ROOT", &cache)
    .current_dir(&repository)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
  stdin
    .stdin
    .take()
    .unwrap()
    .write_all(FRAGMENT.as_bytes())
    .unwrap();
  let stdin = stdin.wait_with_output().unwrap();
  assert!(
    stdin.status.success(),
    "{}",
    String::from_utf8_lossy(&stdin.stderr)
  );
  let stdin_result: serde_json::Value = serde_json::from_slice(&stdin.stdout).unwrap();
  assert_eq!(stdin_result["suite"], "standard-input");
  assert_eq!(stdin_result["scenarios"][0]["name"], "fragment hover");
  assert!(String::from_utf8_lossy(&stdin.stderr).contains("DITTO_RESULT="));
}

fn executable(path: &Path, source: &str) {
  fs::write(path, source).unwrap();
  let mut permissions = fs::metadata(path).unwrap().permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(path, permissions).unwrap();
}

const SUITE: &str = r#"name = "fixture"
default_profile = "ios"

[player]
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[baseline]
kind = "filesystem"
namespace = "fixture"
root = "$BASELINE"

[profiles.ios]
target = "ios-simulator"
device = "iPhone 17"
orientation = "portrait"

[[scenarios]]
name = "unsupported hover"

[[scenarios.steps]]
hover = { target = [0.5, 0.5] }
"#;

const RUNNABLE_SUITE: &str = r#"name = "fixture"
default_profile = "macos-local"

[player]
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[profiles.macos-local]
target = "macos"
display = { width = 1280, height = 720, scale = 1.0 }

[[scenarios]]
name = "runnable assertion"

[[scenarios.steps]]
assert = { object = "00000000-0000-0000-0000-000000000001", state = "exists" }
"#;

const FRAGMENT: &str = r#"[[scenarios]]
name = "fragment hover"

[[scenarios.steps]]
hover = { target = [0.5, 0.5] }
"#;
