use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::Path,
};

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::wire::{
  job::PerformancePass,
  lifecycle::{ObserverFrameTiming, StepPerformance},
  result::{
    Distribution, MeasuredStepAttempt, MeasuredStepPerformance, ObserverPassSummary,
    ObserverStageSummary, PerformanceAttemptSummary, PerformanceHotspot, PerformanceResult,
    RunResult,
  },
};

const TIMING_PROXY: &str = "unity-wait-for-end-of-frame";
const TIMING_LIMITATION: &str = "Unity end-of-frame callback timestamps are a deterministic CPU-side proxy. They are not OS compositor, display-scanout, or input-to-photon timestamps.";
type ObserverStageSelector = (&'static str, fn(&ObserverFrameTiming) -> u64);

pub(crate) fn materialize(result: &mut RunResult, directory: &Path) -> Result<()> {
  if result.command != crate::wire::result::ResultCommand::Profile {
    return Ok(());
  }
  if result.status != crate::wire::result::RunStatus::Passed {
    return Ok(());
  }
  let attempts = result
    .scenarios
    .iter()
    .filter_map(|scenario| {
      scenario
        .performance_attempt
        .as_ref()
        .map(|attempt| (scenario, attempt))
    })
    .collect::<Vec<_>>();
  ensure!(
    !attempts.is_empty(),
    "profile produced no performance attempts"
  );
  let target_fps = attempts[0].1.target_fps;
  ensure!(
    attempts
      .iter()
      .all(|(_, attempt)| attempt.target_fps == target_fps),
    "profile attempts disagree about the performance target"
  );
  let mut score_values = Vec::new();
  let mut score_attempts = Vec::new();
  let mut warmup_attempts = Vec::new();
  let mut pacing_configurations = Vec::new();
  let mut by_step: BTreeMap<String, Vec<(u32, &StepPerformance)>> = BTreeMap::new();
  for (scenario, attempt) in &attempts {
    let mut attempt_total = 0;
    let mut response_total = 0;
    let mut pacing_total = 0;
    let mut no_visual_response_attempts = 0;
    for step in &scenario.steps {
      if let Some(performance) = &step.performance {
        ensure!(
          performance.target_fps == target_fps,
          "measured step disagrees about the performance target"
        );
        ensure!(
          performance.timing_proxy == TIMING_PROXY,
          "measured step uses an unsupported timing proxy"
        );
        ensure!(
          deadline_misses(
            performance.target_fps,
            performance.response_latency_ns,
            &performance.presentation_timestamps_ns,
            performance.no_visual_response,
          )? == performance.missed_interaction_deadlines,
          "player performance deadline count is inconsistent"
        );
        attempt_total += performance.missed_interaction_deadlines;
        response_total += performance.response_missed_deadlines;
        pacing_total += performance.pacing_missed_deadlines;
        no_visual_response_attempts += u32::from(performance.no_visual_response);
        if !pacing_configurations.contains(&performance.pacing_configuration) {
          pacing_configurations.push(performance.pacing_configuration.clone());
        }
        if !attempt.warmup && attempt.pass == PerformancePass::Score {
          by_step
            .entry(
              step
                .name
                .clone()
                .unwrap_or_else(|| format!("step-{}", step.index)),
            )
            .or_default()
            .push((attempt.iteration, performance));
        }
      }
    }
    let summary = PerformanceAttemptSummary {
      pass: attempt.pass,
      iteration: attempt.iteration,
      warmup: attempt.warmup,
      missed_interaction_deadlines: attempt_total,
      response_missed_deadlines: response_total,
      pacing_missed_deadlines: pacing_total,
      no_visual_response_attempts,
    };
    if attempt.warmup {
      warmup_attempts.push(summary);
    } else if attempt.pass == PerformancePass::Score {
      score_values.push(attempt_total);
      score_attempts.push(summary);
    }
  }
  ensure!(
    score_values.len()
      == usize::try_from(crate::profile_commands::MEASURED_ITERATIONS)
        .expect("measurement iteration count fits usize"),
    "profile did not produce exactly five measured score attempts"
  );
  ensure!(
    attempts
      .iter()
      .filter(|(_, attempt)| attempt.pass == PerformancePass::Detail && !attempt.warmup)
      .count()
      == usize::try_from(crate::profile_commands::MEASURED_ITERATIONS)
        .expect("measurement iteration count fits usize"),
    "profile did not produce exactly five measured detail attempts"
  );
  warmup_attempts.sort_by_key(|attempt| (performance_pass_key(attempt.pass), attempt.iteration));
  score_attempts.sort_by_key(|attempt| attempt.iteration);
  score_values.sort_unstable();
  let headline = percentile(&score_values, 50);
  let measured_steps = by_step
    .into_iter()
    .map(|(name, values)| measured_step(name, &values, target_fps))
    .collect::<Result<Vec<_>>>()?;
  let observer_passes = [PerformancePass::Score, PerformancePass::Detail]
    .into_iter()
    .map(|pass| observer_pass(&attempts, pass))
    .collect::<Result<Vec<_>>>()?;
  let detail_hotspots = detail_hotspots(result, directory)?;
  ensure!(
    measured_steps
      .iter()
      .all(|step| { step.attempts == crate::profile_commands::MEASURED_ITERATIONS }),
    "profile measured steps do not all have five attempts"
  );
  result.performance = Some(PerformanceResult {
    headline: format!("Missed {target_fps} Hz interaction deadlines"),
    timing_proxy: TIMING_PROXY.to_owned(),
    timing_limitation: TIMING_LIMITATION.to_owned(),
    target_fps,
    missed_interaction_deadlines: headline,
    goal: 0,
    measured_score_attempts: u32::try_from(score_values.len())
      .expect("score attempt count is bounded"),
    score_attempt_values: score_values,
    pacing_configurations,
    warmup_attempts,
    score_attempts,
    measured_steps,
    observer_passes,
    detail_hotspots,
  });
  write_artifacts(result, directory)
}

pub(crate) fn measured_step(
  name: String,
  values: &[(u32, &StepPerformance)],
  target_fps: u32,
) -> Result<MeasuredStepPerformance> {
  let response = values
    .iter()
    .map(|(_, value)| value.response_latency_ns)
    .collect::<Vec<_>>();
  let semantic = values
    .iter()
    .filter_map(|(_, value)| value.semantic_completion_latency_ns)
    .collect::<Vec<_>>();
  let settled = values
    .iter()
    .filter_map(|(_, value)| value.settled_completion_latency_ns)
    .collect::<Vec<_>>();
  let intervals = values
    .iter()
    .flat_map(|(_, value)| value.presentation_intervals_ns.iter().copied())
    .collect::<Vec<_>>();
  let period_ns = 1_000_000_000 / u64::from(target_fps);
  let attempt_values = values
    .iter()
    .map(|(iteration, value)| MeasuredStepAttempt {
      iteration: *iteration,
      response_latency_ns: value.response_latency_ns,
      semantic_completion_latency_ns: value.semantic_completion_latency_ns,
      settled_completion_latency_ns: value.settled_completion_latency_ns,
      response_missed_deadlines: value.response_missed_deadlines,
      pacing_missed_deadlines: value.pacing_missed_deadlines,
      missed_interaction_deadlines: value.missed_interaction_deadlines,
      no_visual_response: value.no_visual_response,
      presented_frames: u32::try_from(value.presentation_timestamps_ns.len())
        .expect("presented frame count is bounded"),
      presentation_interval_distribution: distribution(&value.presentation_intervals_ns),
    })
    .collect();
  let managed_allocated_bytes = if values
    .iter()
    .all(|(_, value)| value.managed_allocation_deltas.is_some())
  {
    Some(
      values
        .iter()
        .flat_map(|(_, value)| {
          value
            .managed_allocation_deltas
            .as_ref()
            .expect("all allocation samples were present")
        })
        .try_fold(0i64, |total, value| total.checked_add(*value))
        .ok_or_else(|| anyhow::anyhow!("performance allocation total overflow"))?,
    )
  } else {
    None
  };
  Ok(MeasuredStepPerformance {
    name,
    attempts: u32::try_from(values.len()).expect("score attempt count is bounded"),
    no_visual_response_attempts: u32::try_from(
      values
        .iter()
        .filter(|(_, value)| value.no_visual_response)
        .count(),
    )?,
    missed_interaction_deadlines: checked_sum(values, |value| value.missed_interaction_deadlines)?,
    response_missed_deadlines: checked_sum(values, |value| value.response_missed_deadlines)?,
    pacing_missed_deadlines: checked_sum(values, |value| value.pacing_missed_deadlines)?,
    median_response_latency_ns: percentile(&response, 50),
    p95_response_latency_ns: percentile(&response, 95),
    maximum_response_latency_ns: percentile(&response, 100),
    median_semantic_completion_latency_ns: (!semantic.is_empty())
      .then(|| percentile(&semantic, 50)),
    median_settled_completion_latency_ns: (!settled.is_empty()).then(|| percentile(&settled, 50)),
    worst_presentation_interval_ns: percentile(&intervals, 100),
    presentation_interval_distribution: distribution(&intervals),
    over_budget_presentation_intervals: u64::try_from(
      intervals.iter().filter(|value| **value > period_ns).count(),
    )?,
    longest_over_budget_sequence: longest_over_budget_attempt_sequence(
      values
        .iter()
        .map(|(_, value)| value.presentation_intervals_ns.as_slice()),
      period_ns,
    ),
    managed_allocated_bytes,
    attempt_values,
  })
}

fn checked_sum(
  values: &[(u32, &StepPerformance)],
  select: impl Fn(&StepPerformance) -> u64,
) -> Result<u64> {
  values.iter().try_fold(0u64, |total, (_, value)| {
    total
      .checked_add(select(value))
      .ok_or_else(|| anyhow::anyhow!("performance total overflow"))
  })
}

pub(crate) fn observer_pass(
  attempts: &[(
    &crate::wire::result::ScenarioResult,
    &crate::wire::job::PerformanceAttempt,
  )],
  pass: PerformancePass,
) -> Result<ObserverPassSummary> {
  let selected = attempts
    .iter()
    .filter(|(_, attempt)| attempt.pass == pass && !attempt.warmup)
    .flat_map(|(scenario, _)| scenario.steps.iter())
    .filter_map(|step| step.performance.as_ref())
    .flat_map(|performance| performance.observer_timings.iter())
    .collect::<Vec<_>>();
  let stages: [ObserverStageSelector; 13] = [
    ("motion-prepare", |value| value.motion_prepare_ns),
    ("runner-frame", |value| value.runner_frame_ns),
    ("native-frame-complete", |value| {
      value.native_frame_complete_ns
    }),
    ("input-release-and-sync-transport", |value| {
      value.input_release_and_sync_transport_ns
    }),
    ("response-decode", |value| value.response_decode_ns),
    ("response-apply", |value| value.response_apply_ns),
    ("end-of-frame-wait", |value| value.end_of_frame_wait_ns),
    ("texture-setup", |value| value.texture_setup_ns),
    ("capture-request-cpu", |value| value.capture_request_cpu_ns),
    ("synchronous-readback", |value| {
      value.synchronous_readback_ns
    }),
    ("cpu-hash", |value| value.cpu_hash_ns),
    ("layout-observation", |value| value.layout_observation_ns),
    ("recorder-bookkeeping", |value| {
      value.recorder_bookkeeping_ns
    }),
  ];
  let stages = stages
    .into_iter()
    .map(|(stage, select)| {
      let values = selected
        .iter()
        .map(|value| select(value))
        .collect::<Vec<_>>();
      let total_ns = values.iter().try_fold(0u64, |total, value| {
        total
          .checked_add(*value)
          .ok_or_else(|| anyhow::anyhow!("observer timing total overflow"))
      })?;
      Ok(ObserverStageSummary {
        stage: stage.to_owned(),
        total_ns,
        p95_ns: percentile(&values, 95),
        maximum_ns: percentile(&values, 100),
      })
    })
    .collect::<Result<Vec<_>>>()?;
  Ok(ObserverPassSummary {
    pass,
    attempts: crate::profile_commands::MEASURED_ITERATIONS,
    observed_frames: u64::try_from(selected.len())?,
    observed_pixels: selected.iter().try_fold(0u64, |total, value| {
      total
        .checked_add(u64::from(value.observed_pixels))
        .ok_or_else(|| anyhow::anyhow!("observer pixel total overflow"))
    })?,
    stages,
  })
}

fn distribution(values: &[u64]) -> Distribution {
  Distribution {
    count: u64::try_from(values.len()).expect("performance sample count is bounded"),
    minimum_ns: values.iter().copied().min().unwrap_or(0),
    p50_ns: percentile(values, 50),
    p95_ns: percentile(values, 95),
    p99_ns: percentile(values, 99),
    maximum_ns: percentile(values, 100),
  }
}

fn percentile(values: &[u64], percentile: usize) -> u64 {
  if values.is_empty() {
    return 0;
  }
  let mut sorted = values.to_vec();
  sorted.sort_unstable();
  let rank = sorted
    .len()
    .saturating_mul(percentile)
    .div_ceil(100)
    .saturating_sub(1)
    .min(sorted.len() - 1);
  sorted[rank]
}

fn longest_over_budget_sequence(values: &[u64], period_ns: u64) -> u32 {
  values
    .iter()
    .fold((0u32, 0u32), |(longest, current), value| {
      let current = if *value > period_ns { current + 1 } else { 0 };
      (longest.max(current), current)
    })
    .0
}

fn longest_over_budget_attempt_sequence<'a>(
  attempts: impl IntoIterator<Item = &'a [u64]>,
  period_ns: u64,
) -> u32 {
  attempts
    .into_iter()
    .map(|values| longest_over_budget_sequence(values, period_ns))
    .max()
    .unwrap_or(0)
}

