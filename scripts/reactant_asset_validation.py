#!/usr/bin/env python3

"""Run focused, exhaustive, or performance validation for Reactant assets."""

from __future__ import annotations

import argparse
from contextlib import nullcontext
import json
import math
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from typing import Callable

from resource_slots import unity_editor_lease


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
UNITY_VERSION = "6000.5.8f1"
FAST_TEST = (
    "real_browser_batch_emits_deterministic_rgba_png_metadata_for_every_paint_family"
)
CLI_TEST_TARGETS = (
    "reactant_assets_browser_tests",
    "reactant_assets_cli_tests",
    "reactant_assets_commands_preview_tests",
    "reactant_assets_manifest_tests",
    "reactant_assets_render_tests",
    "reactant_assets_transaction_tests",
)
ZERO_WORK_FIELDS = (
    "browserContextsCreated",
    "browserExecutableOpens",
    "browserLaunches",
    "cargoMetadataRuns",
    "dependencyFileOpens",
    "filesWritten",
    "generatedPngOpens",
    "rustSourceOpens",
    "subprocessesStarted",
)
COMMAND_TRANSCRIPT: list[dict[str, object]] | None = None


def run(
    command: list[str],
    *,
    cwd: Path = REPOSITORY_ROOT,
    environment: dict[str, str] | None = None,
    capture: bool = False,
) -> subprocess.CompletedProcess[str]:
    """Run one public validation command and echo it into the transcript."""
    print(f"$ {' '.join(command)}", flush=True)
    started = time.monotonic()
    status = "failed"
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=environment,
            check=True,
            text=True,
            capture_output=capture,
        )
        status = "passed"
        return completed
    finally:
        if COMMAND_TRANSCRIPT is not None:
            COMMAND_TRANSCRIPT.append(
                {
                    "command": command,
                    "cwd": str(cwd),
                    "elapsedSeconds": time.monotonic() - started,
                    "status": status,
                }
            )


def timed(name: str, function: Callable[[], None]) -> float:
    """Run one portion and print its elapsed wall-clock time."""
    started = time.monotonic()
    try:
        function()
    finally:
        elapsed = time.monotonic() - started
        print(f"reactant-assets fast {name}: {elapsed:.3f}s", flush=True)
    return elapsed


def cargo_environment(scope: str) -> dict[str, str]:
    environment = os.environ.copy()
    environment.setdefault("CARGO_BUILD_JOBS", "3")
    if "CARGO_TARGET_DIR" not in environment:
        environment["CARGO_TARGET_DIR"] = str(
            REPOSITORY_ROOT / "target/reactant-asset-validation" / scope
        )
    return environment


def rust_in_process() -> None:
    run(
        [
            "cargo",
            "test",
            "-p",
            "battlement-reactant-asset-syntax",
            "-p",
            "battlement-reactant-assets",
            "--lib",
        ],
        environment=cargo_environment("fast-rust"),
    )


def compile_fixture() -> None:
    run(
        [
            "cargo",
            "test",
            "-p",
            "battlement-reactant",
            "--test",
            "generated_assets",
        ],
        environment=cargo_environment("fast-compile"),
    )


def cli_browser() -> None:
    run(
        [
            "cargo",
            "test",
            "-p",
            "battlement-cli",
            "--test",
            "reactant_assets_render_tests",
            FAST_TEST,
            "--",
            "--exact",
            "--ignored",
            "--nocapture",
        ],
        environment=cargo_environment("fast-cli-browser"),
    )


def unity_editor() -> Path:
    configured = os.environ.get("UNITY_EDITOR")
    if configured is not None:
        return Path(configured)
    if platform.system() == "Darwin":
        return Path(
            f"/Applications/Unity/Hub/Editor/{UNITY_VERSION}/Unity.app/Contents/MacOS/Unity"
        )
    if platform.system() == "Linux":
        return Path.home() / f"Unity/Hub/Editor/{UNITY_VERSION}/Editor/Unity"
    if platform.system() == "Windows":
        program_files = Path(os.environ.get("PROGRAMFILES", "C:/Program Files"))
        return program_files / f"Unity/Hub/Editor/{UNITY_VERSION}/Editor/Unity.exe"
    raise RuntimeError("Unity is unsupported on this operating system")


