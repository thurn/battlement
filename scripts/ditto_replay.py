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
TRANSCRIPT_VOLATILE_FIELDS = {
    "action_id", "artifact_id", "batch_id", "battlement_error_id", "engine_session_id",
    "error_ref", "job_id", "player_session_id", "primary_error_ref", "request_id",
    "run_id", "scenario_id", "screenshot_artifact_id", "sequence", "session_id",
    "timestamp_unix_us", "video_input_id",
}
TRANSCRIPT_DIAGNOSTIC_EVENTS = {"battlement.frame.slow"}
TRANSCRIPT_DIAGNOSTIC_SOURCES = {"battlement"}


def semantic_observation(result: dict) -> dict:
    """Return outcome-bearing fields while excluding run IDs, paths, and durations."""
    scenarios = []
    for scenario in result.get("scenarios") or []:
        steps = []
        for step in scenario.get("steps") or []:
            screenshot = step.get("screenshot") or {}
            actual = screenshot.get("actual") or {}
            steps.append({
                "index": step.get("index"),
                "kind": step.get("kind"),
                "status": step.get("status"),
                "status_reason": step.get("status_reason"),
                "assertion": step.get("assertion"),
                "screenshot_status": screenshot.get("status"),
                "screenshot_sha256": actual.get("sha256"),
                "comparison_status": (screenshot.get("comparison") or {}).get("status"),
            })
        scenarios.append({
            "name": scenario.get("name"),
            "status": scenario.get("status"),
            "steps": steps,
        })
    return {
        "status": result.get("status"),
        "exit_code": result.get("exit_code"),
        "scenarios": scenarios,
        "error_codes": [error.get("code") for error in result.get("errors") or []],
    }


def semantic_hash(result: dict) -> str:
    encoded = json.dumps(
        semantic_observation(result), sort_keys=True, separators=(",", ":")
    ).encode()
    return hashlib.sha256(encoded).hexdigest()


def event_transcript_hash(path: Path) -> str:
    """Hash ordered event meaning while excluding generated identity and timing fields."""
    def normalized(value):
        if isinstance(value, dict):
            return {
                key: normalized(item)
                for key, item in sorted(value.items())
                if key not in TRANSCRIPT_VOLATILE_FIELDS
                and key != "duration_ms"
                and not key.endswith("_duration_ms")
            }
        if isinstance(value, list):
            return [normalized(item) for item in value]
        return value

    events = [
        normalized(event)
        for line in path.read_text().splitlines()
        if line
        and (event := json.loads(line)).get("event_name") not in TRANSCRIPT_DIAGNOSTIC_EVENTS
        and event.get("source") not in TRANSCRIPT_DIAGNOSTIC_SOURCES
    ]
    encoded = json.dumps(events, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def paired_observation(path: Path) -> dict:
    """Load one semantic and transcript observation from retained run evidence."""
    document = json.loads(path.read_text())
    if "replay_semantic_hash" in document:
        semantic = document.get("replay_semantic_hash")
        transcript = document.get("replay_event_transcript_hash")
    elif "source_semantic_hash" in document:
        semantic = document.get("source_semantic_hash")
        transcript = document.get("source_event_transcript_hash")
    else:
        semantic = semantic_hash(document)
        event_log = path.parent / "logs/events.jsonl"
        transcript = event_transcript_hash(event_log) if event_log.is_file() else None
    if semantic is None or transcript is None:
        raise ValueError(f"paired evidence lacks semantic or transcript hash: {path}")
    return {"semantic_hash": semantic, "event_transcript_hash": transcript}


def classify_paired_observations(base: list[dict], candidate: list[dict]) -> str:
    """Attribute a candidate only after both sides repeat one exact observation."""
    if len(base) < 2 or len(candidate) < 2:
        return "stability-unestablished"
    base_hashes = {
        (item["semantic_hash"], item["event_transcript_hash"]) for item in base
    }
    candidate_hashes = {
        (item["semantic_hash"], item["event_transcript_hash"]) for item in candidate
    }
    if len(base_hashes) != 1 or len(candidate_hashes) != 1:
        return "nondeterministic-infrastructure"
    if base_hashes == candidate_hashes:
        return "no-candidate-change"
    return "candidate-introduced"


def classify_and_retain(base_paths: list[Path], candidate_paths: list[Path], output: Path) -> str:
    """Classify and retain the exact paired evidence used for CI attribution."""
    base = [paired_observation(path) for path in base_paths]
    candidate = [paired_observation(path) for path in candidate_paths]
    classification = classify_paired_observations(base, candidate)
    payload = {
        "schema": 1,
        "classification": classification,
        "base": [
            {"path": str(path.resolve()), "observation": observation}
            for path, observation in zip(base_paths, base, strict=True)
        ],
        "candidate": [
            {"path": str(path.resolve()), "observation": observation}
            for path, observation in zip(candidate_paths, candidate, strict=True)
        ],
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    return classification


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


def save(
    recipe: dict, output: Path, result: dict | None = None, event_log: Path | None = None,
) -> None:
    """Link replay inputs to the original terminal result, preserving failed outcomes."""
    if result is not None:
        recipe["source_run_id"] = result["run_id"]
        recipe["source_status"] = result["status"]
        recipe["build"] = result.get("build")
        recipe["source_semantic_hash"] = semantic_hash(result)
        if event_log is not None:
            recipe["source_event_transcript_hash"] = event_transcript_hash(event_log)
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
    replay_result = None
    result_path = output / "result.json"
    if result_path.is_file():
        try:
            replay_result = json.loads(result_path.read_text())
        except json.JSONDecodeError:
            pass
    replay_hash = semantic_hash(replay_result) if replay_result is not None else None
    source_hash = recipe.get("source_semantic_hash")
    run_directories = [
        Path(line.removeprefix("DITTO_RUN_DIR="))
        for line in completed.stderr.splitlines()
        if line.startswith("DITTO_RUN_DIR=")
    ]
    replay_event_log = run_directories[-1] / "logs/events.jsonl" if run_directories else None
    replay_event_hash = (
        event_transcript_hash(replay_event_log)
        if replay_event_log is not None and replay_event_log.is_file()
        else None
    )
    source_event_hash = recipe.get("source_event_transcript_hash")
    classification = (
        "nondeterministic-infrastructure"
        if (
            source_hash is not None
            and replay_hash is not None
            and (
                source_hash != replay_hash
                or (
                    source_event_hash is not None
                    and replay_event_hash is not None
                    and source_event_hash != replay_event_hash
                )
            )
        )
        else "stability-unestablished"
    )
    (output / "stability.json").write_text(json.dumps({
        "schema": 1,
        "classification": classification,
        "source_run_id": recipe.get("source_run_id"),
        "source_semantic_hash": source_hash,
        "replay_semantic_hash": replay_hash,
        "source_event_transcript_hash": source_event_hash,
        "replay_event_transcript_hash": replay_event_hash,
    }, indent=2, sort_keys=True) + "\n")
    print(completed.stderr, end="")
    print(
        f"Original {recipe['source_run_id']}: {recipe['source_status']}; "
        f"classification: {classification}; replay evidence: {output}"
    )
    return completed.returncode
