#!/usr/bin/env python3
"""Verify the production typed persistence store against real browser IndexedDB."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import uuid

import ci
from platform_support import user_cache_path
from resource_slots import compiler_capacity_lease
from web_compatibility import check_site


ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures/browser-persistence"
TARGET = "wasm32-unknown-emscripten"


def build(destination: Path) -> None:
    """Link the real Rust backend and shipped JavaScript bridge without a renderer."""
    editor = ci.unity_editor()
    if os.name == "nt":
        tools = editor.parent / "Data/PlaybackEngines/WebGLSupport/BuildTools/Emscripten"
    else:
        tools = editor.parents[3] / "PlaybackEngines/WebGLSupport/BuildTools/Emscripten"
    target = Path(ci.cargo_environment(None)["CARGO_TARGET_DIR"]) / "browser-persistence"
    target.mkdir(parents=True, exist_ok=True)
    config = target / ".emscripten"
    config.write_text("\n".join([
        f"LLVM_ROOT = {str(tools / 'llvm')!r}",
        f"BINARYEN_ROOT = {str(tools / 'binaryen')!r}",
        f"NODE_JS = {str(tools / ('node/node.exe' if os.name == 'nt' else 'node/node'))!r}",
    ]) + "\n")
    exports = ["_main", "_fixture_reset", "_fixture_update", "_fixture_clear",
               "_fixture_retry", "_fixture_snapshot"]
    bridge = ROOT / "Packages/com.battlement.client/Runtime/BattlementPersistence.jslib"
    bootstrap = FIXTURE / "bootstrap.js"
    linker = [
        "-fwasm-exceptions", "-pthread", "-lidbfs.js",
        "-sALLOW_MEMORY_GROWTH=1", "-sEXIT_RUNTIME=0", "-sENVIRONMENT=web,worker",
        "-sEXPORTED_FUNCTIONS=" + json.dumps(exports),
        "--js-library", str(bridge), "--pre-js", str(bootstrap),
    ]
    flags = ["-C", "panic=unwind", "-C", "target-feature=+atomics,+bulk-memory,+mutable-globals"]
    # Cargo does not discover external JavaScript linker inputs in Rust dep-info.
    link_identity = hashlib.sha256(bridge.read_bytes() + b"\0" + bootstrap.read_bytes()).hexdigest()
    link_flags = ["-C", "metadata=" + link_identity]
    for argument in linker:
        link_flags.extend(["-C", "link-arg=" + argument])
    environment = os.environ | {
        "RUSTC_BOOTSTRAP": "1", "CARGO_TARGET_DIR": str(target),
        "CARGO_ENCODED_RUSTFLAGS": "\x1f".join(flags),
        "CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER": str(tools / ("emscripten/emcc.bat" if os.name == "nt" else "emscripten/emcc")),
        "EM_CONFIG": str(config), "EM_CACHE": str(target / "emscripten-cache"),
        "PATH": os.pathsep.join([str(tools / part) for part in
                                 ("emscripten", "llvm", "binaryen/bin", "node")] + [os.environ["PATH"]]),
    }
    with compiler_capacity_lease():
        subprocess.run([
            "cargo", "rustc", "--locked", "-j", "3", "-p", "reactant",
            "--example", "browser_persistence", "--target", TARGET, "--release",
            "-Z", "build-std=std,panic_unwind", "--", *link_flags,
        ], cwd=ROOT, env=environment, check=True)
    destination.mkdir(parents=True, exist_ok=True)
    for path in (target / TARGET / "release/examples").glob("browser_persistence.*"):
        if path.suffix in {".js", ".wasm"}:
            shutil.copy2(path, destination / path.name)
    (destination / "index.html").write_text(
        '<!doctype html><meta charset="utf-8"><title>Persistence contract</title>'
        '<script src="browser_persistence.js"></script>'
    )
    (destination / "build.json").write_text(json.dumps({
        "target": TARGET, "unity_editor": str(editor), "link_arguments": linker,
        "link_identity": link_identity,
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "cargo_lock_sha256": hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
        "bridge_sha256": hashlib.sha256(bridge.read_bytes()).hexdigest(),
    }, indent=2) + "\n")


def run(output: Path | None = None) -> Path:
    """Retain exact linked bytes, source identity, and browser transaction evidence."""
    artifact = output or user_cache_path("Battlement", "web-compatibility") / str(uuid.uuid4())
    site = artifact / "site"
    site.mkdir(parents=True, exist_ok=False)
    build(site / "persistence")
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    evidence = check_site(ROOT, site, ["persistence"], revision, artifact_root=artifact / "check")
    print(f"Browser persistence evidence: {evidence}", flush=True)
    return evidence


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--build-only", type=Path)
    arguments = parser.parse_args()
    if arguments.build_only:
        build(arguments.build_only)
    else:
        run(arguments.output)