fn performance_pass_key(pass: PerformancePass) -> u8 {
  match pass {
    PerformancePass::Score => 0,
    PerformancePass::Detail => 1,
  }
}

fn write_artifacts(result: &RunResult, directory: &Path) -> Result<()> {
  let root = directory.join("performance");
  fs::create_dir_all(&root).context("create performance artifact directory")?;
  let summary = result
    .performance
    .as_ref()
    .context("performance summary missing")?;
  fs::write(
    root.join("summary.json"),
    serde_json::to_vec_pretty(summary)?,
  )?;
  for pass in [PerformancePass::Score, PerformancePass::Detail] {
    let label = match pass {
      PerformancePass::Score => "score",
      PerformancePass::Detail => "detail",
    };
    let mut output = String::new();
    for scenario in &result.scenarios {
      let Some(attempt) = &scenario.performance_attempt else {
        continue;
      };
      if attempt.pass != pass || attempt.warmup {
        continue;
      }
      for step in &scenario.steps {
        let Some(performance) = &step.performance else {
          continue;
        };
        for (frame_index, timestamp_ns) in performance.presentation_timestamps_ns.iter().enumerate()
        {
          output.push_str(&serde_json::to_string(&FrameArtifact {
            scenario: &scenario.name,
            iteration: attempt.iteration,
            step: step.name.as_deref().unwrap_or("unnamed"),
            frame_index: u64::try_from(frame_index).expect("frame index fits u64"),
            timestamp_ns: *timestamp_ns,
            interval_ns: frame_index
              .checked_sub(1)
              .and_then(|index| performance.presentation_intervals_ns.get(index))
              .copied(),
            managed_allocation_delta: performance
              .managed_allocation_deltas
              .as_ref()
              .and_then(|values| values.get(frame_index))
              .copied(),
            observer_timing: performance.observer_timings.get(frame_index),
          })?);
          output.push('\n');
        }
      }
    }
    fs::write(root.join(format!("{label}-frames.ndjson")), output)?;
  }
  fs::write(root.join("report.md"), markdown(result, summary))?;
  Ok(())
}

