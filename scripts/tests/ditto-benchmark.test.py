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
    print("Ditto benchmark tests passed.")


if __name__ == "__main__":
    main()
