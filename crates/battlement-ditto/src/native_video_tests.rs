use std::{fs, path::Path};

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use uuid::Uuid;

use crate::{
  native_video::{NativeVideoProcessor, ensure_available, required_bytes},
  wire::{lifecycle::NativeVideoInput, result::VideoResult},
};

#[test]
fn encodes_rgba_at_fixed_rate_and_removes_the_raw_input() {
  let fixture = Fixture::new(true);
  let input = fixture.input(vec![255; 2 * 2 * 4 * 3], 3);

  let result = fixture.processor().process(&input, 100).unwrap();

  let VideoResult::Encoded {
    path,
    width,
    height,
    frame_rate,
    duration_ms,
    truncated,
    ..
  } = result
  else {
    panic!("video was not encoded");
  };
  assert_eq!((width, height, frame_rate, duration_ms), (2, 2, 30, 100));
  assert!(truncated);
  assert!(!Path::new(&input.path).exists());
  assert!(fixture.root.path().join(path).is_file());
}

#[test]
fn rejects_partial_and_wrong_sized_frames_while_retaining_bounded_raw_diagnostics() {
  let partial = Fixture::new(true);
  let partial_input = partial.input(vec![1; 17], 1);
  let failure = partial
    .processor()
    .process(&partial_input, 100)
    .unwrap_err();
  assert!(
    failure.message.contains("partial frame"),
    "{}",
    failure.message
  );
  assert_eq!(failure.diagnostic_paths.len(), 1);
  assert_eq!(
    fs::metadata(partial.root.path().join(&failure.diagnostic_paths[0]))
      .unwrap()
      .len(),
    17
  );

  let wrong = Fixture::new(true);
  let wrong_input = wrong.input(vec![1; 32], 1);
  let failure = wrong.processor().process(&wrong_input, 100).unwrap_err();
  assert!(
    failure.message.contains("expected 16"),
    "{}",
    failure.message
  );
  assert_eq!(failure.diagnostic_paths.len(), 1);
}

#[test]
fn ffmpeg_failure_never_publishes_an_mp4_and_keeps_the_raw_input() {
  let fixture = Fixture::new(false);
  let input = fixture.input(vec![7; 16], 1);

  let failure = fixture.processor().process(&input, 100).unwrap_err();

  assert!(
    failure.message.contains("FFmpeg exited"),
    "{}",
    failure.message
  );
  assert_eq!(failure.diagnostic_paths.len(), 1);
  assert!(
    !fixture
      .root
      .path()
      .join(format!("videos/{}.mp4", input.input_id))
      .exists()
  );
  assert!(Path::new(&input.path).is_file());
}

#[test]
fn disk_preflight_uses_checked_ceiling_frames_and_sixty_four_megabytes() {
  let expected = 2_u64 * 3 * 4 * 31 + 64 * 1024 * 1024;
  assert_eq!(required_bytes(2, 3, 1_001).unwrap(), expected);
  ensure_available(expected, expected).unwrap();
  let error = ensure_available(expected, expected - 1)
    .unwrap_err()
    .to_string();
  assert!(error.contains(&expected.to_string()), "{error}");
  assert!(error.contains(&(expected - 1).to_string()), "{error}");
  assert!(required_bytes(u32::MAX, u32::MAX, u64::MAX).is_err());
}

struct Fixture {
  root: TempDir,
  ffmpeg: std::path::PathBuf,
}

impl Fixture {
  fn new(succeeds: bool) -> Self {
    let root = tempfile::tempdir().unwrap();
    let ffmpeg = root.path().join("ffmpeg");
    let source = if succeeds {
      r#"#!/bin/sh
case " $* " in
  *" -f rawvideo "*)
    for output do :; done
    printf '\000\000\000\030ftypisom00000000' > "$output"
    ;;
esac
exit 0
"#
    } else {
      "#!/bin/sh\necho injected encoder failure >&2\nexit 9\n"
    };
    fs::write(&ffmpeg, source).unwrap();
    executable(&ffmpeg);
    Self { root, ffmpeg }
  }

  fn input(&self, bytes: Vec<u8>, frame_count: u64) -> NativeVideoInput {
    let input_id = Uuid::new_v4().to_string();
    let path = self.root.path().join(format!("{input_id}.raw"));
    fs::write(&path, &bytes).unwrap();
    NativeVideoInput {
      input_id,
      start_step_index: 0,
      path: path.to_string_lossy().into_owned(),
      sha256: format!("{:x}", Sha256::digest(&bytes)),
      width: 2,
      height: 2,
      frame_count,
      truncated: true,
    }
  }

  fn processor(&self) -> NativeVideoProcessor {
    NativeVideoProcessor::new(self.ffmpeg.clone(), self.root.path().to_path_buf())
  }
}

#[cfg(unix)]
fn executable(path: &Path) {
  use std::os::unix::fs::PermissionsExt;

  let mut permissions = fs::metadata(path).unwrap().permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(path, permissions).unwrap();
}

#[cfg(not(unix))]
fn executable(_: &Path) {}