fn markdown(result: &RunResult, summary: &PerformanceResult) -> String {
  let mut output = format!(
    "# Performance report\n\n**Legacy comparison gate — {}: {}** (goal: {})\n\nTarget: {} Hz. Sorted score values: {:?}. The unchanged gate is their median; warmups and detail passes are excluded.\n\nTiming proxy: `{}`. {}\n\nThe score pass hashes only the configured target-local witness at full pixel resolution. The detail pass reads the full framebuffer and retains the same target-local witness for response detection. Neither path downsamples.\n\n## Pacing configuration\n\n",
    summary.headline,
    summary.missed_interaction_deadlines,
    summary.goal,
    summary.target_fps,
    summary.score_attempt_values,
    summary.timing_proxy,
    summary.timing_limitation,
  );
  for value in &summary.pacing_configurations {
    output.push_str(&format!(
      "- vSync {}, software target {} fps, display refresh {}, {}x{}, debug build {}\n",
      value.v_sync_count,
      value.target_frame_rate,
      value
        .display_refresh_hz
        .map(|refresh| format!("{refresh:.2} Hz"))
        .unwrap_or_else(|| "unavailable".to_owned()),
      value.width,
      value.height,
      value.debug_build,
    ));
  }
  output.push_str(
    "\nThis is a software-capped deterministic diagnostic, not a realistic display-smoothness or uncapped-capacity benchmark.\n",
  );
  if let Some(baseline) = result
    .player_sessions
    .iter()
    .find_map(|session| session.startup_report.observer_baseline.as_ref())
  {
    output.push_str(&format!(
      "\nStartup observer plumbing baseline: {} pixels; texture setup {:.3} ms; request CPU {:.3} ms; request-to-callback {:.3} ms; callback-to-main-thread {:.3} ms. This two-by-two asynchronous probe is not a full-frame cost estimate.\n",
      baseline.observed_pixels,
      baseline.texture_setup_ns as f64 / 1_000_000.0,
      baseline.request_cpu_ns as f64 / 1_000_000.0,
      baseline.request_to_callback_ns as f64 / 1_000_000.0,
      baseline.callback_to_main_thread_ns as f64 / 1_000_000.0,
    ));
  }
  output.push_str(
    "\n## Warmup attempts\n\n| Pass | Iteration | Total misses | Response | Pacing | No response |\n|---|---:|---:|---:|---:|---:|\n",
  );
  for attempt in &summary.warmup_attempts {
    output.push_str(&format!(
      "| {:?} | {} | {} | {} | {} | {} |\n",
      attempt.pass,
      attempt.iteration,
      attempt.missed_interaction_deadlines,
      attempt.response_missed_deadlines,
      attempt.pacing_missed_deadlines,
      attempt.no_visual_response_attempts,
    ));
  }
  output.push_str(
    "\n## Score attempts\n\n| Iteration | Total misses | Response | Pacing | No response |\n|---:|---:|---:|---:|---:|\n",
  );
  for attempt in &summary.score_attempts {
    output.push_str(&format!(
      "| {} | {} | {} | {} | {} |\n",
      attempt.iteration,
      attempt.missed_interaction_deadlines,
      attempt.response_missed_deadlines,
      attempt.pacing_missed_deadlines,
      attempt.no_visual_response_attempts,
    ));
  }
  output.push_str(
    "\n## Measured interactions\n\n| Interaction | Misses (response + pacing) | No response | Response p50 / p95 / max | Frame p50 / p95 / p99 / max | Over budget | Longest stall | Completion semantic / settled | Managed allocations |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|\n",
  );
  for step in &summary.measured_steps {
    output.push_str(&format!(
      "| {} | {} ({} + {}) | {} | {:.2} / {:.2} / {:.2} ms | {:.2} / {:.2} / {:.2} / {:.2} ms | {} / {} | {} | {} / {} | {} |\n",
      step.name,
      step.missed_interaction_deadlines,
      step.response_missed_deadlines,
      step.pacing_missed_deadlines,
      step.no_visual_response_attempts,
      step.median_response_latency_ns as f64 / 1_000_000.0,
      step.p95_response_latency_ns as f64 / 1_000_000.0,
      step.maximum_response_latency_ns as f64 / 1_000_000.0,
      step.presentation_interval_distribution.p50_ns as f64 / 1_000_000.0,
      step.presentation_interval_distribution.p95_ns as f64 / 1_000_000.0,
      step.presentation_interval_distribution.p99_ns as f64 / 1_000_000.0,
      step.worst_presentation_interval_ns as f64 / 1_000_000.0,
      step.over_budget_presentation_intervals,
      step.presentation_interval_distribution.count,
      step.longest_over_budget_sequence,
      milliseconds(step.median_semantic_completion_latency_ns),
      milliseconds(step.median_settled_completion_latency_ns),
      step
        .managed_allocated_bytes
        .map(|value| format!("{value} B"))
        .unwrap_or_else(|| "unavailable".to_owned()),
    ));
  }
  output.push_str(
    "\nEach interaction's five raw attempts, including completion milestones and frame distributions, are retained in `summary.json`. Percentiles use nearest-rank samples. A frame is over budget when its proxy interval exceeds the target period.\n\n## Observer cost by pass\n\nStage timings are measured independently and may overlap; do not sum them into an application-frame total. GPU queueing, compositor work, scanout, and input-to-photon latency are unavailable.\n\n| Pass | Stage | Frames | Pixels | Total | p95 | Maximum |\n|---|---|---:|---:|---:|---:|---:|\n",
  );
  for pass in &summary.observer_passes {
    for stage in &pass.stages {
      output.push_str(&format!(
        "| {:?} | `{}` | {} | {} | {:.2} ms | {:.2} ms | {:.2} ms |\n",
        pass.pass,
        stage.stage,
        pass.observed_frames,
        pass.observed_pixels,
        stage.total_ns as f64 / 1_000_000.0,
        stage.p95_ns as f64 / 1_000_000.0,
        stage.maximum_ns as f64 / 1_000_000.0,
      ));
    }
  }
  output.push_str(
    "\n## Reactant detail hotspots\n\nExclusive/self time; inclusive time remains in `summary.json`.\n\n| Component | Calls | Self total | Self maximum |\n|---|---:|---:|---:|\n",
  );
  for hotspot in &summary.detail_hotspots {
    output.push_str(&format!(
      "| `{}` | {} | {:.2} ms | {:.2} ms |\n",
      hotspot.component,
      hotspot.calls,
      hotspot.total_self_duration_us as f64 / 1_000.0,
      hotspot.maximum_self_duration_us as f64 / 1_000.0,
    ));
  }
  output.push_str(
    "\n`score-frames.ndjson` is the comparable target-local evidence. `detail-frames.ndjson` is observer-heavier diagnostic evidence. `runner-frame` and `native-frame-complete` include nested work such as response decoding/application, so stage rows are intentionally non-additive.\n",
  );
  output
}

