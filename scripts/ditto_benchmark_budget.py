#!/usr/bin/env python3

"""Enforce Ditto's fingerprint, cold-launch, and warm-watch budgets."""

from __future__ import annotations

import argparse
import fcntl
import json
import math
import os
from pathlib import Path
import selectors
import signal
import statistics
import subprocess
import sys
import tempfile
import time
from typing import Any

import ditto_benchmark as benchmark


HASH_BUDGET_MS = 250
COLD_BUDGET_MS = 20_000
WARM_BUDGET_MS = 5_000
HASH_REPETITIONS = 20
COLD_REPETITIONS = 10
WARM_REPETITIONS = 20


def distribution(values: list[int]) -> dict[str, int | float]:
    """Return minimum, median, nearest-rank p95, and maximum observations."""
    if not values:
        raise RuntimeError("a performance distribution has no observations")
    ordered = sorted(values)
    return {
        "minimum": ordered[0],
        "median": statistics.median(ordered),
        "p95": ordered[math.ceil(len(ordered) * 0.95) - 1],
        "maximum": ordered[-1],
    }


def execution_ms(summary: dict[str, Any]) -> int:
    """Measure execution while retaining source hashing as a separate observation."""
    return summary["duration_ms"] - summary["non_normative_ms"]["build"]


def summarize_repetition(samples: list[dict[str, Any]]) -> dict[str, Any]:
    """Summarize one serialized pass through all fixed benchmark constituents."""
    if sum(item["scenario_count"] for item in samples) != 20:
        raise RuntimeError("performance repetition did not execute exactly 20 scenarios")
    if sum(item["checkpoint_count"] for item in samples) != 40:
        raise RuntimeError("performance repetition did not capture exactly 40 screenshots")
    phases = sorted({name for item in samples for name in item["phases_ms"]})
    return {
        "measurement_unit": "maximum constituent public Ditto run",
        "execution_ms": max(execution_ms(item) for item in samples),
        "source_hashing_ms": max(item["phases_ms"]["source_hashing_ms"] for item in samples),
        "phases_ms": {
            name: max(
                item["phases_ms"].get(name) or 0
                for item in samples
            )
            for name in phases
        },
        "scenario_count": 20,
        "checkpoint_count": 40,
        "samples": samples,
    }


def distributions(repetitions: list[dict[str, Any]]) -> dict[str, Any]:
    """Aggregate phase distributions without discarding any repetition."""
    phases = sorted({name for item in repetitions for name in item["phases_ms"]})
    return {
        "execution_ms": distribution([item["execution_ms"] for item in repetitions]),
        "phases_ms": {
            name: distribution([int(item["phases_ms"].get(name) or 0) for item in repetitions])
            for name in phases
        },
    }


def enforce(
    hashing: list[int], cold: list[dict[str, Any]], warm: list[dict[str, Any]]
) -> dict[str, Any]:
    """Reject incomplete evidence or any observed maximum over its budget."""
    if len(hashing) != HASH_REPETITIONS:
        raise RuntimeError(f"source hashing requires exactly {HASH_REPETITIONS} repetitions")
    if len(cold) != COLD_REPETITIONS:
        raise RuntimeError(f"cold launch requires exactly {COLD_REPETITIONS} repetitions")
    if len(warm) != WARM_REPETITIONS:
        raise RuntimeError(f"warm watch requires exactly {WARM_REPETITIONS} repetitions")
    result = {
        "source_hashing_ms": distribution(hashing),
        "cold": distributions(cold),
        "warm": distributions(warm),
    }
    failures = [
        ("source hashing", result["source_hashing_ms"]["maximum"], HASH_BUDGET_MS),
        ("cold launch", result["cold"]["execution_ms"]["maximum"], COLD_BUDGET_MS),
        ("warm watch", result["warm"]["execution_ms"]["maximum"], WARM_BUDGET_MS),
    ]
    for name, observed, budget_ms in failures:
        if observed > budget_ms:
            raise RuntimeError(f"{name} maximum {observed} ms exceeds {budget_ms} ms")
    return result


def cold_repetitions(
    definition: dict[str, Any], binary: Path, output: Path
) -> list[dict[str, Any]]:
    repetitions: list[dict[str, Any]] = []
    for repetition in range(1, COLD_REPETITIONS + 1):
        samples = []
        for sample in definition["samples"]:
            result, resources = benchmark.execute_suite(
                binary, sample, sample["scenarios"], output / f"{repetition:02d}"
            )
            summary = benchmark.validate_result(result, sample, sample["scenarios"])
            samples.append({**summary, **resources, "sample": sample["name"]})
        repetitions.append(summarize_repetition(samples))
    return repetitions


