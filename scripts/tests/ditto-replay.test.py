#!/usr/bin/env python3
"""Exercise the supported replay command and its prerequisite failures."""

import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ditto_replay


def check_render_receipts(root: Path) -> None:
    """Receipt timing is incidental; pixels, action frames and order remain meaningful."""
    first = root / "receipt-first.jsonl"
    second = root / "receipt-second.jsonl"
    receipt = {
        "event_name": "ditto.context", "source": "ditto-player",
        "body": {"context": "artifact-accepted", "artifact_kind": {
            "kind": "screenshot", "checkpoint": "board",
            "render_commit": {"frame": 28, "render_generation": 417,
                              "pixel_fingerprint": 1234},
        }},
    }
    advance = {"event_name": "ditto.context", "body": {
        "context": "step-started", "advance": {"frames": 10}, "frame": 28,
        "render_generation": 417,
    }}
    first.write_text("\n".join(map(json.dumps, [receipt, advance])) + "\n")
    expected = ditto_replay.event_transcript_hash(first)

    def observe(events):
        second.write_text("\n".join(map(json.dumps, events)) + "\n")
        return ditto_replay.event_transcript_hash(second)

    commit = receipt["body"]["artifact_kind"]["render_commit"]
    commit.update(frame=27, render_generation=416)
    assert observe([receipt, advance]) == expected
    assert observe([advance, receipt]) != expected
    commit["pixel_fingerprint"] += 1
    assert observe([receipt, advance]) != expected
    commit["pixel_fingerprint"] -= 1
    for key in ("frame", "render_generation"):
        advance["body"][key] += 1
        assert observe([receipt, advance]) != expected
        advance["body"][key] -= 1
    advance["body"]["advance"]["frames"] += 1
    assert observe([receipt, advance]) != expected
    advance["body"]["advance"]["frames"] -= 1
    receipt["body"]["artifact_kind"]["checkpoint"] = "other-board"
    assert observe([receipt, advance]) != expected


def check_input_sessions(root: Path) -> None:
    """Generated UUIDs vary; input ownership transitions and receipts must not."""
    first_id = "89d2b5f2-33bf-4ba2-9620-e44b989718df"
    second_id = "07d8ce2c-8de8-4bbf-9691-612812298803"
    replacement_id = "e3c6c40c-94c8-4d2f-b60e-8173c4a88c9c"
    trace = {"session": first_id + ":19", "generation": 20, "receipts": [{
        "route": "world-logical", "expected_target": "pawn", "actual_hit": "pawn",
        "capture_owner": "pawn", "presentation_boundary": 15, "pointer_id": 0,
    }]}
    event = {"event_name": "ditto.context", "body": {"result": {"input_trace": trace}}}
    events = [copy.deepcopy(event) for _ in range(4)]
    events[2]["body"]["result"]["input_trace"]["session"] = second_id + ":20"
    path = root / "input-sessions.jsonl"

    def observe(values):
        path.write_text("\n".join(map(json.dumps, values)) + "\n")
        return ditto_replay.event_transcript_hash(path)

    expected = observe(events)
    renamed = json.loads(json.dumps(events).replace(first_id, replacement_id).replace(second_id, first_id))
    assert observe(renamed) == expected
    for session in (first_id + ":20", second_id + ":19", "named-session", None):
        changed = copy.deepcopy(events)
        changed[1]["body"]["result"]["input_trace"]["session"] = session
        assert observe(changed) != expected
    for key, value in (("route", "none"), ("expected_target", "board"),
                       ("actual_hit", None), ("capture_owner", None),
                       ("presentation_boundary", 16), ("pointer_id", 1)):
        changed = copy.deepcopy(events)
        changed[0]["body"]["result"]["input_trace"]["receipts"][0][key] = value
        assert observe(changed) != expected
    changed = copy.deepcopy(events)
    changed[0]["body"]["result"]["input_trace"]["generation"] += 1
    assert observe(changed) != expected
    assert observe(list(reversed(events))) != expected
    for value in (first_id, replacement_id):
        event["body"]["session"] = value
        events[0] = copy.deepcopy(event)
        if value == first_id:
            unrelated = observe(events)
        else:
            assert observe(events) != unrelated


