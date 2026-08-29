//! Native raw-video preflight and host-side MP4 processing.

use std::{
  fs,
  path::{Path, PathBuf},
  process::{Command, Stdio},
};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};

use crate::wire::{
  common::{ErrorCode, ErrorSource},
  lifecycle::NativeVideoInput,
  result::VideoResult,
};

const FRAME_RATE: u64 = 30;
const MEDIA_RESERVE_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) struct NativeVideoProcessor {
  ffmpeg: PathBuf,
  run_directory: PathBuf,
}

#[derive(Debug)]
pub(crate) struct NativeVideoFailure {
  pub code: ErrorCode,
  pub source: ErrorSource,
  pub message: String,
  pub diagnostic_paths: Vec<String>,
}

impl NativeVideoProcessor {
  pub(crate) fn new(ffmpeg: PathBuf, run_directory: PathBuf) -> Self {
    Self {
      ffmpeg,
      run_directory,
    }
  }

  pub(crate) fn process(
    &self,
    input: &NativeVideoInput,
    max_duration_ms: u64,
  ) -> std::result::Result<VideoResult, NativeVideoFailure> {
    self
      .process_inner(input, max_duration_ms)
      .map_err(|failure| self.retain_failure(input, failure))
  }

  fn process_inner(
    &self,
    input: &NativeVideoInput,
    max_duration_ms: u64,
  ) -> std::result::Result<VideoResult, NativeVideoFailure> {
    let raw = Path::new(&input.path);
    let frame_bytes = frame_bytes(input.width, input.height).map_err(recording_failure)?;
    let maximum_frames = maximum_frames(max_duration_ms).map_err(recording_failure)?;
    if input.frame_count > maximum_frames {
      return Err(recording_failure(anyhow::anyhow!(
        "native video contains {} frames but its declared maximum is {maximum_frames}",
        input.frame_count
      )));
    }
    let expected_bytes = frame_bytes
      .checked_mul(input.frame_count)
      .context("native video byte size overflow")
      .map_err(recording_failure)?;
    let actual_bytes = fs::metadata(raw)
      .with_context(|| format!("inspect native video input {}", raw.display()))
      .map_err(recording_failure)?
      .len();
    if actual_bytes % frame_bytes != 0 {
      return Err(recording_failure(anyhow::anyhow!(
        "native video ends with a partial frame: {actual_bytes} bytes for {frame_bytes}-byte frames"
      )));
    }
    if actual_bytes != expected_bytes {
      return Err(recording_failure(anyhow::anyhow!(
        "native video byte size is {actual_bytes}; expected {expected_bytes}"
      )));
    }
    let actual_hash = sha256(raw).map_err(recording_failure)?;
    if actual_hash != input.sha256 {
      return Err(recording_failure(anyhow::anyhow!(
        "native video SHA-256 does not match its metadata"
      )));
    }

    let relative = format!("videos/{}.mp4", input.input_id);
    let output = self.run_directory.join(&relative);
    let temporary = output.with_extension("new.mp4");
    if let Some(parent) = output.parent() {
      fs::create_dir_all(parent).map_err(|error| recording_failure(error.into()))?;
    }
    let size = format!("{}x{}", input.width, input.height);
    let encoded = Command::new(&self.ffmpeg)
      .args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "rawvideo",
        "-pixel_format",
        "rgba",
        "-video_size",
        &size,
        "-framerate",
        "30",
        "-i",
      ])
      .arg(raw)
      .args([
        "-an",
        "-c:v",
        "libx264",
        "-crf",
        "18",
        "-pix_fmt",
        "yuv420p",
        "-movflags",
        "+faststart",
      ])
      .arg(&temporary)
      .stdout(Stdio::null())
      .output()
      .map_err(|error| ffmpeg_failure(error.into()))?;
    if !encoded.status.success() {
      return Err(ffmpeg_failure(anyhow::anyhow!(
        "FFmpeg exited with {}: {}",
        encoded.status,
        String::from_utf8_lossy(&encoded.stderr).trim()
      )));
    }
    validate_mp4(&self.ffmpeg, &temporary).map_err(ffmpeg_failure)?;
    fs::rename(&temporary, &output).map_err(|error| recording_failure(error.into()))?;
    let hash = sha256(&output).map_err(recording_failure)?;
    fs::remove_file(raw).map_err(|error| recording_failure(error.into()))?;
    Ok(VideoResult::Encoded {
      path: relative,
      sha256: hash,
      width: input.width,
      height: input.height,
      frame_rate: FRAME_RATE as u32,
      duration_ms: input.frame_count.saturating_mul(1_000) / FRAME_RATE,
      truncated: input.truncated,
    })
  }

  fn retain_failure(
    &self,
    input: &NativeVideoInput,
    mut failure: NativeVideoFailure,
  ) -> NativeVideoFailure {
    let source = Path::new(&input.path);
    let temporary = self
      .run_directory
      .join(format!("videos/{}.new.mp4", input.input_id));
    let _ = fs::remove_file(temporary);
    if source.is_file() {
      let relative = format!("diagnostics/video/{}.raw", input.input_id);
      let destination = self.run_directory.join(&relative);
      let retained = destination
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| {
          if source == destination {
            Ok(())
          } else {
            fs::copy(source, &destination).map(|_| ())
          }
        });
      if retained.is_ok() {
        failure.diagnostic_paths.push(relative);
      }
    }
    failure
  }
}