def next_watch_result(
    process: subprocess.Popen[str], selector: selectors.BaseSelector, timeout_seconds: float
) -> dict[str, Any]:
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"Ditto watch exited before a result with status {process.returncode}")
        events = selector.select(min(1.0, deadline - time.monotonic()))
        if not events:
            continue
        line = process.stdout.readline() if process.stdout is not None else ""
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict) and value.get("command") == "run":
            return value
    raise RuntimeError("Ditto watch did not produce a result before its deadline")


def wait_for_watch_ready(process: subprocess.Popen[str], stderr_path: Path) -> None:
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError("Ditto watch exited before observing its inputs")
        if stderr_path.is_file() and "DITTO_REVIEW_URL=" in stderr_path.read_text(encoding="utf-8"):
            time.sleep(0.25)
            return
        time.sleep(0.05)
    raise RuntimeError("Ditto watch did not become ready to observe changes")


def watch_sample(
    binary: Path, sample: dict[str, Any], output: Path
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    output.mkdir(parents=True, exist_ok=True)
    stderr_path = output / f"{sample['name']}.stderr.log"
    command = [
        str(binary), "--config", sample["config"], "run", "--profile", "macos",
        "--no-build", "--json", "--watch",
        *[scenario["name"] for scenario in sample["scenarios"]],
    ]
    with stderr_path.open("w", encoding="utf-8") as stderr:
        process = subprocess.Popen(
            command, cwd=benchmark.REPOSITORY_ROOT, stdout=subprocess.PIPE,
            stderr=stderr, text=True, start_new_session=True,
        )
        if process.stdout is None:
            raise RuntimeError("Ditto watch stdout is unavailable")
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        try:
            initial = next_watch_result(process, selector, 120)
            warm = []
            config_path = benchmark.REPOSITORY_ROOT / sample["config"]
            wait_for_watch_ready(process, stderr_path)
            for _ in range(WARM_REPETITIONS):
                os.utime(config_path)
                warm.append(next_watch_result(process, selector, 120))
            return initial, warm
        finally:
            selector.close()
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGINT)
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)


def warm_repetitions(
    definition: dict[str, Any], binary: Path, output: Path
) -> tuple[list[dict[str, Any]], list[int]]:
    by_sample: list[list[dict[str, Any]]] = []
    hashing: list[list[int]] = []
    for sample in definition["samples"]:
        initial, results = watch_sample(binary, sample, output)
        benchmark.validate_result(initial, sample, sample["scenarios"], watch=True)
        summaries = [
            benchmark.validate_result(result, sample, sample["scenarios"], watch=True)
            for result in results
        ]
        if any(summary["cycle"] <= 1 for summary in summaries):
            raise RuntimeError(f"{sample['name']} did not reuse its warm watch session")
        by_sample.append([{**summary, "sample": sample["name"]} for summary in summaries])
        hashing.append([summary["phases_ms"]["source_hashing_ms"] for summary in summaries])
    warm = [summarize_repetition([items[index] for items in by_sample]) for index in range(WARM_REPETITIONS)]
    source_hashing = [max(items[index] for items in hashing) for index in range(HASH_REPETITIONS)]
    return warm, source_hashing


def full_samples(
    definition: dict[str, Any], binary: Path, output: Path
) -> list[dict[str, Any]]:
    results = []
    for sample in definition["samples"]:
        scenarios = benchmark.full_scenarios(sample)
        result, resources = benchmark.execute_suite(binary, sample, scenarios, output)
        summary = benchmark.validate_result(result, sample, scenarios)
        results.append({**summary, **resources, "sample": sample["name"]})
    return results


def before_profile(path: Path | None) -> dict[str, Any] | None:
    if path is None:
        return None
    report = json.loads(path.read_text(encoding="utf-8"))
    if report.get("status") != "passed" or report.get("scenario_count") != 20:
        raise RuntimeError("the before profile is not a passing fixed benchmark report")
    return {"host": report["host"], "ordinary": report["ordinary"]}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--definition", type=Path, default=benchmark.DEFAULT_DEFINITION)
    parser.add_argument("--ditto", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--before", type=Path)
    arguments = parser.parse_args()
    definition = benchmark.load_definition(arguments.definition)
    benchmark.ensure_host()
    binary = arguments.ditto.resolve()
    output = arguments.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    lock_path = Path(tempfile.gettempdir()) / "battlement-ditto-performance.lock"
    with lock_path.open("w") as lock, benchmark.unity_editor_lease():
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        cold = cold_repetitions(definition, binary, output / "cold")
        warm, hashing = warm_repetitions(definition, binary, output / "warm")
        samples = full_samples(definition, binary, output / "full-samples")
    report = {
        "schema": 1,
        "benchmark": definition["name"],
        "status": "passed",
        "host": benchmark.host_facts(),
        "budgets": enforce(hashing, cold, warm),
        "before_profile": before_profile(arguments.before),
        "after_profile": {"cold": cold[0], "warm": warm[0]},
        "cold_repetitions": cold,
        "warm_repetitions": warm,
        "full_samples": samples,
    }
    report_path = output / "budget.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