def main() -> None:
    scripts = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="ditto-replay-test.") as temporary:
        root = Path(temporary)
        check_render_receipts(root)
        check_input_sessions(root)
        (root / "scripts").mkdir()
        for name in (
            "ditto_ci.py", "ditto_replay.py", "ditto_evidence.py", "operation_log.py",
            "perf_log.py", "platform_support.py", "process_identity.py", "process_priority.py", "process_usage.py",
            "ditto_build_leases.py", "ci_steps.py",
        ):
            shutil.copy2(scripts / name, root / "scripts" / name)
        config = root / "samples/chess/ditto.toml"
        config.parent.mkdir(parents=True)
        configuration = """name = 'fixture'
[[scenarios]]
name = 'gallery reset'
steps = [{screenshot = {name = 'initial'}}]
[[scenarios]]
name = 'collection components'
steps = []
"""
        config.write_text(configuration)
        config.with_name("ditto.lock").write_text("original baselines")
        binary = root / "ditto"
        binary.write_text(f"#!{sys.executable}\n" + '''
import json, os, pathlib, sys
if "--version" in sys.argv:
    print("4.5.0")
    raise SystemExit(0)
assert os.environ["DITTO_REPLAY_BUILD_FINGERPRINT"] == "a" * 64
assert "--no-build" in sys.argv
assert sys.argv[sys.argv.index("--profile") + 1] == "macos"
if sys.argv[-3] == "gallery reset":
    assert pathlib.Path(os.environ["DITTO_ODIFF_PATH"]).is_file()
else:
    assert sys.argv[-3] == "collection components"
    assert "DITTO_ODIFF_PATH" not in os.environ
pathlib.Path(os.environ["PLAYER_MARKER"]).write_text("executed")
output = pathlib.Path(sys.argv[sys.argv.index("--output") + 1])
output.write_text(json.dumps({"status":"passed", "errors":[]}))
''')
        binary.chmod(0o755)
        if os.name == "nt":
            script = binary
            binary = root / "ditto.cmd"
            binary.write_text(f'@"{sys.executable}" "{script}" %*\n')
        environment = {"DITTO_CACHE_ROOT": str(root / "cache"), "DITTO_ODIFF_PATH": str(binary)}
        recipe = ditto_replay.record(root, binary, root / "cache", "chess",
                                     ["gallery reset", "collection components"], environment)
        result = {"run_id": "original-failure", "status": "failed",
                  "build": {"fingerprint": "a" * 64}}
        retained = root / "replay.json"
        ditto_replay.save(recipe, retained, result)
        original = retained.read_bytes()
        marker = root / "player-started"
        env = os.environ.copy()
        env["PLAYER_MARKER"] = str(marker)
        env["DITTO_ODIFF_PATH"] = "/incorrect/ambient/tool"

        def invoke(recipe_path=retained, scenario="gallery reset"):
            return subprocess.run([sys.executable, str(root / "scripts/ditto_ci.py"),
                                   "replay", str(recipe_path), scenario],
                                  env=env, capture_output=True, text=True)

        binary.write_text("a rebuilt runner must not be used")
        passed = invoke()
        assert passed.returncode == 0, passed.stderr + passed.stdout
        assert marker.read_text() == "executed"
        assert "Original original-failure: failed" in passed.stdout
        assert retained.read_bytes() == original
        passed_observation = {
            "semantic_hash": "passed", "event_transcript_hash": "passed-events",
        }
        failed_observation = {
            "semantic_hash": "failed", "event_transcript_hash": "failed-events",
        }
        assert ditto_replay.classify_paired_observations(
            [passed_observation], [failed_observation]
        ) == "stability-unestablished"
        assert ditto_replay.classify_paired_observations(
            [passed_observation, passed_observation],
            [failed_observation, passed_observation],
        ) == "nondeterministic-infrastructure"
        assert ditto_replay.classify_paired_observations(
            [passed_observation, passed_observation],
            [failed_observation, failed_observation],
        ) == "candidate-introduced"
        base_evidence = [root / "base-one.json", root / "base-two.json"]
        candidate_evidence = [root / "candidate-one.json", root / "candidate-two.json"]
        for path in base_evidence:
            path.write_text(json.dumps({
                "replay_semantic_hash": "passed",
                "replay_event_transcript_hash": "passed-events",
            }))
        for path in candidate_evidence:
            path.write_text(json.dumps({
                "replay_semantic_hash": "failed",
                "replay_event_transcript_hash": "failed-events",
            }))
        classification_path = root / "classification.json"
        assert ditto_replay.classify_and_retain(
            base_evidence, candidate_evidence, classification_path
        ) == "candidate-introduced"
        retained_classification = json.loads(classification_path.read_text())
        assert retained_classification["classification"] == "candidate-introduced"
        assert len(retained_classification["base"]) == 2
        assert len(retained_classification["candidate"]) == 2
        first_events = root / "first-events.jsonl"
        second_events = root / "second-events.jsonl"
        first_events.write_text(json.dumps({
            "sequence": 1, "timestamp_unix_us": 10, "job_id": "one",
            "event_name": "ditto.step", "body": {"scenario_id": "one", "status": "passed"},
        }) + "\n")
        second_events.write_text(json.dumps({
            "sequence": 99, "timestamp_unix_us": 20, "job_id": "two",
            "event_name": "ditto.step", "body": {"scenario_id": "two", "status": "passed"},
        }) + "\n")
        assert ditto_replay.event_transcript_hash(
            first_events
        ) == ditto_replay.event_transcript_hash(second_events)
        second_events.write_text(second_events.read_text() + json.dumps({
            "event_name": "battlement.frame.slow", "duration_ms": 200,
        }) + "\n")
        assert ditto_replay.event_transcript_hash(
            first_events
        ) == ditto_replay.event_transcript_hash(second_events)
        second_events.write_text(second_events.read_text().replace("passed", "failed"))
        assert ditto_replay.event_transcript_hash(
            first_events
        ) != ditto_replay.event_transcript_hash(second_events)
        marker.unlink()

        no_comparison = ditto_replay.record(
            root, Path(recipe["tools"]["runner"]["path"]), root / "cache", "chess",
            ["collection components"], {"DITTO_CACHE_ROOT": str(root / "cache")},
        )
        no_comparison_path = root / "no-comparison.json"
        ditto_replay.save(no_comparison, no_comparison_path, result)
        no_odiff = invoke(no_comparison_path, "collection components")
        assert no_odiff.returncode == 0, no_odiff.stderr + no_odiff.stdout
        assert marker.read_text() == "executed"
        marker.unlink()

        config.write_text("changed scenario")
        changed = invoke()
        assert changed.returncode == 1
        assert "Replay configuration changed" in changed.stderr
        assert not marker.exists()
        config.write_text(configuration)
        pinned = Path(recipe["tools"]["DITTO_ODIFF_PATH"]["path"])
        pinned.unlink()
        missing = invoke()
        assert missing.returncode == 1
        assert "dependency is missing or changed" in missing.stderr
        assert not marker.exists()
        assert retained.read_bytes() == original
    print("Ditto replay tests passed.")


if __name__ == "__main__":
    main()
