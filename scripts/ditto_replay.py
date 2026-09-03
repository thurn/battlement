"""Record and replay native CI invocations against their retained immutable players."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tomllib
import uuid


ENVIRONMENT_KEYS = (
    "DITTO_CACHE_ROOT", "DITTO_ODIFF_PATH", "DITTO_FFMPEG_PATH",
    "BATTLEMENT_FFMPEG", "BATTLEMENT_RESOURCE_SLOTS", "UNITY_EDITOR",
    "DITTO_CONTAINED_SESSION",
)


def digest(path: Path) -> str:
    """Hash file contents without loading a player or runner into memory."""
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def pin_tool(path: Path, cache: Path) -> dict[str, str]:
    """Retain an executable by content so rebuilding a runner cannot change a replay."""
    sha256 = digest(path)
    retained = cache / "replay-tools" / sha256 / path.name
    retained.parent.mkdir(parents=True, exist_ok=True)
    if not retained.exists():
        temporary = retained.with_name(f".{uuid.uuid4()}.tmp")
        shutil.copy2(path, temporary)
        if digest(temporary) != sha256:
            temporary.unlink()
            raise RuntimeError(f"Tool changed while recording replay: {path}")
        temporary.replace(retained)
    return {"path": str(retained), "sha256": sha256}


def record(
    repository: Path, binary: Path, cache: Path, sample: str,
    scenarios: list[str], environment: dict[str, str],
) -> dict:
    """Capture the effective CI configuration without recording credentials."""
    config = Path(f"samples/{sample}/ditto.toml")
    lock = config.with_name("ditto.lock")
    suite = tomllib.loads((repository / config).read_text())
    comparison_scenarios = [
        scenario["name"] for scenario in suite["scenarios"]
        if any("screenshot" in step for step in scenario["steps"])
    ]
    tools = {"runner": pin_tool(binary, cache)}
    for key in ("DITTO_ODIFF_PATH", "DITTO_FFMPEG_PATH"):
        value = environment.get(key)
        if value and Path(value).is_file():
            tools[key] = pin_tool(Path(value), cache)
    return {
        "sample": sample, "profile": "macos", "scenarios": scenarios,
        "comparison_scenarios": comparison_scenarios,
        "config": str(config),
        "files": {str(path): digest(repository / path) for path in (config, lock)
                  if (repository / path).is_file()},
        "absent_files": [str(path) for path in (config, lock)
                         if not (repository / path).exists()],
        "tools": tools,
        "environment": {key: environment.get(key) for key in ENVIRONMENT_KEYS},
    }


def save(recipe: dict, output: Path, result: dict | None = None) -> None:
    """Link replay inputs to the original terminal result, preserving failed outcomes."""
    if result is not None:
        recipe["source_run_id"] = result["run_id"]
        recipe["source_status"] = result["status"]
        recipe["build"] = result.get("build")
    output.write_text(json.dumps(recipe, indent=2, sort_keys=True) + "\n")


def prepare(recipe_path: Path, repository: Path, scenarios: list[str]) -> tuple[dict, list[str], dict[str, str]]:
    """Validate all replay inputs before allowing any player process to launch."""
    recipe = json.loads(recipe_path.read_text())
    build = recipe.get("build") or {}
    if not build.get("fingerprint"):
        raise RuntimeError("This run has no retained player to replay; resolve its preflight failure first.")
    for relative, expected in recipe["files"].items():
        path = repository / relative
        if not path.is_file() or digest(path) != expected:
            raise RuntimeError(f"Replay configuration changed: {relative}; use the recorded source checkout.")
    for relative in recipe["absent_files"]:
        if (repository / relative).exists():
            raise RuntimeError(f"Replay configuration changed: {relative} now exists.")
    selection = scenarios or recipe["scenarios"]
    unknown = set(selection) - set(recipe["scenarios"])
    if unknown:
        raise RuntimeError(f"Scenarios were not in the original run: {sorted(unknown)}")
    environment = os.environ.copy()
    for key in ENVIRONMENT_KEYS:
        environment.pop(key, None)
        if recipe["environment"].get(key) is not None:
            environment[key] = recipe["environment"][key]
    for key, tool in recipe["tools"].items():
        path = Path(tool["path"])
        if not path.is_file() or digest(path) != tool["sha256"]:
            raise RuntimeError(f"Replay dependency is missing or changed: {key} at {path}")
        if not os.access(path, os.X_OK):
            raise RuntimeError(f"Replay dependency is not executable: {path}")
        if key != "runner":
            environment[key] = str(path)
    requires_comparison = bool(set(selection) & set(recipe["comparison_scenarios"]))
    if requires_comparison and "DITTO_ODIFF_PATH" not in recipe["tools"]:
        raise RuntimeError("The original run did not record an available ODiff dependency.")
    environment["DITTO_REPLAY_BUILD_FINGERPRINT"] = build["fingerprint"]
    arguments = [recipe["tools"]["runner"]["path"], "--config", recipe["config"],
                 "run", "--profile", recipe["profile"], "--no-build", "--json"]
    return recipe, [*arguments, *selection], environment


def replay(recipe_path: Path, repository: Path, scenarios: list[str], output: Path) -> int:
    """Run one checked replay and retain separate results without overwriting its source."""
    recipe, arguments, environment = prepare(recipe_path, repository, scenarios)
    output.mkdir(parents=True, exist_ok=False)
    save(recipe, output / "source-replay.json")
    completed = subprocess.run(
        [*arguments, "--output", str(output / "result.json")],
        cwd=repository, env=environment, capture_output=True, text=True,
    )
    (output / "stdout.log").write_text(completed.stdout)
    (output / "stderr.log").write_text(completed.stderr)
    print(completed.stderr, end="")
    print(f"Original {recipe['source_run_id']}: {recipe['source_status']}; replay evidence: {output}")
    return completed.returncode
