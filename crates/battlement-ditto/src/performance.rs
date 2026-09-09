use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::Path,
};

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::wire::{
  job::PerformancePass,
  result::{MeasuredStepPerformance, PerformanceHotspot, PerformanceResult, RunResult},
};

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
  let mut by_step: BTreeMap<String, Vec<&crate::wire::lifecycle::StepPerformance>> =
    BTreeMap::new();
  for (scenario, attempt) in &attempts {
    if attempt.warmup || attempt.pass != PerformancePass::Score {
      continue;
    }
    let mut attempt_total = 0;
    for step in &scenario.steps {
      if let Some(performance) = &step.performance {
        ensure!(
          performance.target_fps == target_fps,
          "measured step disagrees about the performance target"
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
        by_step
          .entry(
            step
              .name
              .clone()
              .unwrap_or_else(|| format!("step-{}", step.index)),
          )
          .or_default()
          .push(performance);
      }
    }
    score_values.push(attempt_total);
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
  score_values.sort_unstable();
  let headline = median(&score_values);
  let measured_steps = by_step
    .into_iter()
    .map(|(name, values)| {
      let mut response = values
        .iter()
        .map(|value| value.response_latency_ns)
        .collect::<Vec<_>>();
      response.sort_unstable();
      MeasuredStepPerformance {
        name,
        attempts: u32::try_from(values.len()).expect("score attempt count is bounded"),
        missed_interaction_deadlines: values
          .iter()
          .map(|value| value.missed_interaction_deadlines)
          .sum(),
        median_response_latency_ns: median(&response),
        worst_presentation_interval_ns: values
          .iter()
          .flat_map(|value| value.presentation_intervals_ns.iter())
          .copied()
          .max()
          .unwrap_or(0),
        managed_allocated_bytes: values
          .iter()
          .flat_map(|value| value.managed_allocation_deltas.iter())
          .sum(),
        no_visual_response_attempts: u32::try_from(
          values
            .iter()
            .filter(|value| value.no_visual_response)
            .count(),
        )
        .expect("score attempt count is bounded"),
      }
    })
    .collect::<Vec<_>>();
  let detail_hotspots = detail_hotspots(result, directory)?;
  ensure!(
    measured_steps
      .iter()
      .all(|step| { step.attempts == crate::profile_commands::MEASURED_ITERATIONS }),
    "profile measured steps do not all have five attempts"
  );
  result.performance = Some(PerformanceResult {
    headline: format!("Missed {target_fps} Hz interaction deadlines"),
    target_fps,
    missed_interaction_deadlines: headline,
    goal: 0,
    measured_score_attempts: u32::try_from(score_values.len())
      .expect("score attempt count is bounded"),
    score_attempt_values: score_values,
    measured_steps,
    detail_hotspots,
  });
  write_artifacts(result, directory)
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
              .get(frame_index)
              .copied()
              .unwrap_or(0),
          })?);
          output.push('\n');
        }
      }
    }
    fs::write(root.join(format!("{label}-frames.ndjson")), output)?;
  }
  fs::write(root.join("report.md"), markdown(summary))?;
  Ok(())
}

fn markdown(summary: &PerformanceResult) -> String {
  let mut output = format!(
    "# Performance report\n\n**{}: {}** (goal: {})\n\nTarget: {} Hz. Score attempts: {:?}. The headline is their median. Warmups and detail passes are excluded.\n\n## Measured hotspots\n\n| Interaction | Misses across attempts | No-response attempts | Median response | Worst frame | Managed allocations |\n|---|---:|---:|---:|---:|---:|\n",
    summary.headline,
    summary.missed_interaction_deadlines,
    summary.goal,
    summary.target_fps,
    summary.score_attempt_values,
  );
  for step in &summary.measured_steps {
    output.push_str(&format!(
      "| {} | {} | {} | {:.2} ms | {:.2} ms | {} B |\n",
      step.name,
      step.missed_interaction_deadlines,
      step.no_visual_response_attempts,
      step.median_response_latency_ns as f64 / 1_000_000.0,
      step.worst_presentation_interval_ns as f64 / 1_000_000.0,
      step.managed_allocated_bytes,
    ));
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
    "\n`score-frames.ndjson` is the comparable low-overhead evidence. `detail-frames.ndjson` is diagnostic only.\n",
  );
  output
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

fn median(values: &[u64]) -> u64 {
  values[values.len() / 2]
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
  managed_allocation_delta: i64,
}

#[cfg(test)]
mod tests {
  use std::collections::{BTreeMap, BTreeSet};

  use super::{deadline_misses, detail_hotspots_from_source};

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
