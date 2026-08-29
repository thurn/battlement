use std::{
  fs,
  io::{Read, Write},
  net::{Shutdown, TcpStream},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  thread,
  time::Duration,
};

use battlement_ditto::{
  review_server::ReviewServer,
  wire::{
    common::{StepName, StepStatus},
    job::Motion,
    result::{
      BaselineOutcome, ImageFile, Recovery, ResultCommand, RunResult, RunStatus, ScenarioResult,
      ScenarioStatus, ScenarioTimings, ScreenshotResult, StepResult,
    },
  },
};

#[test]
fn review_server_is_offline_read_only_and_artifact_scoped() {
  let temporary = tempfile::tempdir().unwrap();
  fs::create_dir_all(temporary.path().join("images")).unwrap();
  fs::write(temporary.path().join("images/actual.png"), b"retained-png").unwrap();
  let server = ReviewServer::bind(temporary.path(), capture_result()).unwrap();
  assert!(server.url().starts_with("http://127.0.0.1:"));
  let base = server.url();
  let interrupted = Arc::new(AtomicBool::new(false));
  let worker_interrupt = Arc::clone(&interrupted);
  let worker = thread::spawn(move || server.serve(&worker_interrupt).unwrap());

  let page = exchange("GET", &base, "/");
  assert_eq!(page.status, 200);
  assert!(page.headers.contains("Content-Security-Policy:"));
  assert!(page.body.contains("Ditto Review"));
  assert!(!page.body.contains("https://"));

  let script = exchange("GET", &base, "/app.js");
  assert_eq!(script.status, 200);
  for capability in ["split", "swipe", "overlay", "mask", "Panzoom"] {
    assert!(script.body.contains(capability), "missing {capability}");
  }

  let result = exchange("GET", &base, "/api/result");
  assert_eq!(result.status, 200);
  let result: RunResult = serde_json::from_str(&result.body).unwrap();
  assert_eq!(result.run_id, "39e15c94-f631-454e-86a0-2659299d1637");

  let artifact = exchange("GET", &base, "/artifact/images%2Factual.png");
  assert_eq!(artifact.status, 200);
  assert_eq!(artifact.body.as_bytes(), b"retained-png");
  assert_eq!(
    exchange("GET", &base, "/artifact/not-retained.png").status,
    404
  );
  assert_eq!(exchange("POST", &base, "/api/result").status, 405);

  interrupted.store(true, Ordering::Release);
  worker.join().unwrap();
}

struct HttpResponse {
  status: u16,
  headers: String,
  body: String,
}

fn exchange(method: &str, base: &str, path: &str) -> HttpResponse {
  let authority = base.strip_prefix("http://").unwrap().trim_end_matches('/');
  let mut stream = TcpStream::connect(authority).unwrap();
  stream
    .set_read_timeout(Some(Duration::from_secs(2)))
    .unwrap();
  write!(
    stream,
    "{method} {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
  )
  .unwrap();
  stream.shutdown(Shutdown::Write).unwrap();
  let mut bytes = Vec::new();
  stream.read_to_end(&mut bytes).unwrap();
  let response = String::from_utf8(bytes).unwrap();
  let (headers, body) = response.split_once("\r\n\r\n").unwrap();
  let status = headers
    .lines()
    .next()
    .unwrap()
    .split_whitespace()
    .nth(1)
    .unwrap()
    .parse()
    .unwrap();
  HttpResponse {
    status,
    headers: headers.to_owned(),
    body: body.to_owned(),
  }
}

fn capture_result() -> RunResult {
  RunResult {
    run_id: "39e15c94-f631-454e-86a0-2659299d1637".to_owned(),
    source_run_id: None,
    lock_sha256: None,
    command: ResultCommand::Capture,
    source_command: None,
    cycle: 1,
    suite: Some("review suite".to_owned()),
    profile: Some("macos-local".to_owned()),
    started_at: "2026-08-29T10:00:00Z".to_owned(),
    duration_ms: 14,
    status: RunStatus::Passed,
    exit_code: 0,
    build: None,
    phases: vec![],
    player_sessions: vec![],
    jobs: vec![],
    scenarios: vec![ScenarioResult {
      id: "0b277b84-e0e4-47ba-8804-90eb640f7519".to_owned(),
      name: "menu opens".to_owned(),
      status: ScenarioStatus::Passed,
      status_reason: None,
      motion: Motion::Instant,
      duration_ms: 14,
      expired_deadline: None,
      timings: ScenarioTimings::default(),
      steps: vec![StepResult {
        index: 0,
        name: None,
        kind: StepName::Screenshot,
        status: StepStatus::Passed,
        status_reason: None,
        duration_ms: 6,
        expired_deadline: None,
        error_ids: vec![],
        assertion: None,
        screenshot: Some(ScreenshotResult::Captured {
          checkpoint: "menu".to_owned(),
          actual: ImageFile {
            path: "images/actual.png".to_owned(),
            sha256: "a".repeat(64),
            width: 1,
            height: 1,
          },
          baseline: BaselineOutcome::NotLoaded,
          comparison: None,
          matched_before_update: None,
          updated: None,
        }),
        video: None,
      }],
      logs: None,
      failure_frame: None,
      recovery: Recovery::None,
    }],
    warnings: vec![],
    errors: vec![],
    baseline_writes: vec![],
    artifacts: vec!["images/actual.png".to_owned()],
  }
}