def unity(category: str, evidence: Path | None = None) -> None:
    editor = unity_editor()
    if not os.access(editor, os.X_OK):
        raise RuntimeError(f"Unity executable was not found at {editor}")
    output = evidence or Path(tempfile.mkdtemp(prefix="reactant-assets-unity."))
    output.mkdir(parents=True, exist_ok=True)
    results = output / f"{category}.xml.txt"
    log = Path(os.devnull)
    lease = unity_editor_lease() if platform.system() != "Windows" else nullcontext()
    with lease:
        run(
            [
                str(editor),
                "-batchmode",
                "-nographics",
                "--burst-disable-compilation",
                "-projectPath",
                str(REPOSITORY_ROOT),
                "-runTests",
                "-testPlatform",
                "EditMode",
                "-testCategory",
                category,
                "-testResults",
                str(results),
                "-logFile",
                str(log),
            ]
        )
    contents = results.read_text(encoding="utf-8", errors="replace")
    if 'result="Passed"' not in contents:
        raise RuntimeError(f"Unity category {category} did not report a passing run")
    if not contents.endswith("\n"):
        results.write_text(contents + "\n", encoding="utf-8")
    print(f"Unity results: {results}", flush=True)


def fast(portion: str) -> None:
    portions: tuple[tuple[str, Callable[[], None]], ...] = (
        ("in-process", rust_in_process),
        ("compile", compile_fixture),
        ("cli/browser", cli_browser),
        ("Unity", lambda: unity("ReactantGeneratedAssetsFast")),
    )
    selected = portions if portion == "all" else tuple(
        item for item in portions if item[0] == portion
    )
    started = time.monotonic()
    timings = {name: timed(name, function) for name, function in selected}
    for name, _function in portions:
        timings.setdefault(name, 0.0)
    total = time.monotonic() - started
    print(
        "reactant-assets fast timing "
        + " ".join(f"{name}={timings[name]:.3f}s" for name, _ in portions)
        + f" total={total:.3f}s",
        flush=True,
    )


