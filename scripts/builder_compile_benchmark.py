#!/usr/bin/env python3
"""Measure warm incremental sample builds with reversible, equivalent edits."""

from __future__ import annotations

import argparse
import json
import platform
from pathlib import Path
import statistics
import subprocess
import time


ROOT = Path(__file__).resolve().parent.parent
SAMPLES = {
    "reactant": ("composition.rs", "state_identity.rs", "StateIdentity", "compact", "bool"),
    "chess-ui": ("review_text.rs", "caret.rs", "Caret", "is_open", "bool"),
}


def measure(command: list[str]) -> float:
    start = time.perf_counter()
    subprocess.run(command, cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
    return time.perf_counter() - start


def edits(sample: str, scenario: str, iteration: int, generated: bool) -> dict[Path, str]:
    body_file, prop_file, component, field, ty = SAMPLES[sample]
    source = ROOT / "samples" / sample / "rules" / "src"
    if scenario == "body":
        path = source / body_file
        text = path.read_text()
        needle = '"COMPOSITION"' if sample == "reactant" else '"page-heading"'
        if needle not in text:
            # ReviewText owns no heading-name literal: edit its typography value.
            needle = ".font_size(24)"
            replacement = f".font_size({24 + iteration})"
        else:
            replacement = json.dumps(needle[1:-1] + str(iteration))
        assert needle in text, (path, needle)
        return {path: text.replace(needle, replacement, 1)}
    if scenario == "chain":
        path = source / prop_file
        if sample == "reactant" and not generated:
            expression = f"{component} {{ compact: {str(iteration % 2 == 0).lower()} }}"
        elif sample == "reactant":
            expression = f"{component}::new().compact({str(iteration % 2 == 0).lower()})"
        else:
            method = "is_open" if generated else "open"
            expression = f"{component}::new().{method}({str(iteration % 2 == 0).lower()})"
        return {path: path.read_text() + f"\n#[allow(dead_code)]\nfn builder_benchmark_chain_{iteration}() {{ let _ = {expression}; }}\n"}
    if scenario == "prop":
        path = source / prop_file
        text = path.read_text()
        needle = f"{field}: {ty}"
        assert needle in text, (path, needle)
        alias = f"BuilderBenchmarkProperty{iteration}"
        return {path: text.replace(needle, f"{field}: {alias}", 1) + f"\ntype {alias} = {ty};\n"}
    return {}


def run_sample(sample: str, mode: str, runs: int, generated: bool) -> dict:
    command = ["cargo", mode, "--quiet", "--manifest-path", f"samples/{sample}/rules/Cargo.toml"]
    measure(command)
    result = {}
    for scenario in ("noop", "body", "chain", "prop"):
        observations = []
        for iteration in range(runs):
            changes = edits(sample, scenario, iteration, generated)
            originals = {path: path.read_text() for path in changes}
            try:
                for path, text in changes.items():
                    path.write_text(text)
                observations.append(measure(command))
            finally:
                for path, text in originals.items():
                    path.write_text(text)
            if changes:
                measure(command)
        result[scenario] = {
            "seconds": observations,
            "median": statistics.median(observations),
            "min": min(observations),
            "max": max(observations),
        }
        print(f"{sample} {mode} {scenario}: {result[scenario]['median']:.3f}s", flush=True)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--generated", action="store_true")
    parser.add_argument("--runs", type=int, default=10)
    args = parser.parse_args()
    assert args.runs > 0
    result = {
        "platform": platform.platform(),
        "hardware": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip()
        if platform.system() == "Darwin" else platform.processor(),
        "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
        "generated": args.generated,
        "runs": args.runs,
        "samples": {},
    }
    for sample in SAMPLES:
        result["samples"][sample] = {}
        for mode in ("check", "build"):
            result["samples"][sample][mode] = run_sample(sample, mode, args.runs, args.generated)
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
