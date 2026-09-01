#![cfg(target_os = "macos")]

use std::{
  fs,
  os::unix::fs::PermissionsExt,
  path::{Path, PathBuf},
  time::{Duration, Instant},
};

use battlement_ditto::{
  image_comparison::{ImageComparisonRequest, OdiffPool, OdiffServer},
  wire::{job::Comparison, result::ComparisonOutcome},
};

#[test]
fn one_warm_server_handles_exact_tolerated_boundary_and_mismatch_results() {
  let fixture = Fixture::new();
  let mut server = fixture.server("odiff.log");
  let exact = fixture.compare(&mut server, "exact", 100, 100, settings("0.1", "0"));
  assert!(matches!(
    exact.outcome,
    ComparisonOutcome::Passed {
      changed_pixels: 0,
      total_pixels: 10_000,
      ..
    }
  ));
  assert!(exact.diff.is_none());
  assert!(!fixture.root.join("exact-diff.png").exists());

  let tolerated = fixture.compare(&mut server, "tolerated", 100, 100, settings("0.1", "1"));
  assert!(matches!(
    tolerated.outcome,
    ComparisonOutcome::Passed {
      changed_pixels: 1,
      total_pixels: 10_000,
      ..
    }
  ));
  assert!(tolerated.diff.unwrap().path.ends_with("tolerated-diff.png"));

  let boundary = fixture.compare(&mut server, "boundary", 100, 100, settings("0.1", "0.01"));
  assert!(matches!(
    boundary.outcome,
    ComparisonOutcome::Passed {
      changed_pixels: 1,
      ..
    }
  ));

  let mismatch = fixture.compare(&mut server, "mismatch", 100, 100, settings("0.05", "0.01"));
  let ComparisonOutcome::Mismatch {
    changed_pixels,
    total_pixels,
    settings,
    diff,
  } = mismatch.outcome
  else {
    panic!("material change unexpectedly passed")
  };
  assert_eq!((changed_pixels, total_pixels), (2, 10_000));
  assert_eq!(settings.threshold, "0.05");
  assert_eq!(settings.max_changed_percent, "0.01");
  assert_eq!(diff, mismatch.diff.unwrap());
  assert_eq!(
    fs::read_to_string(&fixture.starts).unwrap().lines().count(),
    1
  );
}