pub(crate) fn required_bytes(width: u32, height: u32, max_duration_ms: u64) -> Result<u64> {
  frame_bytes(width, height)?
    .checked_mul(maximum_frames(max_duration_ms)?)
    .and_then(|bytes| bytes.checked_add(MEDIA_RESERVE_BYTES))
    .context("native video disk preflight overflow")
}

pub(crate) fn ensure_available(required: u64, available: u64) -> Result<()> {
  ensure!(
    available >= required,
    "native video requires {required} bytes but only {available} bytes are available"
  );
  Ok(())
}

fn maximum_frames(max_duration_ms: u64) -> Result<u64> {
  max_duration_ms
    .checked_mul(FRAME_RATE)
    .and_then(|value| value.checked_add(999))
    .map(|value| value / 1_000)
    .context("native video frame-count overflow")
}

fn frame_bytes(width: u32, height: u32) -> Result<u64> {
  u64::from(width)
    .checked_mul(u64::from(height))
    .and_then(|pixels| pixels.checked_mul(4))
    .context("native video frame-size overflow")
}

fn validate_mp4(ffmpeg: &Path, path: &Path) -> Result<()> {
  let bytes = fs::read(path).with_context(|| format!("read encoded MP4 {}", path.display()))?;
  ensure!(
    bytes.len() >= 12 && &bytes[4..8] == b"ftyp",
    "FFmpeg output is not an MP4 container"
  );
  let decoded = Command::new(ffmpeg)
    .args(["-hide_banner", "-loglevel", "error", "-i"])
    .arg(path)
    .args(["-map", "0:v:0", "-f", "null", "-"])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .output()
    .context("validate encoded MP4 through FFmpeg")?;
  ensure!(
    decoded.status.success(),
    "FFmpeg rejected its MP4 output: {}",
    String::from_utf8_lossy(&decoded.stderr).trim()
  );
  Ok(())
}

fn sha256(path: &Path) -> Result<String> {
  Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn recording_failure(error: anyhow::Error) -> NativeVideoFailure {
  NativeVideoFailure {
    code: ErrorCode::MediaRecordingFailed,
    source: ErrorSource::Filesystem,
    message: format!("{error:#}"),
    diagnostic_paths: Vec::new(),
  }
}

fn ffmpeg_failure(error: anyhow::Error) -> NativeVideoFailure {
  NativeVideoFailure {
    code: ErrorCode::MediaFfmpegFailed,
    source: ErrorSource::FFmpeg,
    message: format!("{error:#}"),
    diagnostic_paths: Vec::new(),
  }
}
