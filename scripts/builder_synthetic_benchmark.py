#!/usr/bin/env python3
"""Compare generated, handwritten any-order, and fixed-order property builders."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import time


ROOT = Path(__file__).resolve().parent.parent
CASES = [(0, 8, False), (2, 8, True), (8, 8, True), (16, 8, True), (8, 32, True), (16, 8, False)]


def type_name(name: str, states: list[str]) -> str:
    return name + ("<" + ",".join(states) + ">" if states else "")


def assignments(required: int, optional: int, changed: int | None = None) -> str:
    return ",".join(
        [f"r{i}:" + ("value" if i == changed else f"self.r{i}") for i in range(required)]
        + [f"o{i}:self.o{i}" for i in range(optional)]
    )


def optional_methods(optional: int) -> str:
    return "\n".join(
        f"fn o{i}(mut self,value:u32)->Self {{self.o{i}=value;self}}"
        for i in range(optional)
    )


def generated(name: str, required: int, optional: int) -> str:
    fields = ",".join(
        [f"#[builder(required)] r{i}:u32" for i in range(required)]
        + [f"o{i}:u32" for i in range(optional)]
    )
    return f"#[builder(support = support)] struct {name} {{{fields}}}"


def handwritten(name: str, required: int, optional: int) -> str:
    params = [f"S{i}" for i in range(required)]
    declaration = type_name(name, [f"{param}=u32" for param in params])
    missing = ["Missing<u32>"] * required
    fields = ",".join([f"r{i}:S{i}" for i in range(required)] + [f"o{i}:u32" for i in range(optional)])
    defaults = ",".join([f"r{i}:Missing::new()" for i in range(required)] + [f"o{i}:0" for i in range(optional)])
    result = [f"struct {declaration} {{{fields}}}", f"impl {name} {{fn new()->{type_name(name, missing)} {{{name} {{{defaults}}}}}}}"]
    result.append(f"impl{type_name('',params)} {type_name(name,params)} {{{optional_methods(optional)}}}")
    for index in range(required):
        receiver, returned = params.copy(), params.copy()
        receiver[index], returned[index] = "Missing<u32>", "u32"
        others = [param for i, param in enumerate(params) if i != index]
        result.append(
            f"impl{type_name('',others)} {type_name(name,receiver)} {{fn r{index}(self,value:u32)->{type_name(name,returned)} "
            f"{{{name} {{{assignments(required,optional,index)}}}}}}}"
        )
    return "\n".join(result)


def fixed(name: str, required: int, optional: int) -> str:
    stage = lambda index: name if index == required else f"{name}Stage{index}"
    result = []
    for index in range(required + 1):
        fields = ",".join(
            [f"r{i}:" + ("u32" if i < index else "Missing<u32>") for i in range(required)]
            + [f"o{i}:u32" for i in range(optional)]
        )
        result.append(f"struct {stage(index)} {{{fields}}}")
        result.append(f"impl {stage(index)} {{{optional_methods(optional)}}}")
        if index < required:
            result.append(f"impl {stage(index)} {{fn r{index}(self,value:u32)->{stage(index+1)} {{{stage(index+1)} {{{assignments(required,optional,index)}}}}}}}")
    defaults = ",".join([f"r{i}:Missing::new()" for i in range(required)] + [f"o{i}:0" for i in range(optional)])
    result.append(f"impl {name} {{fn new()->{stage(0)} {{{stage(0)} {{{defaults}}}}}}}")
    return "\n".join(result)


def source(variant: str, required: int, optional: int, varied: bool, components: int, seed: int) -> str:
    declarations = []
    uses = []
    generate = {"generated": generated, "handwritten": handwritten, "fixed": fixed}[variant]
    for component in range(components):
        name = f"Component{component}"
        declarations.append(generate(name, required, optional))
        for order in range(2):
            indices = list(range(required))
            if varied and order and variant != "fixed":
                indices.reverse()
            chain = f"{name}::new()"
            for index in indices:
                chain += f".r{index}({index})"
                if optional:
                    chain += f".o0({index})"
            for index in range(optional):
                chain += f".o{index}({index})"
            uses.append(f"std::hint::black_box({chain});")
    return "\n".join([
        "#![allow(dead_code,unused_imports)]",
        "use battlement_builder::{builder,support}; use support::Missing;",
        *declarations,
        f"pub fn exercise() {{ std::hint::black_box({seed}_u32); {' '.join(uses)} }}",
    ])


def timed(command: list[str], environment: dict[str, str]) -> float:
    started = time.perf_counter()
    subprocess.run(command, cwd=ROOT, env=environment, check=True, stdout=subprocess.DEVNULL)
    return time.perf_counter() - started


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=10)
    parser.add_argument("--components", type=int, default=16)
    args = parser.parse_args()
    assert args.runs > 0 and args.components > 0
    cache = ROOT / "target" / "builder-synthetic"
    cache.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ, CARGO_TARGET_DIR=str(cache / "target"))
    results = {"runs": args.runs, "components": args.components, "cases": []}
    for required, optional, varied in CASES:
        case = {"required": required, "optional": optional, "varied_order": varied, "variants": {}}
        fixtures = {}
        for variant in ("generated", "handwritten", "fixed"):
            directory = cache / f"{variant}-{required}-{optional}-{varied}"
            (directory / "src").mkdir(parents=True, exist_ok=True)
            manifest = directory / "Cargo.toml"
            manifest.write_text(f'[package]\nname="builder-benchmark-{variant}-{required}-{optional}-{str(varied).lower()}"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies]\nbattlement-builder={{path={json.dumps(str(ROOT / "crates/battlement-builder"))}}}\n')
            path = directory / "src/lib.rs"
            path.write_text(source(variant, required, optional, varied, args.components, 0))
            fixtures[variant] = (manifest, path)
            case["variants"][variant] = {}
        for mode in ("check", "build"):
            for manifest, _ in fixtures.values():
                timed(["cargo", mode, "--quiet", "--manifest-path", str(manifest)], environment)
            observations = {variant: [] for variant in fixtures}
            for iteration in range(args.runs):
                # Rotate the order to reduce systematic warm-cache bias.
                order = list(fixtures)
                order = order[iteration % 3:] + order[:iteration % 3]
                for variant in order:
                    manifest, path = fixtures[variant]
                    path.write_text(source(variant, required, optional, varied, args.components, iteration + 1))
                    observations[variant].append(timed(["cargo", mode, "--quiet", "--manifest-path", str(manifest)], environment))
            for variant, values in observations.items():
                case["variants"][variant][mode] = {"seconds": values, "median": statistics.median(values), "min": min(values), "max": max(values)}
        for variant, (manifest, _) in fixtures.items():
            expanded = subprocess.check_output(
                ["cargo", "rustc", "--quiet", "--manifest-path", str(manifest), "--lib", "--", "-Zunpretty=expanded"],
                cwd=ROOT, env=dict(environment, RUSTC_BOOTSTRAP="1"), text=True,
            )
            case["variants"][variant]["expanded_lines"] = len(expanded.splitlines())
            case["variants"][variant]["expanded_bytes"] = len(expanded.encode())
        results["cases"].append(case)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(results, indent=2) + "\n")
        print(f"required={required} optional={optional} varied={varied}: " + ", ".join(f"{v} check={case['variants'][v]['check']['median']:.3f}s" for v in fixtures), flush=True)


if __name__ == "__main__":
    main()
