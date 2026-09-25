"""Bounded parallel validation of isolated repository-tool fixtures."""

from __future__ import annotations

from pathlib import Path
import sys

import ci_steps
import operation_log


CHECKS = (
    ("Test process resource accounting", "scripts/tests/process-usage.test.py"),
    ("Test process scheduling priority", "scripts/tests/process-priority.test.py"),
    ("Test operation telemetry", "scripts/tests/operation-log.test.py"),
    ("Test CI job handles", "scripts/tests/ci-job.test.py"),
    ("Test validation preparation", "scripts/tests/prepare-validation.test.py"),
    ("Test resource slots", "scripts/tests/resource-slots.test.py"),
    ("Test Unity transactions", "scripts/tests/unity-transaction.test.py"),
    ("Test Web sample server", "scripts/tests/serve-web.test.py"),
    ("Test Web demo cache", "scripts/tests/prepare-web-demo.test.py"),
    ("Test sample deployment workflow", "scripts/tests/deploy.test.py"),
    ("Test isolated Playwright transport", "scripts/tests/playwright-mcp.test.py"),
    ("Test browser risk selection", "scripts/tests/web-selection.test.py"),
    ("Test Stylon validation", "scripts/tests/stylon-validation.test.py"),
    ("Test CI sample discovery", "scripts/tests/ci.test.py"),
    ("Test affected CI selection", "scripts/tests/ci-selection.test.py"),
    ("Test CI Cache", "scripts/tests/ci-cache.test.py"),
    ("Test Unity affected-test selection", "scripts/tests/unity-test-selection.test.py"),
    ("Test native sample selection", "scripts/tests/native-validation-selection.test.py"),
    ("Test performance reporting", "scripts/tests/perf-report.test.py"),
    ("Test candidate performance reporting", "scripts/tests/perf-candidate.test.py"),
    ("Test Tollgate evidence collection", "scripts/tests/tollgate-evidence.test.py"),
    ("Test trusted prose validation", "scripts/tests/prose-validation.test.py"),
    ("Test Ditto CI", "scripts/tests/ditto-ci.test.py"),
    ("Test Ditto replay", "scripts/tests/ditto-replay.test.py"),
    ("Test Ditto build-cache lifetime", "scripts/tests/ditto-cache-lifetime.test.py"),
)

PERFORMANCE_CHECKS = (
    ("Test Ditto performance benchmark", "scripts/tests/ditto-benchmark.test.py"),
    ("Test Ditto cutover", "scripts/tests/ditto-cutover.test.py"),
)


def run(repository: Path, *, performance: bool = False) -> None:
    """Run each fixture in its own process, retaining every failure and trace."""
    checks = CHECKS + (PERFORMANCE_CHECKS if performance else ())
    ci_steps.run_parallel_steps(
        [
            (
                name,
                lambda script=script: operation_log.run(
                    [sys.executable, script], cwd=repository,
                ),
            )
            for name, script in checks
        ],
        workers=3,
    )