#[test]
fn malformed_inputs_and_server_failures_are_infrastructure_errors() {
  let fixture = Fixture::new();
  let mut server = fixture.server("validation.log");
  let baseline = fixture.png("baseline", 10, 10);
  let wrong = fixture.png("wrong", 11, 10);
  let corrupt = fixture.root.join("corrupt.png");
  fs::write(&corrupt, b"not a PNG").unwrap();
  let diff = fixture.root.join("validation-diff.png");
  assert!(
    server
      .compare(request(&baseline, &wrong, &diff))
      .unwrap_err()
      .to_string()
      .contains("dimensions differ")
  );
  assert!(
    server
      .compare(request(&baseline, &corrupt, &diff))
      .unwrap_err()
      .to_string()
      .contains("actual PNG")
  );

  let server_error = fixture.png("error", 10, 10);
  assert!(
    server
      .compare(request(&baseline, &server_error, &diff))
      .unwrap_err()
      .to_string()
      .contains("fixture error")
  );

  let mut exited = fixture.server("exit.log");
  let exit = fixture.png("exit", 10, 10);
  assert!(
    exited
      .compare(request(&baseline, &exit, &diff))
      .unwrap_err()
      .to_string()
      .contains("ODiff comparison failed")
  );

  let mut timed_out = fixture.server("timeout.log");
  let timeout = fixture.png("timeout", 10, 10);
  let started = Instant::now();
  assert!(
    timed_out
      .compare(request(&baseline, &timeout, &diff))
      .unwrap_err()
      .to_string()
      .contains("timed out")
  );
  assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn pool_retries_the_current_comparison_after_a_server_failure() {
  let fixture = Fixture::new();
  let baseline = fixture.png("recover-baseline", 10, 10);
  let actual = fixture.png("recover-actual", 10, 10);
  let diff = fixture.root.join("recover-diff.png");

  let comparison = OdiffPool::default()
    .compare(
      &fixture.binary,
      &fixture.root.join("recover.log"),
      Duration::from_secs(2),
      request(&baseline, &actual, &diff),
    )
    .unwrap();

  assert!(matches!(
    comparison.outcome,
    ComparisonOutcome::Passed {
      changed_pixels: 0,
      ..
    }
  ));
  assert_eq!(
    fs::read_to_string(&fixture.starts).unwrap().lines().count(),
    2
  );
}

#[test]
fn wrong_binary_is_rejected_before_server_start() {
  let fixture = Fixture::new();
  let wrong = fixture.root.join("wrong-odiff");
  fs::write(&wrong, "#!/bin/sh\necho 'ODiff 1.0.0'\n").unwrap();
  fs::set_permissions(&wrong, fs::Permissions::from_mode(0o755)).unwrap();
  let error = OdiffServer::start(
    &wrong,
    &fixture.root.join("wrong.log"),
    Duration::from_secs(2),
  )
  .err()
  .unwrap();
  assert!(error.to_string().contains("not version 4.5.0"));
  assert!(!fixture.starts.exists());

  let hanging = fixture.root.join("hanging-odiff");
  fs::write(&hanging, "#!/bin/sh\nsleep 2\n").unwrap();
  fs::set_permissions(&hanging, fs::Permissions::from_mode(0o755)).unwrap();
  let started = Instant::now();
  let error = OdiffServer::start(
    &hanging,
    &fixture.root.join("hanging.log"),
    Duration::from_millis(100),
  )
  .err()
  .unwrap();
  assert!(error.to_string().contains("version probe timed out"));
  assert!(started.elapsed() < Duration::from_secs(1));
}

struct Fixture {
  _temporary: tempfile::TempDir,
  root: PathBuf,
  binary: PathBuf,
  starts: PathBuf,
}

impl Fixture {
  fn new() -> Self {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().to_owned();
    let binary = root.join("odiff");
    let starts = root.join("starts");
    fs::write(&binary, SERVER).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    Self {
      _temporary: temporary,
      root,
      binary,
      starts,
    }
  }

  fn server(&self, diagnostic: &str) -> OdiffServer {
    OdiffServer::start(
      &self.binary,
      &self.root.join(diagnostic),
      Duration::from_secs(2),
    )
    .unwrap()
  }

  fn png(&self, name: &str, width: u32, height: u32) -> PathBuf {
    let path = self.root.join(format!("{name}.png"));
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bytes.extend(width.to_be_bytes());
    bytes.extend(height.to_be_bytes());
    fs::write(&path, bytes).unwrap();
    path
  }

  fn compare(
    &self,
    server: &mut OdiffServer,
    name: &str,
    width: u32,
    height: u32,
    settings: Comparison,
  ) -> battlement_ditto::image_comparison::ImageComparison {
    let baseline = self.png(&format!("{name}-baseline"), width, height);
    let actual = self.png(&format!("{name}-actual"), width, height);
    let diff = self.root.join(format!("{name}-diff.png"));
    if name == "exact" {
      self.png("exact-diff", width, height);
    }
    server
      .compare(ImageComparisonRequest {
        baseline: &baseline,
        actual: &actual,
        diff: &diff,
        settings,
        timeout: Duration::from_millis(250),
      })
      .unwrap()
  }
}

fn settings(threshold: &str, percent: &str) -> Comparison {
  Comparison {
    threshold: threshold.to_owned(),
    anti_alias: true,
    max_changed_percent: percent.to_owned(),
  }
}

fn request<'a>(baseline: &'a Path, actual: &'a Path, diff: &'a Path) -> ImageComparisonRequest<'a> {
  ImageComparisonRequest {
    baseline,
    actual,
    diff,
    settings: settings("0.1", "0"),
    timeout: Duration::from_millis(250),
  }
}

const SERVER: &str = r#"#!/usr/bin/env python3
import json
import os
import shutil
import sys
import time

if '--version' in sys.argv:
    print('ODiff 4.5.0')
    sys.exit(0)

with open(os.path.join(os.path.dirname(__file__), 'starts'), 'a') as count:
    count.write('start\n')
print('{"ready":true}', flush=True)
for line in sys.stdin:
    request = json.loads(line)
    name = os.path.basename(request['compare'])
    response = {'requestId': request['requestId']}
    if 'exit' in name:
        sys.exit(7)
    if 'recover' in name and sum(1 for _ in open(os.path.join(os.path.dirname(__file__), 'starts'))) == 1:
        sys.exit(8)
    if 'timeout' in name:
        time.sleep(1)
        continue
    if 'error' in name:
        response['error'] = 'fixture error'
    elif 'exact' in name or 'recover' in name:
        response['match'] = True
    else:
        shutil.copyfile(request['compare'], request['output'])
        response.update({
            'match': False,
            'reason': 'pixel-diff',
            'diffCount': 2 if 'mismatch' in name else 1,
            'diffPercentage': 0.01,
        })
    print(json.dumps(response, separators=(',', ':')), flush=True)
"#;