def exhaustive(evidence: Path) -> None:
    global COMMAND_TRANSCRIPT

    evidence.mkdir(parents=True, exist_ok=True)
    COMMAND_TRANSCRIPT = []
    started = time.monotonic()
    status = "failed"
    try:
        run(
            ["cargo", "test", "--workspace"],
            environment=cargo_environment("exhaustive-rust"),
        )
        for target in CLI_TEST_TARGETS:
            run(
                [
                    "cargo",
                    "test",
                    "-p",
                    "battlement-cli",
                    "--test",
                    target,
                    "--",
                    "--ignored",
                    "--nocapture",
                ],
                environment=cargo_environment("exhaustive-cli"),
            )
        unity("ReactantGeneratedAssetsExhaustive", evidence)
        run(
            [
                "cargo",
                "run",
                "--quiet",
                "-p",
                "battlement-cli",
                "--",
                "sample",
                "build",
                "reactant",
            ],
            environment=cargo_environment("exhaustive-native"),
        )
        run(
            [
                "cargo",
                "run",
                "--quiet",
                "-p",
                "battlement-cli",
                "--",
                "sample",
                "build",
                "reactant",
                "--web",
            ],
            environment=cargo_environment("exhaustive-web"),
        )
        status = "passed"
    finally:
        total = time.monotonic() - started
        (evidence / "exhaustive.json").write_text(
            json.dumps(
                {
                    "commands": COMMAND_TRANSCRIPT,
                    "elapsedSeconds": total,
                    "status": status,
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )
        COMMAND_TRANSCRIPT = None
        print(f"reactant-assets exhaustive total: {total:.3f}s")


def binary_path(environment: dict[str, str]) -> Path:
    suffix = ".exe" if os.name == "nt" else ""
    return Path(environment["CARGO_TARGET_DIR"]) / "debug" / f"cargo-battlement{suffix}"


def write_performance_fixture(root: Path) -> Path:
    project = root / "game"
    source = project / "rules/src"
    for directory in (project / "Assets", project / "Packages", project / "ProjectSettings", source):
        directory.mkdir(parents=True, exist_ok=True)
    (project / "Packages/manifest.json").write_text("{}\n", encoding="utf-8")
    (project / "ProjectSettings/ProjectVersion.txt").write_text(
        "m_EditorVersion: performance-fixture\n", encoding="utf-8"
    )
    reactant = REPOSITORY_ROOT / "crates/battlement-reactant"
    (project / "rules/Cargo.toml").write_text(
        "[package]\n"
        'name = "reactant-asset-performance"\n'
        'version = "0.1.0"\n'
        'edition = "2024"\n'
        "[dependencies]\n"
        f"battlement-reactant = {{ path = {json.dumps(str(reactant))} }}\n",
        encoding="utf-8",
    )
    modules = []
    for index in range(1_000):
        modules.append(f"mod source_{index:04};")
        declaration = ""
        if index < 100:
            variant = index % 8
            width = 20 + variant
            declaration = (
                "battlement_reactant::asset_generator::generate! {\n"
                f"  @background PERF_{index:03} {{\n"
                f"    @canvas {width}px 16px;\n"
                f"    @subject 2px 2px {width - 4}px 12px;\n"
                f"    background: linear-gradient({1.0 + variant * 3.0:.1f}deg, red, blue);\n"
                "  }\n"
                "}\n"
            )
        (source / f"source_{index:04}.rs").write_text(declaration, encoding="utf-8")
    (source / "lib.rs").write_text("\n".join(modules) + "\n", encoding="utf-8")
    return project


def percentile_95(samples: list[float]) -> float:
    return sorted(samples)[math.ceil(len(samples) * 0.95) - 1]


def validate_work_report(report: dict[str, int]) -> None:
    for field in ZERO_WORK_FIELDS:
        if report[field] != 0:
            raise RuntimeError(f"warm report recorded {field}={report[field]}")
    if report["statCalls"] > 1_250:
        raise RuntimeError(f"warm report exceeded 1,250 stat calls: {report['statCalls']}")
    if report["filesOpened"] > 8:
        raise RuntimeError(f"warm report exceeded eight file opens: {report['filesOpened']}")
    if report["bytesRead"] > 1024 * 1024:
        raise RuntimeError(f"warm report exceeded one MiB read: {report['bytesRead']}")


def performance(evidence: Path) -> None:
    if platform.system() != "Darwin":
        raise RuntimeError("the Reactant asset performance tier requires the macOS reference host")
    evidence.mkdir(parents=True, exist_ok=True)
    environment = cargo_environment("performance")
    run(["cargo", "build", "--quiet", "-p", "battlement-cli"], environment=environment)
    executable = binary_path(environment)
    with tempfile.TemporaryDirectory(prefix="reactant-asset-performance.") as temporary:
        project = write_performance_fixture(Path(temporary))
        report = Path(temporary) / "warmup.json"
        command = [
            str(executable),
            "reactant",
            "assets",
            "generate",
            "--work-report",
            str(report),
        ]
        run(command, cwd=project, environment=environment, capture=True)
        run(command, cwd=project, environment=environment, capture=True)
        samples = []
        reports = []
        for index in range(20):
            report = evidence / f"warm-{index + 1:02}.json"
            invocation = [*command[:-1], str(report)]
            started = time.perf_counter()
            run(invocation, cwd=project, environment=environment, capture=True)
            samples.append((time.perf_counter() - started) * 1_000)
            work = json.loads(report.read_text(encoding="utf-8"))
            validate_work_report(work)
            reports.append(work)
    result = {
        "fixture": {"rustFiles": 1_000, "declarations": 100, "invocations": 20},
        "milliseconds": samples,
        "medianMilliseconds": statistics.median(samples),
        "p95Milliseconds": percentile_95(samples),
        "maximumWork": {
            key: max(report[key] for report in reports) for key in reports[0]
        },
    }
    (evidence / "performance.json").write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    if result["medianMilliseconds"] >= 200:
        raise RuntimeError(f"median exceeded 200 ms: {result['medianMilliseconds']:.3f}")
    if result["p95Milliseconds"] >= 300:
        raise RuntimeError(f"p95 exceeded 300 ms: {result['p95Milliseconds']:.3f}")
    print(json.dumps(result, indent=2, sort_keys=True), flush=True)


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="tier", required=True)
    fast_parser = subcommands.add_parser("fast")
    fast_parser.add_argument(
        "--portion",
        choices=("all", "in-process", "compile", "cli/browser", "Unity"),
        default="all",
    )
    for tier in ("exhaustive", "performance"):
        command = subcommands.add_parser(tier)
        command.add_argument(
            "--evidence",
            type=Path,
            default=REPOSITORY_ROOT / "artifacts/reactant-asset-validation" / tier,
        )
    return parser.parse_args()


def main() -> None:
    arguments = parse_arguments()
    if arguments.tier == "fast":
        fast(arguments.portion)
    elif arguments.tier == "exhaustive":
        exhaustive(arguments.evidence.resolve())
    else:
        performance(arguments.evidence.resolve())


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