fn milliseconds(value: Option<u64>) -> String {
  value
    .map(|value| format!("{:.2} ms", value as f64 / 1_000_000.0))
    .unwrap_or_else(|| "n/a".to_owned())
}

fn detail_hotspots(result: &RunResult, directory: &Path) -> Result<Vec<PerformanceHotspot>> {
  let measured_steps = result
    .scenarios
    .iter()
    .filter(|scenario| {
      scenario
        .performance_attempt
        .as_ref()
        .is_some_and(|attempt| attempt.pass == PerformancePass::Detail && !attempt.warmup)
    })
    .map(|scenario| {
      (
        scenario.id.clone(),
        scenario
          .steps
          .iter()
          .filter(|step| step.performance.is_some())
          .map(|step| step.index)
          .collect::<BTreeSet<_>>(),
      )
    })
    .collect::<BTreeMap<_, _>>();
  let log_path = directory.join("logs/events.jsonl");
  let source = fs::read_to_string(&log_path)
    .with_context(|| format!("read detail logs {}", log_path.display()))?;
  detail_hotspots_from_source(&source, &measured_steps)
}

fn detail_hotspots_from_source(
  source: &str,
  measured_steps: &BTreeMap<String, BTreeSet<u32>>,
) -> Result<Vec<PerformanceHotspot>> {
  let mut totals: BTreeMap<String, (u64, u64, u64, u64)> = BTreeMap::new();
  let mut active_step: Option<(String, u32)> = None;
  for line in source.lines() {
    let value: serde_json::Value = serde_json::from_str(line)?;
    if value.get("event_name").and_then(serde_json::Value::as_str) == Some("ditto.context") {
      update_active_detail_step(&value, measured_steps, &mut active_step);
      continue;
    }
    if active_step.is_none() {
      continue;
    }
    if value.get("event_name").and_then(serde_json::Value::as_str)
      != Some("reactant.component.render")
    {
      continue;
    }
    let Some(fields) = value.get("fields") else {
      continue;
    };
    let Some(component) = fields.get("component").and_then(serde_json::Value::as_str) else {
      continue;
    };
    let Some(duration) = fields.get("duration_us").and_then(|value| {
      value
        .as_u64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    }) else {
      continue;
    };
    let Some(self_duration) = fields.get("self_duration_us").and_then(|value| {
      value
        .as_u64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    }) else {
      continue;
    };
    let entry = totals.entry(component.to_owned()).or_default();
    entry.0 += 1;
    entry.1 += self_duration;
    entry.2 = entry.2.max(self_duration);
    entry.3 += duration;
  }
  let mut hotspots = totals
    .into_iter()
    .map(
      |(
        component,
        (calls, total_self_duration_us, maximum_self_duration_us, total_inclusive_duration_us),
      )| PerformanceHotspot {
        component,
        calls,
        total_self_duration_us,
        maximum_self_duration_us,
        total_inclusive_duration_us,
      },
    )
    .collect::<Vec<_>>();
  hotspots.sort_by_key(|value| std::cmp::Reverse(value.total_self_duration_us));
  hotspots.truncate(20);
  Ok(hotspots)
}

