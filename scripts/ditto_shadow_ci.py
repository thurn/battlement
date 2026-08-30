#!/usr/bin/env python3

"""Run and retain the Ditto shadow-CI matrix."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tarfile
import tomllib
from typing import Any


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
ARTIFACT_ROOT = REPOSITORY_ROOT / "artifacts/ditto-ci"
DITTO = Path(os.environ.get("DITTO_SHADOW_BINARY", REPOSITORY_ROOT / "target/release/ditto"))
SAMPLES = ("basic", "tictactoe", "reactant", "chess", "ui")
ADAPTER_TESTS = {
    "webgl": "webgl_capture_tests",
    "ios": "ios_simulator_tests",
}
UNITY_GENERATED_DIRECTORIES = ("Library", "Logs", "Temp", "UserSettings")


def command(
    arguments: list[str], *, check: bool = True, environment: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    """Run a repository command while preserving output in the CI log."""
    result = subprocess.run(
        arguments, cwd=REPOSITORY_ROOT, capture_output=True, text=True, env=environment
    )
    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)
    if check and result.returncode != 0:
        raise RuntimeError(f"command exited with {result.returncode}: {' '.join(arguments)}")
    return result


def artifact_directory(name: str) -> Path:
    path = ARTIFACT_ROOT / name
    shutil.rmtree(path, ignore_errors=True)
    path.mkdir(parents=True)
    return path


def run_directory(stderr: str) -> Path | None:
    values = [line.removeprefix("DITTO_RUN_DIR=") for line in stderr.splitlines()
              if line.startswith("DITTO_RUN_DIR=")]
    return Path(values[-1]) if values else None


def retain_directory(source: Path, archive: Path, name: str) -> None:
    """Retain a directory as one bounded artifact-discovery entry."""
    if not source.is_dir():
        raise RuntimeError(f"cannot retain missing directory: {source}")
    with tarfile.open(archive, "w:gz") as output:
        output.add(source, arcname=name)
    shutil.rmtree(source)


def retain_run(source: Path | None, destination: Path) -> None:
    if source is None:
        return
    if not source.is_dir():
        raise RuntimeError(f"Ditto reported a missing run directory: {source}")
    if not (source / "logs/events.jsonl").is_file():
        raise RuntimeError("Ditto run directory omitted its ordered event log")
    retained = destination / "run"
    shutil.copytree(source, retained, dirs_exist_ok=True)
    retain_directory(retained, destination / "run.tar.gz", "run")


def load_result(path: Path) -> dict[str, Any]:
    if not path.is_file():
        raise RuntimeError("Ditto did not write its terminal result")
    return json.loads(path.read_text(encoding="utf-8"))


def validate_result(
    result: dict[str, Any], sample: str, expected: list[str], require_reuse: bool
) -> None:
    if result.get("status") != "passed":
        raise RuntimeError(f"{sample} suite status is {result.get('status', 'missing')}")
    disposition = (result.get("build") or {}).get("disposition")
    if require_reuse and disposition != "reused":
        raise RuntimeError(f"{sample} used {disposition or 'no'} player build")
    scenarios = result.get("scenarios") or []
    if [item.get("name") for item in scenarios] != expected:
        raise RuntimeError(f"{sample} did not execute its exact scenario inventory")
    if any(item.get("status") != "passed" for item in scenarios):
        raise RuntimeError(f"{sample} has an incomplete scenario outcome")


def clean_unity_workspace(sample: str) -> None:
    """Remove generated Unity project state before Tollgate scans artifacts."""
    root = REPOSITORY_ROOT / "samples" / sample
    for name in UNITY_GENERATED_DIRECTORIES:
        shutil.rmtree(root / name, ignore_errors=True)


def execute_sample(sample: str, *, prepare: bool) -> dict[str, Any]:
    output = artifact_directory(f"prepare-{sample}" if prepare else sample)
    result_path = output / "result.json"
    suite = tomllib.loads(
        (REPOSITORY_ROOT / f"samples/{sample}/ditto.toml").read_text()
    )
    expected = [scenario["name"] for scenario in suite["scenarios"]]
    arguments = [
        str(DITTO), "--config", f"samples/{sample}/ditto.toml", "run",
        "--profile", "macos", "--json", "--output", str(result_path),
    ]
    if prepare:
        expected = expected[:1]
        arguments.append(expected[0])
    else:
        arguments.append("--no-build")
    completed = command(arguments, check=False)
    source = run_directory(completed.stderr)
    retain_run(source, output)
    result = load_result(result_path)
    validate_result(result, sample, expected, require_reuse=not prepare)
    if completed.returncode != 0:
        raise RuntimeError(f"{sample} Ditto suite exited with {completed.returncode}")
    return {
        "sample": sample,
        "run_id": result["run_id"],
        "build": result["build"]["disposition"],
        "result": str(result_path),
    }


def prepare() -> None:
    command(["cargo", "build", "--release", "-p", "battlement-ditto"])
    samples = []
    for sample in SAMPLES:
        try:
            samples.append(execute_sample(sample, prepare=True))
        finally:
            clean_unity_workspace(sample)
    report = {
        "schema": 1,
        "status": "passed",
        "host": platform_report(),
        "samples": samples,
    }
    (ARTIFACT_ROOT / "preparation.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def sample_suite(sample: str) -> None:
    if sample not in SAMPLES:
        raise RuntimeError(f"unknown sample: {sample}")
    try:
        execute_sample(sample, prepare=False)
    finally:
        clean_unity_workspace(sample)


def adapter(name: str) -> None:
    test = ADAPTER_TESTS.get(name)
    if test is None:
        raise RuntimeError(f"unknown adapter: {name}")
    output = artifact_directory(f"adapter-{name}")
    completed = command([
        "cargo", "test", "-p", "battlement-ditto", "--test", test, "--", "--nocapture",
    ])
    report = {
        "schema": 1,
        "status": "passed",
        "adapter": name,
        "host": platform_report(),
        "test": test,
        "output": completed.stdout + completed.stderr,
    }
    (output / "report.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def platform_report() -> dict[str, str]:
    architecture = platform.machine()
    if platform.system() != "Darwin" or architecture not in {"arm64", "x86_64"}:
        raise RuntimeError("Ditto shadow CI requires an arm64 or x86_64 macOS host")
    return {"system": "macos", "architecture": architecture}


def performance() -> None:
    output = artifact_directory("performance")
    measurement = output / "measurement"
    environment = os.environ.copy()
    environment["DITTO_CONTAINED_SESSION"] = "1"
    command([
        sys.executable, "scripts/ditto_benchmark_budget.py", "--ditto", str(DITTO),
        "--output", str(measurement),
    ], environment=environment)
    command(["git", "update-index", "--refresh"])
    command(["git", "diff-files", "--quiet"])
    retain_directory(measurement, output / "measurement.tar.gz", "measurement")


def publish() -> None:
    default_branch = os.environ.get("DITTO_DEFAULT_BRANCH", "master")
    branch = os.environ.get("DITTO_CI_BRANCH")
    if branch is None:
        branch = command(["git", "branch", "--show-current"]).stdout.strip()
    if branch != default_branch:
        print(f"baseline publication skipped on {branch or 'detached HEAD'}")
        return
    for sample in SAMPLES:
        command([
            str(DITTO), "--config", f"samples/{sample}/ditto.toml",
            "storage", "publish",
        ])


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    subcommands.add_parser("prepare")
    sample_parser = subcommands.add_parser("sample")
    sample_parser.add_argument("name", choices=SAMPLES)
    adapter_parser = subcommands.add_parser("adapter")
    adapter_parser.add_argument("name", choices=ADAPTER_TESTS)
    subcommands.add_parser("performance")
    subcommands.add_parser("publish")
    arguments = parser.parse_args()
    if arguments.command == "prepare":
        prepare()
    elif arguments.command == "sample":
        sample_suite(arguments.name)
    elif arguments.command == "adapter":
        adapter(arguments.name)
    elif arguments.command == "performance":
        performance()
    else:
        publish()


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
