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
            "ditto_build_leases.py", "ci_steps.py", "resource_slots.py",
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
names = [name for name in ("gallery reset", "collection components") if name in sys.argv]
if "gallery reset" in names:
    assert pathlib.Path(os.environ["DITTO_ODIFF_PATH"]).is_file()
else:
    assert names == ["collection components"]
    assert "DITTO_ODIFF_PATH" not in os.environ
pathlib.Path(os.environ["PLAYER_MARKER"]).write_text("executed")
output = pathlib.Path(sys.argv[sys.argv.index("--output") + 1])
status = os.environ.get("REPLAY_STATUS", "passed")
output.write_text(json.dumps({"status":status, "errors":[],
    "scenarios":[{"name":name, "status":status} for name in names]}))
if "REPLAY_EVENTS" in os.environ:
    run = output.parent / "fake-run"
    (run / "logs").mkdir(parents=True)
    (run / "logs/events.jsonl").write_text(os.environ["REPLAY_EVENTS"] + "\\n")
    print("DITTO_RUN_DIR=" + str(run), file=sys.stderr)
raise SystemExit(int(os.environ.get("REPLAY_EXIT_CODE", "0")))
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
            selection = [scenario] if scenario is not None else []
            return subprocess.run([sys.executable, str(root / "scripts/ditto_ci.py"),
                                   "replay", str(recipe_path), *selection],
                                  env=env, capture_output=True, text=True)

        binary.write_text("a rebuilt runner must not be used")
        passed = invoke()
        assert passed.returncode == 0, passed.stderr + passed.stdout
        assert marker.read_text() == "executed"
        assert "Original original-failure: failed" in passed.stdout
        assert retained.read_bytes() == original
        passed_observation = {
            "semantic_hash": "passed", "event_transcript_hash": "passed-events",
            "scenarios": ["gallery reset", "collection components"],
        }
        failed_observation = {
            "semantic_hash": "failed", "event_transcript_hash": "failed-events",
            "scenarios": ["gallery reset", "collection components"],
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
        raw_events = root / "paired-events.jsonl"
        for path in base_evidence:
            raw_events.write_text(json.dumps({"status": "passed"}) + "\n")
            path.write_text(json.dumps({
                "replay_scenarios": ["gallery reset", "collection components"],
                "replay_semantic_hash": "passed",
                "replay_event_transcript_hash": "passed-events",
                "replay_event_log": ditto_replay.retain_transcript(raw_events, path),
            }))
        for path in candidate_evidence:
            raw_events.write_text(json.dumps({"status": "failed"}) + "\n")
            path.write_text(json.dumps({
                "replay_scenarios": ["gallery reset", "collection components"],
                "replay_semantic_hash": "failed",
                "replay_event_transcript_hash": "failed-events",
                "replay_event_log": ditto_replay.retain_transcript(raw_events, path),
            }))
        classification_path = root / "classification.json"
        assert ditto_replay.classify_and_retain(
            base_evidence, candidate_evidence, classification_path
        ) == "candidate-introduced"
        retained_classification = json.loads(classification_path.read_text())
        assert retained_classification["classification"] == "candidate-introduced"
        assert len(retained_classification["base"]) == 2
        assert len(retained_classification["candidate"]) == 2
        assert retained_classification["scope_comparison"] == "matching scenario selections"
        unknown_scope = json.loads(candidate_evidence[0].read_text())
        del unknown_scope["replay_scenarios"]
        candidate_evidence[0].write_text(json.dumps(unknown_scope))
        assert ditto_replay.classify_and_retain(
            base_evidence, candidate_evidence, classification_path
        ) == "incomparable-scope"
        assert json.loads(classification_path.read_text())["scope_comparison"].startswith("unavailable:")
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

        source_events = root / "source-events.jsonl"
        input_event = {"body": {"result": {"input_trace": {
            "session": "89d2b5f2-33bf-4ba2-9620-e44b989718df:19",
            "generation": 20, "receipts": [{"route": "world-logical"}],
        }}}}
        source_events.write_text(json.dumps(input_event) + "\n")
        portable = root / "portable"
        portable.mkdir()
        current_recipe = dict(recipe)
        ditto_replay.save(current_recipe, portable / "replay.json", {
            **result, "status": "passed", "errors": [],
            "scenarios": [{"name": name, "status": "passed"} for name in recipe["scenarios"]],
        }, source_events)
        current_recipe["source_event_transcript_hash"] = "obsolete-normalization"
        (portable / "replay.json").write_text(json.dumps(current_recipe))
        moved = root / "moved"
        portable.rename(moved)
        source_events.unlink()
        env["REPLAY_EVENTS"] = json.dumps(input_event).replace(
            "89d2b5f2-33bf-4ba2-9620-e44b989718df", "07d8ce2c-8de8-4bbf-9691-612812298803"
        )

        def replay_observation(scenario=None, expected_exit=0):
            completed = invoke(moved / "replay.json", scenario)
            assert completed.returncode == expected_exit, completed.stderr + completed.stdout
            evidence = Path(completed.stdout.rsplit("replay evidence: ", 1)[1].splitlines()[0])
            return json.loads((evidence / "stability.json").read_text()), evidence

        subset, subset_evidence = replay_observation("gallery reset")
        assert subset["classification"] == "incomparable-scope"
        assert subset["source_scenarios"] == recipe["scenarios"]
        assert subset["replay_scenarios"] == ["gallery reset"]
        assert subset["scope_comparison"] == "incomparable: scenario selections differ"
        assert subset["transcript_comparison"].startswith("incomparable:")
        assert subset["source_semantic_hash"] != subset["replay_semantic_hash"]
        assert ditto_replay.classify_and_retain(
            [moved / "replay.json"] * 2, [subset_evidence / "stability.json"] * 2,
            classification_path,
        ) == "incomparable-scope"
        env["REPLAY_STATUS"] = "failed"
        env["REPLAY_EXIT_CODE"] = "7"
        failed_subset, _ = replay_observation("gallery reset", 7)
        assert failed_subset["classification"] == "incomparable-scope"
        del env["REPLAY_STATUS"], env["REPLAY_EXIT_CODE"]

        same, evidence = replay_observation()
        assert same["classification"] == "stability-unestablished"
        assert same["transcript_comparison"] == "compared"
        assert same["source_event_transcript_hash"] == same["replay_event_transcript_hash"]
        assert ditto_replay.paired_observation(evidence / "stability.json") == \
            ditto_replay.paired_observation(moved / "replay.json")
        env["REPLAY_EVENTS"] = env["REPLAY_EVENTS"].replace("world-logical", "none")
        different, _ = replay_observation()
        assert different["classification"] == "nondeterministic-infrastructure"
        env["REPLAY_EVENTS"] = env["REPLAY_EVENTS"].replace("none", "world-logical")
        env["REPLAY_STATUS"] = "failed"
        different, _ = replay_observation()
        assert different["classification"] == "nondeterministic-infrastructure"
        del env["REPLAY_STATUS"]
        retained_events = moved / current_recipe["source_event_log"]["path"]
        for changed in (True, False):
            if changed:
                retained_events.write_text("changed evidence\n")
            else:
                retained_events.unlink()
            unavailable, _ = replay_observation()
            assert unavailable["classification"] == "stability-unestablished"
            assert unavailable["transcript_comparison"] == "unavailable: source transcript missing or changed"
            try:
                ditto_replay.paired_observation(moved / "replay.json")
            except ValueError as error:
                assert "verified transcript evidence" in str(error)
            else:
                raise AssertionError("Unavailable source transcript accepted for attribution")
        del env["REPLAY_EVENTS"]
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