fn update_active_detail_step(
  value: &serde_json::Value,
  measured_steps: &BTreeMap<String, BTreeSet<u32>>,
  active: &mut Option<(String, u32)>,
) {
  let Some(body) = value.get("body") else {
    return;
  };
  let Some(context) = body.get("context").and_then(serde_json::Value::as_str) else {
    return;
  };
  let Some(scenario_id) = body.get("scenario_id").and_then(serde_json::Value::as_str) else {
    return;
  };
  match context {
    "step-started" => {
      let Some(step_index) = body.get("step_index").and_then(serde_json::Value::as_u64) else {
        return;
      };
      let Ok(step_index) = u32::try_from(step_index) else {
        return;
      };
      if measured_steps
        .get(scenario_id)
        .is_some_and(|steps| steps.contains(&step_index))
      {
        *active = Some((scenario_id.to_owned(), step_index));
      }
    }
    "step-ended" => {
      let step_index = body
        .pointer("/result/index")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
      if active
        .as_ref()
        .is_some_and(|(active_scenario, active_index)| {
          active_scenario == scenario_id && Some(*active_index) == step_index
        })
      {
        *active = None;
      }
    }
    _ => {}
  }
}

fn deadline_misses(
  target_fps: u32,
  response_ns: u64,
  timestamps_ns: &[u64],
  no_visual_response: bool,
) -> Result<u64> {
  ensure!(target_fps > 0, "performance target must be positive");
  let period = 1_000_000_000 / u64::from(target_fps);
  let nearest_slot = |value: u64| value.saturating_add(period / 2) / period;
  let mut response_misses = nearest_slot(response_ns).saturating_sub(1);
  if no_visual_response {
    response_misses = response_misses.max(1);
  }
  let mut pacing_misses: u64 = 0;
  let mut previous_slot: u64 = 0;
  for timestamp in timestamps_ns {
    if *timestamp <= response_ns {
      continue;
    }
    let slot = nearest_slot(timestamp.saturating_sub(response_ns));
    if slot <= previous_slot {
      continue;
    }
    pacing_misses = pacing_misses.saturating_add(slot.saturating_sub(previous_slot + 1));
    previous_slot = slot;
  }
  Ok(response_misses.saturating_add(pacing_misses))
}

