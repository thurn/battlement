#!/usr/bin/env python3

"""Measure Ditto's fixed ordinary run and complete sample suites."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tempfile
import time
import tomllib
from typing import Any

from platform_support import user_cache_path
from visual_capture_lib import unity_editor_lease


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_DEFINITION = REPOSITORY_ROOT / "benchmarks/ditto/ordinary.json"
REQUIRED_PHASES = {"build", "launch", "startup", "scenarios", "reset", "durability", "cleanup"}
REQUIRED_TIMINGS = {
    "startup_ms", "settle_ms", "capture_ms", "baseline_read_ms", "odiff_ms",
    "reset_ms", "durability_ms",
}


def load_definition(path: Path) -> dict[str, Any]:
    """Load and verify the immutable 20-scenario, 40-checkpoint definition."""
    definition = json.loads(path.read_text(encoding="utf-8"))
    if definition.get("schema") != 1 or definition.get("profile") != "macos":
        raise RuntimeError("benchmark schema and profile must be canonical")
    if (definition.get("width"), definition.get("height")) != (1280, 720):
        raise RuntimeError("benchmark display must be exactly 1280x720")
    if definition.get("host_class") != "apple-silicon":
        raise RuntimeError("benchmark host class must be Apple silicon")
    if definition.get("competing_load_policy") != "exclusive-unity-and-ditto":
        raise RuntimeError("benchmark must exclude competing Unity and Ditto work")
    samples = definition.get("samples", [])
    scenarios = [scenario for sample in samples for scenario in sample.get("scenarios", [])]
    checkpoints = [name for scenario in scenarios for name in scenario.get("checkpoints", [])]
    if len(samples) != 5 or len(scenarios) != 20 or len(checkpoints) != 40:
        raise RuntimeError("benchmark shape must be exactly five samples, 20 scenarios, and 40 screenshots")
    if len({scenario["name"] for scenario in scenarios}) != len(scenarios):
        raise RuntimeError("benchmark scenario names must be globally unique")
    for sample in samples:
        verify_sample_definition(sample, definition)
    return definition


def verify_sample_definition(sample: dict[str, Any], definition: dict[str, Any]) -> None:
    config_path = REPOSITORY_ROOT / sample["config"]
    suite = tomllib.loads(config_path.read_text(encoding="utf-8"))
    if suite.get("name") != sample["name"]:
        raise RuntimeError(f"benchmark sample identity changed: {sample['name']}")
    display = suite["profiles"][definition["profile"]]["display"]
    if (display["width"], display["height"]) != (definition["width"], definition["height"]):
        raise RuntimeError(f"benchmark sample display changed: {sample['name']}")
    actual = {
        scenario["name"]: [step["screenshot"]["name"] for step in scenario["steps"] if "screenshot" in step]
        for scenario in suite["scenarios"]
    }
    expected = {scenario["name"]: scenario["checkpoints"] for scenario in sample["scenarios"]}
    if any(actual.get(name) != checkpoints for name, checkpoints in expected.items()):
        raise RuntimeError(f"benchmark scenario or checkpoint changed: {sample['name']}")


def validate_result(
    result: dict[str, Any],
    sample: dict[str, Any],
    scenarios: list[dict[str, Any]],
    watch: bool = False,
) -> dict[str, Any]:
    """Reject invalid, stale, downloaded, excluded, or incompletely timed runs."""
    if result.get("status") != "passed" or result.get("command") != "run":
        raise RuntimeError(f"{sample['name']} benchmark run did not pass")
    build = result.get("build") or {}
    if build.get("disposition") != "reused":
        raise RuntimeError(f"{sample['name']} benchmark did not reuse an exact build")
    phases = {phase.get("name"): phase for phase in result.get("phases", [])}
    required_phases = REQUIRED_PHASES
    if watch:
        required_phases = required_phases - {"cleanup"}
    if result.get("cycle", 0) > 1:
        required_phases = REQUIRED_PHASES - {"launch", "cleanup"}
    missing = required_phases - phases.keys()
    if missing:
        raise RuntimeError(f"{sample['name']} result is missing phases: {sorted(missing)}")
    if phases.get("baseline-download", {}).get("duration_ms", 0) != 0:
        raise RuntimeError(f"{sample['name']} downloaded a baseline during measurement")
    reached = result.get("scenarios", [])
    if [item.get("name") for item in reached] != [item["name"] for item in scenarios]:
        raise RuntimeError(f"{sample['name']} scenario order or selection changed")
    for expected, actual in zip(scenarios, reached, strict=True):
        if actual.get("status") != "passed":
            raise RuntimeError(f"{sample['name']} scenario did not pass: {expected['name']}")
        timings = actual.get("timings", {})
        absent = [name for name in REQUIRED_TIMINGS if timings.get(name) is None]
        if absent:
            raise RuntimeError(f"{sample['name']} scenario is missing timings: {absent}")
        if timings.get("baseline_download_ms") not in (None, 0):
            raise RuntimeError(f"{sample['name']} downloaded a baseline during measurement")
        checkpoints = [
            step["screenshot"]["checkpoint"]
            for step in actual.get("steps", [])
            if (step.get("screenshot") or {}).get("status") == "captured"
        ]
        if checkpoints != expected["checkpoints"]:
            raise RuntimeError(f"{sample['name']} checkpoint order changed: {expected['name']}")
        images = [
            step["screenshot"]["actual"]
            for step in actual.get("steps", [])
            if (step.get("screenshot") or {}).get("status") == "captured"
        ]
        if any((image["width"], image["height"]) != (1280, 720) for image in images):
            raise RuntimeError(f"{sample['name']} captured a non-canonical image")
    return summarize_result(result)


def summarize_result(result: dict[str, Any]) -> dict[str, Any]:
    timings = [scenario["timings"] for scenario in result["scenarios"]]
    phases = {phase["name"]: phase["duration_ms"] for phase in result["phases"]}
    measured = {
        "source_hashing_ms": result["build"]["duration_ms"],
        "cold_launch_ms": (
            None if result.get("cycle", 0) > 1
            else phases["launch"] + phases["startup"]
        ),
        "warm_watch_execution_ms": (
            result["duration_ms"] if result.get("cycle", 0) > 1 else None
        ),
        "setup_ms": sum(item["startup_ms"] for item in timings),
        "settle_ms": sum(item["settle_ms"] for item in timings),
        "capture_ms": sum(item["capture_ms"] for item in timings),
        "baseline_read_ms": sum(item["baseline_read_ms"] for item in timings),
        "odiff_ms": sum(item["odiff_ms"] for item in timings),
        "reset_ms": sum(item["reset_ms"] for item in timings),
        "durability_ms": sum(item["durability_ms"] for item in timings),
    }
    return {
        "run_id": result["run_id"],
        "cycle": result["cycle"],
        "source_fingerprint": result["build"]["source_fingerprint"],
        "build_fingerprint": result["build"]["fingerprint"],
        "scenario_count": len(result["scenarios"]),
        "checkpoint_count": sum(
            (step.get("screenshot") or {}).get("status") == "captured"
            for scenario in result["scenarios"] for step in scenario["steps"]
        ),
        "duration_ms": result["duration_ms"],
        "phases_ms": measured,
        "excluded_ms": 0,
        "non_normative_ms": {
            name: phases.get(name, 0) for name in ("build", "baseline-download", "simulator-boot")
        },
    }


def command_version(command: list[str]) -> str | None:
    try:
        completed = subprocess.run(command, check=True, capture_output=True, text=True)
        return completed.stdout.strip() or completed.stderr.strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def host_facts() -> dict[str, Any]:
    """Return all environment facts required to interpret a measurement."""
    project_version = (REPOSITORY_ROOT / "ProjectSettings/ProjectVersion.txt").read_text()
    unity = re.search(r"m_EditorVersion: (.+)", project_version)
    filesystem = next(
        (
            line.split("(", 1)[1].split(",", 1)[0]
            for line in (command_version(["mount"]) or "").splitlines()
            if " on /System/Volumes/Data " in line
        ),
        None,
    )
    power = command_version(["pmset", "-g", "custom"])
    odiff_binary = (
        user_cache_path("Battlement", "ditto")
        / "tools/odiff/4.5.0/odiff-macos-arm64"
    )
    return {
        "host_class": "apple-silicon",
        "hardware_model": command_version(["sysctl", "-n", "hw.model"]),
        "cpu_count": os.cpu_count(),
        "memory_bytes": int(command_version(["sysctl", "-n", "hw.memsize"]) or 0),
        "macos": platform.mac_ver()[0],
        "unity": unity.group(1) if unity else None,
        "rust": command_version(["rustc", "--version"]),
        "odiff": command_version([str(odiff_binary), "--version"]),
        "filesystem": filesystem,
        "power_mode": power,
        "competing_load_policy": "exclusive-unity-and-ditto",
    }


def ensure_host() -> None:
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise RuntimeError("Ditto performance measurement requires an Apple silicon macOS host")
    expected = os.environ.get("DITTO_BENCHMARK_HOST_CLASS", "apple-silicon")
    if expected != "apple-silicon":
        raise RuntimeError("Tollgate benchmark lane is not pinned to Apple silicon")
    processes = subprocess.run(
        ["ps", "-axo", "pid=,comm="], check=True, capture_output=True, text=True
    ).stdout.splitlines()
    competitors = [line for line in processes if re.search(r"/(Unity|ditto)$", line.strip())]
    if competitors:
        raise RuntimeError("competing Unity or Ditto process is active: " + ", ".join(competitors))


def resource_usage(path: Path) -> dict[str, Any]:
    values: dict[str, float] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        leading = re.match(r"\s*([0-9.]+)\s+(.+)", line)
        trailing = re.match(r"\s*(real|user|sys)\s+([0-9.]+)\s*$", line)
        if leading:
            values[leading.group(2)] = float(leading.group(1))
        elif trailing:
            values[trailing.group(1)] = float(trailing.group(2))
    return {
        "cpu_ms": round(1000 * (values.get("user", 0) + values.get("sys", 0))),
        "peak_memory_bytes": round(values.get("maximum resident set size", 0)),
    }


def execute_suite(
    binary: Path,
    sample: dict[str, Any],
    scenarios: list[dict[str, Any]],
    output: Path,
    no_build: bool = True,
) -> tuple[dict[str, Any], dict[str, Any]]:
    output.mkdir(parents=True, exist_ok=True)
    result_path = output / f"{sample['name']}.result.json"
    resource_path = output / f"{sample['name']}.resources.txt"
    command = [
        "/usr/bin/time", "-lp", "-o", str(resource_path), str(binary),
        "--config", sample["config"], "run", "--profile", "macos",
    ]
    if no_build:
        command.append("--no-build")
    command.extend(("--json", "--output", str(result_path)))
    command.extend(scenario["name"] for scenario in scenarios)
    started = time.monotonic_ns()
    completed = subprocess.run(command, cwd=REPOSITORY_ROOT, capture_output=True, text=True)
    wall_ms = (time.monotonic_ns() - started) // 1_000_000
    if completed.returncode != 0:
        raise RuntimeError(f"{sample['name']} Ditto failed: {completed.stderr.strip()}")
    result = json.loads(result_path.read_text(encoding="utf-8"))
    resources = resource_usage(resource_path)
    resources["wall_ms"] = wall_ms
    resources["unreported_ms"] = max(0, wall_ms - result["duration_ms"])
    return result, resources


def full_scenarios(sample: dict[str, Any]) -> list[dict[str, Any]]:
    suite = tomllib.loads((REPOSITORY_ROOT / sample["config"]).read_text(encoding="utf-8"))
    return [
        {
            "name": scenario["name"],
            "checkpoints": [step["screenshot"]["name"] for step in scenario["steps"] if "screenshot" in step],
        }
        for scenario in suite["scenarios"]
    ]


def measure(
    definition: dict[str, Any], binary: Path, output: Path, prepare: bool
) -> dict[str, Any]:
    ensure_host()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise RuntimeError("--ditto must name a previously compiled executable")
    output.mkdir(parents=True, exist_ok=False)
    fixed: list[dict[str, Any]] = []
    samples: list[dict[str, Any]] = []
    preparation: list[dict[str, Any]] = []
    lock_path = Path(tempfile.gettempdir()) / "battlement-ditto-performance.lock"
    with lock_path.open("w") as lock, unity_editor_lease():
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if prepare:
            for sample in definition["samples"]:
                result, resources = execute_suite(
                    binary, sample, sample["scenarios"], output / "preparation", no_build=False
                )
                if result.get("status") != "passed":
                    raise RuntimeError(f"{sample['name']} benchmark preparation failed")
                preparation.append({**summarize_result(result), **resources})
        for sample in definition["samples"]:
            result, resources = execute_suite(binary, sample, sample["scenarios"], output)
            fixed.append({**validate_result(result, sample, sample["scenarios"]), **resources})
        for sample in definition["samples"]:
            scenarios = full_scenarios(sample)
            result, resources = execute_suite(binary, sample, scenarios, output / "full-samples")
            samples.append({**validate_result(result, sample, scenarios), **resources})
    return {
        "schema": 1,
        "benchmark": definition["name"],
        "status": "passed",
        "host": host_facts(),
        "preparation": preparation,
        "ordinary": fixed,
        "full_samples": samples,
        "scenario_count": sum(item["scenario_count"] for item in fixed),
        "checkpoint_count": sum(item["checkpoint_count"] for item in fixed),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--definition", type=Path, default=DEFAULT_DEFINITION)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--check-host", action="store_true")
    parser.add_argument("--ditto", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--prepare", action="store_true")
    arguments = parser.parse_args()
    definition = load_definition(arguments.definition)
    if arguments.check:
        if arguments.check_host:
            ensure_host()
        print(json.dumps({"status": "passed", "scenarios": 20, "checkpoints": 40}))
        return
    if arguments.ditto is None or arguments.output is None:
        parser.error("--ditto and --output are required for measurement")
    report = measure(
        definition, arguments.ditto.resolve(), arguments.output.resolve(), arguments.prepare
    )
    report_path = arguments.output / "benchmark.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
