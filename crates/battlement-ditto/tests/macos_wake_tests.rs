#![cfg(target_os = "macos")]

use std::{
  fs,
  os::unix::fs::PermissionsExt,
  path::Path,
  process::Command,
  thread,
  time::{Duration, Instant},
};

use battlement_ditto::{
  macos_capture::{ImmutableMacosLauncher, MacosPlayerLauncher},
  player_supervision::PlayerSupervisor,
};

#[test]
fn wake_assertions_follow_the_supervised_player_on_success_failure_and_cancel() {
  for exit in [Some(0), Some(7), None] {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("player");
    let started = directory.path().join("started");
    let release = directory.path().join("release");
    fs::write(
      &executable,
      format!(
        "#!/usr/bin/python3\nimport os, pathlib, time\npathlib.Path({started:?}).write_text(str(os.getpid()))\nwhile not pathlib.Path({release:?}).exists(): time.sleep(0.01)\nraise SystemExit({})\n",
        exit.unwrap_or(0)
      ),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let child = ImmutableMacosLauncher
      .launch(&executable, "http://fixture", &started, 1280, 720, None)
      .unwrap();
    let player_pid = child.id();
    let mut supervisor = PlayerSupervisor::macos(child);
    wait_until(|| fs::read_to_string(&started).ok().as_deref() == Some(&player_pid.to_string()));
    assert_eq!(
      fs::read_to_string(&started).unwrap(),
      player_pid.to_string()
    );
    let mut wake_pid = None;
    wait_until(|| {
      wake_pid = wake_child(player_pid);
      wake_pid.is_some()
    });
    let wake_pid = wake_pid.unwrap();
    wait_until(|| {
      let assertions = output("/usr/bin/pmset", &["-g", "assertions"]);
      let owned = assertions
        .lines()
        .filter(|line| line.contains(&format!("pid {wake_pid}(")))
        .collect::<Vec<_>>();
      ["PreventUserIdleDisplaySleep", "PreventUserIdleSystemSleep"]
        .iter()
        .all(|kind| owned.iter().any(|line| line.contains(kind)))
    });
    if let Some(code) = exit {
      fs::write(&release, "exit").unwrap();
      let mut observed = None;
      wait_until(|| {
        observed = supervisor.poll().unwrap();
        observed.is_some()
      });
      assert_eq!(observed.unwrap().code, Some(code));
    }
    drop(supervisor);
    wait_until(|| !is_live(player_pid) && !is_live(wake_pid));
    wait_until(|| {
      !output("/usr/bin/pmset", &["-g", "assertions"]).contains(&format!("pid {wake_pid}("))
    });
  }
}

#[test]
fn missing_player_preserves_spawn_failure_exit_code() {
  let directory = tempfile::tempdir().unwrap();
  let child = ImmutableMacosLauncher
    .launch(
      &directory.path().join("missing"),
      "http://fixture",
      Path::new("/dev/null"),
      1280,
      720,
      None,
    )
    .unwrap();
  let mut supervisor = PlayerSupervisor::macos(child);
  let mut observed = None;
  wait_until(|| {
    observed = supervisor.poll().unwrap();
    observed.is_some()
  });
  assert_eq!(observed.unwrap().code, Some(127));
}

fn wait_until(mut ready: impl FnMut() -> bool) {
  let started = Instant::now();
  while !ready() {
    assert!(
      started.elapsed() < Duration::from_secs(5),
      "player lifecycle deadline expired"
    );
    thread::sleep(Duration::from_millis(10));
  }
}

fn wake_child(parent: u32) -> Option<u32> {
  output("/bin/ps", &["-axo", "pid=,ppid=,comm="])
    .lines()
    .find_map(|line| {
      let mut columns = line.split_whitespace();
      let pid = columns.next()?.parse().ok()?;
      let ppid = columns.next()?.parse::<u32>().ok()?;
      (ppid == parent && columns.next()? == "/usr/bin/caffeinate").then_some(pid)
    })
}

fn is_live(pid: u32) -> bool {
  let status = output("/bin/ps", &["-p", &pid.to_string(), "-o", "stat="]);
  !status.trim().is_empty() && !status.trim().starts_with('Z')
}

fn output(program: &str, args: &[&str]) -> String {
  String::from_utf8(Command::new(program).args(args).output().unwrap().stdout).unwrap()
}