#[derive(Serialize)]
struct FrameArtifact<'a> {
  scenario: &'a str,
  iteration: u32,
  step: &'a str,
  frame_index: u64,
  timestamp_ns: u64,
  interval_ns: Option<u64>,
  managed_allocation_delta: Option<i64>,
  observer_timing: Option<&'a ObserverFrameTiming>,
}

#[cfg(test)]
mod tests {
  use std::collections::{BTreeMap, BTreeSet};

  use super::{
    deadline_misses, detail_hotspots_from_source, distribution,
    longest_over_budget_attempt_sequence, longest_over_budget_sequence,
  };

  #[test]
  fn fifty_milliseconds_at_sixty_hertz_misses_two_response_deadlines() {
    assert_eq!(deadline_misses(60, 50_000_000, &[], false).unwrap(), 2);
  }

  #[test]
  fn fast_burst_frames_do_not_recover_an_earlier_gap() {
    assert_eq!(
      deadline_misses(60, 16_666_667, &[16_666_667, 50_000_000, 51_000_000], false,).unwrap(),
      1
    );
  }

  #[test]
  fn early_burst_frames_do_not_prefill_future_presentation_slots() {
    assert_eq!(
      deadline_misses(60, 0, &[17_000_000, 18_000_000, 50_000_000], false).unwrap(),
      1
    );
  }

