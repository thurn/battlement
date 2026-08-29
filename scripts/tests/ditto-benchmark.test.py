#!/usr/bin/env python3

"""Black-box checks for the fixed Ditto performance benchmark."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
import sys
import tempfile


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))
SPEC = importlib.util.spec_from_file_location(
    "ditto_benchmark", REPOSITORY_ROOT / "scripts/ditto_benchmark.py"
)
assert SPEC is not None and SPEC.loader is not None
benchmark = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(benchmark)
BUDGET_SPEC = importlib.util.spec_from_file_location(
    "ditto_benchmark_budget", REPOSITORY_ROOT / "scripts/ditto_benchmark_budget.py"
)
assert BUDGET_SPEC is not None and BUDGET_SPEC.loader is not None
budget = importlib.util.module_from_spec(BUDGET_SPEC)
BUDGET_SPEC.loader.exec_module(budget)


def fixture_result(sample: dict, scenarios: list[dict]) -> dict:
    return {
        "run_id": "0197b35f-6e24-75d8-9482-aa6c22a15133",
        "cycle": 0,
        "command": "run",
        "status": "passed",
        "duration_ms": 100,
        "build": {
            "source_fingerprint": "1" * 64,
            "fingerprint": "2" * 64,
            "disposition": "reused",
            "duration_ms": 10,
        },
        "phases": [
            {"name": name, "duration_ms": value}
            for name, value in (
                ("build", 10), ("launch", 20), ("startup", 5), ("scenarios", 50),
                ("reset", 5), ("durability", 5), ("cleanup", 5),
            )
        ],
        "scenarios": [scenario_result(scenario) for scenario in scenarios],
    }


def scenario_result(scenario: dict) -> dict:
    return {
        "name": scenario["name"],
        "status": "passed",
        "timings": {
            "startup_ms": 1,
            "settle_ms": 2,
            "capture_ms": 3,
            "baseline_read_ms": 4,
            "odiff_ms": 5,
            "reset_ms": 6,
            "baseline_download_ms": 0,
            "durability_ms": 7,
        },
        "steps": [
            {
                "screenshot": {
                    "status": "captured",
                    "checkpoint": checkpoint,
                    "actual": {"width": 1280, "height": 720},
                }
            }
            for checkpoint in scenario["checkpoints"]
        ],
    }


def rejects(result: dict, sample: dict, scenarios: list[dict], mutation) -> None:
    changed = copy.deepcopy(result)
    mutation(changed)
    try:
        benchmark.validate_result(changed, sample, scenarios)
    except RuntimeError:
        return
    raise AssertionError("invalid benchmark result was accepted")


def main() -> None:
    definition = benchmark.load_definition(benchmark.DEFAULT_DEFINITION)
    assert [sample["name"] for sample in definition["samples"]] == [
        "basic", "tictactoe", "reactant", "chess", "ui"
    ]
    sample = definition["samples"][0]
    result = fixture_result(sample, sample["scenarios"])
    first = benchmark.validate_result(result, sample, sample["scenarios"])
    second = benchmark.validate_result(copy.deepcopy(result), sample, sample["scenarios"])
    assert first == second
    assert first["scenario_count"] == 4
    assert first["checkpoint_count"] == 6
    assert first["excluded_ms"] == 0

    warm_result = copy.deepcopy(result)
    warm_result["cycle"] = 2
    warm_result["phases"] = [
        phase for phase in warm_result["phases"]
        if phase["name"] not in {"launch", "cleanup"}
    ]
    warm = benchmark.validate_result(warm_result, sample, sample["scenarios"], watch=True)
    assert warm["phases_ms"]["cold_launch_ms"] is None
    assert warm["phases_ms"]["warm_watch_execution_ms"] == 100

    initial_watch = copy.deepcopy(result)
    initial_watch["phases"] = [
        phase for phase in initial_watch["phases"] if phase["name"] != "cleanup"
    ]
    benchmark.validate_result(initial_watch, sample, sample["scenarios"], watch=True)
    rejects(initial_watch, sample, sample["scenarios"], lambda value: None)

    rejects(result, sample, sample["scenarios"], lambda value: value["phases"].pop())
    rejects(
        result, sample, sample["scenarios"],
        lambda value: value["scenarios"][0]["timings"].pop("settle_ms"),
    )
    rejects(
        result, sample, sample["scenarios"],
        lambda value: value["build"].update(disposition="created"),
    )
    rejects(
        result, sample, sample["scenarios"],
        lambda value: value["scenarios"][0]["timings"].update(baseline_download_ms=1),
    )
    rejects(result, sample, sample["scenarios"], lambda value: value["scenarios"].pop())
    rejects(
        result, sample, sample["scenarios"],
        lambda value: value["scenarios"][0]["steps"].pop(),
    )
    rejects(
        result, sample, sample["scenarios"],
        lambda value: value["scenarios"][0]["steps"][0]["screenshot"]["actual"].update(width=1),
    )

    changed = copy.deepcopy(definition)
    changed["samples"][0]["scenarios"].pop()
    with tempfile.TemporaryDirectory(prefix="ditto-benchmark-test.") as temporary:
        path = Path(temporary) / "definition.json"
        path.write_text(__import__("json").dumps(changed), encoding="utf-8")
        try:
            benchmark.load_definition(path)
        except RuntimeError:
            pass
        else:
            raise AssertionError("changed 20/40 benchmark shape was accepted")

    samples = []
    for fixed_sample in definition["samples"]:
        fixed_result = fixture_result(fixed_sample, fixed_sample["scenarios"])
        summary = benchmark.validate_result(
            fixed_result, fixed_sample, fixed_sample["scenarios"]
        )
        samples.append({**summary, "sample": fixed_sample["name"]})
    repetition = budget.summarize_repetition(samples)
    assert repetition["scenario_count"] == 20
    assert repetition["checkpoint_count"] == 40
    assert repetition["execution_ms"] == 90
    accepted = budget.enforce(
        [10] * budget.HASH_REPETITIONS,
        [repetition] * budget.COLD_REPETITIONS,
        [repetition] * budget.WARM_REPETITIONS,
    )
    assert accepted["source_hashing_ms"] == {
        "minimum": 10, "median": 10.0, "p95": 10, "maximum": 10
    }
    assert accepted["cold"]["execution_ms"]["maximum"] == 90
    assert budget.distribution(list(range(1, 21)))["p95"] == 19

    too_slow = copy.deepcopy(repetition)
    too_slow["execution_ms"] = budget.WARM_BUDGET_MS + 1
    try:
        budget.enforce(
            [10] * budget.HASH_REPETITIONS,
            [repetition] * budget.COLD_REPETITIONS,
            [too_slow] * budget.WARM_REPETITIONS,
        )
    except RuntimeError:
        pass
    else:
        raise AssertionError("an over-budget warm distribution was accepted")
    print("Ditto benchmark tests passed.")


if __name__ == "__main__":
    main()