  #[test]
  fn no_visual_response_always_misses_at_least_one_deadline() {
    assert_eq!(
      deadline_misses(60, 16_666_667, &[16_666_667], true).unwrap(),
      1
    );
  }

  #[test]
  fn distribution_uses_nearest_rank_without_interpolation() {
    let value = distribution(&[50, 10, 40, 20, 30]);
    assert_eq!(value.minimum_ns, 10);
    assert_eq!(value.p50_ns, 30);
    assert_eq!(value.p95_ns, 50);
    assert_eq!(value.p99_ns, 50);
    assert_eq!(value.maximum_ns, 50);
  }

  #[test]
  fn distribution_minimum_is_not_the_first_percentile() {
    let value = distribution(&(0..=100).collect::<Vec<_>>());
    assert_eq!(value.minimum_ns, 0);
    assert_eq!(value.p50_ns, 50);
  }

  #[test]
  fn longest_stall_counts_only_consecutive_over_budget_intervals() {
    assert_eq!(
      longest_over_budget_sequence(&[17, 18, 16, 25, 26, 27, 15], 16),
      3
    );
  }

  #[test]
  fn longest_stall_does_not_join_separate_attempts() {
    let first = vec![17, 18];
    let second = vec![19, 20];
    assert_eq!(
      longest_over_budget_attempt_sequence([first.as_slice(), second.as_slice()], 16,),
      2
    );
  }

  #[test]
  fn detail_hotspots_include_only_measured_step_contexts() {
    let lines = [
      r#"{"event_name":"reactant.component.render","fields":{"component":"startup","duration_us":1000,"self_duration_us":1000}}"#,
      r#"{"event_name":"ditto.context","body":{"context":"step-started","scenario_id":"detail","step_index":1}}"#,
      r#"{"event_name":"reactant.component.render","fields":{"component":"measured","duration_us":"20","self_duration_us":"10"}}"#,
      r#"{"event_name":"ditto.context","body":{"context":"step-ended","scenario_id":"detail","result":{"index":1}}}"#,
      r#"{"event_name":"reactant.component.render","fields":{"component":"after","duration_us":1000,"self_duration_us":1000}}"#,
    ]
    .join("\n");
    let measured = BTreeMap::from([("detail".to_owned(), BTreeSet::from([1]))]);
    let hotspots = detail_hotspots_from_source(&lines, &measured).unwrap();
    assert_eq!(hotspots.len(), 1);
    assert_eq!(hotspots[0].component, "measured");
    assert_eq!(hotspots[0].total_self_duration_us, 10);
    assert_eq!(hotspots[0].total_inclusive_duration_us, 20);
  }
}
