#!/usr/bin/env python3
"""Certify controlled pointer delivery in isolated and concurrent native players."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import selectors
import shutil
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PLAYER_IDENTITY = re.compile(r"player-identity pid=(\d+)")
RUN_DIRECTORY = re.compile(r"^DITTO_RUN_DIR=(.+)$")
POINTER_CAPTURE = re.compile(r"pointer-receipt .* capture=([^ ]+)")


@dataclass
class RunningCapture:
    name: str
    process: subprocess.Popen[str]
    result_path: Path
    stdout_path: Path
    started_ns: int
    run_directory: Path | None = None
    player_pid: int | None = None
    completed_ns: int | None = None


def fragment(trajectories: int) -> str:
    if trajectories < 2 or trajectories % 2 != 0:
        raise ValueError("trajectories per launch must be an even number of at least two")
    per_scenario = trajectories // 2
    if 3 + per_scenario * 4 > 128:
        raise ValueError("the requested trajectories exceed Ditto's 128-step scenario bound")
    chunks = ['name = "controlled pointer stress"\n']
    for scenario_index in range(2):
        chunks.extend(
            [
                "[[scenarios]]\n",
                f'name = "controlled pointer stress {scenario_index + 1}"\n',
                'fixture = "pointer-routing"\n',
                "[[scenarios.steps]]\n",
                'wait = { object = "39110000-0000-4000-8000-000000000001", state = "visible" }\n',
                "[[scenarios.steps]]\n",
                'pointer_action = { target = { role = "button", name = "Toggle passthrough" }, action = "click" }\n',
            ]
        )
        for trajectory in range(per_scenario):
            destination = "[0.82, 0.22]" if trajectory % 2 == 0 else "[0.18, 0.78]"
            chunks.extend(
                [
                    "[[scenarios.steps]]\n",
                    'pointer = { phase = "press", target = "39110000-0000-4000-8000-000000000001" }\n',
                    "[[scenarios.steps]]\n",
                    "advance = { frames = 1 }\n",
                    "[[scenarios.steps]]\n",
                    f'pointer = {{ phase = "move", target = {destination} }}\n',
                    "[[scenarios.steps]]\n",
                    f'pointer = {{ phase = "release", target = {destination} }}\n',
                ]
            )
        chunks.extend(
            [
                "[[scenarios.steps]]\n",
                f'screenshot = {{ name = "stress-terminal-{scenario_index + 1}" }}\n',
            ]
        )
    return "".join(chunks)


def normalized_result(result: dict[str, object]) -> dict[str, object]:
    normalized_scenarios: list[dict[str, object]] = []
    for scenario_index, scenario_value in enumerate(result["scenarios"]):
        scenario = scenario_value
        receipts: list[dict[str, object]] = []
        screenshot_hashes: list[str] = []
        first_boundary: int | None = None
        for step_value in scenario["steps"]:
            step = step_value
            trace = step.get("input_trace")
            if trace is not None:
                for receipt_value in trace["receipts"]:
                    receipt = dict(receipt_value)
                    boundary = int(receipt.pop("presentation_boundary"))
                    if first_boundary is None:
                        first_boundary = boundary
                    receipt["presentation_boundary"] = boundary - first_boundary
                    receipt["x"] = round(float(receipt["x"]), 4)
                    receipt["y"] = round(float(receipt["y"]), 4)
                    receipts.append(receipt)
            screenshot = step.get("screenshot")
            if screenshot is not None:
                screenshot_hashes.append(screenshot["actual"]["sha256"])
        normalized_scenarios.append(
            {
                "index": scenario_index,
                "status": scenario["status"],
                "receipts": receipts,
                "screenshots": screenshot_hashes,
            }
        )
    return {"status": result["status"], "scenarios": normalized_scenarios}


def digest(value: object) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def capture_command(
    config: Path, fragment_path: Path, result_path: Path, no_build: bool
) -> list[str]:
    command = [
        "cargo",
        "run",
        "-q",
        "-p",
        "rt",
        "--",
        "ditto",
        "--config",
        str(config),
        "capture",
        "--profile",
        "macos",
        "--fragment",
        str(fragment_path),
        "--output",
        str(result_path),
        "--json",
    ]
    if no_build:
        command.insert(-3, "--no-build")
    return command


def start_capture(
    name: str,
    config: Path,
    fragment_path: Path,
    directory: Path,
    *,
    no_build: bool,
) -> RunningCapture:
    directory.mkdir(parents=True)
    result_path = directory / "result.json"
    stdout_path = directory / "stdout.log"
    process = subprocess.Popen(
        capture_command(config, fragment_path, result_path, no_build),
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )
    return RunningCapture(name, process, result_path, stdout_path, time.monotonic_ns())


def activate_process(pid: int) -> bool:
    source = (
        'tell application "System Events" to set frontmost of '
        f'(first process whose unix id is {pid}) to true'
    )
    return subprocess.run(
        ["osascript", "-e", source], capture_output=True, text=True, check=False
    ).returncode == 0


def activate_finder() -> bool:
    return subprocess.run(
        ["osascript", "-e", 'tell application "Finder" to activate'],
        capture_output=True,
        text=True,
        check=False,
    ).returncode == 0


def move_physical_pointer(index: int) -> bool:
    coordinate = 20 if index % 2 == 0 else 44
    source = (
        'ObjC.import("CoreGraphics");'
        f'var p=$.CGPointMake({coordinate},{coordinate});'
        'var e=$.CGEventCreateMouseEvent(null,$.kCGEventMouseMoved,p,$.kCGMouseButtonLeft);'
        '$.CGEventPost($.kCGHIDEventTap,e);'
    )
    return subprocess.run(
        ["osascript", "-l", "JavaScript", "-e", source],
        capture_output=True,
        text=True,
        check=False,
    ).returncode == 0


def native_player_pids() -> list[int]:
    process = subprocess.run(
        ["pgrep", "-f", "BattlementDitto.app/Contents/MacOS/BattlementDitto"],
        capture_output=True,
        text=True,
        check=False,
    )
    if process.returncode not in (0, 1):
        return []
    return [int(value) for value in process.stdout.split()]


def discover_player(capture: RunningCapture) -> None:
    if capture.run_directory is None or capture.player_pid is not None:
        return
    logs = sorted((capture.run_directory / "logs").glob("player-*.log"))
    if not logs:
        return
    match = PLAYER_IDENTITY.search(logs[0].read_text(errors="replace"))
    if match is not None:
        capture.player_pid = int(match.group(1))


def run_group(captures: list[RunningCapture], perturb: bool) -> dict[str, int | bool]:
    selector = selectors.DefaultSelector()
    streams: dict[object, RunningCapture] = {}
    writers = {}
    for capture in captures:
        assert capture.process.stdout is not None
        selector.register(capture.process.stdout, selectors.EVENT_READ)
        streams[capture.process.stdout] = capture
        writers[capture.name] = capture.stdout_path.open("w")
    perturbations = 0
    focus_successes = 0
    pointer_successes = 0
    next_perturbation = time.monotonic()
    if perturb:
        activate_finder()
    try:
        while any(capture.completed_ns is None for capture in captures):
            for key, _ in selector.select(timeout=0.05):
                capture = streams[key.fileobj]
                line = key.fileobj.readline()
                if line:
                    writers[capture.name].write(line)
                    writers[capture.name].flush()
                    match = RUN_DIRECTORY.match(line.rstrip())
                    if match is not None:
                        capture.run_directory = Path(match.group(1))
                elif capture.process.poll() is not None:
                    selector.unregister(key.fileobj)
            for capture in captures:
                discover_player(capture)
                if capture.completed_ns is None and capture.process.poll() is not None:
                    capture.completed_ns = time.monotonic_ns()
            if perturb and time.monotonic() >= next_perturbation:
                live_pids = native_player_pids()
                if live_pids:
                    focus_successes += int(activate_process(live_pids[perturbations % len(live_pids)]))
                    pointer_successes += int(move_physical_pointer(perturbations))
                    focus_successes += int(activate_finder())
                    perturbations += 1
                next_perturbation = time.monotonic() + 0.2
    finally:
        for writer in writers.values():
            writer.close()
        selector.close()
    overlap = max(value.started_ns for value in captures) < min(
        value.completed_ns or 0 for value in captures
    )
    return {
        "overlap": overlap,
        "focus_attempts": perturbations * 2,
        "focus_successes": focus_successes,
        "pointer_attempts": perturbations,
        "pointer_successes": pointer_successes,
    }


def inspect_capture(
    capture: RunningCapture, expected_digest: str | None, trajectories: int
) -> dict[str, object]:
    if capture.process.returncode != 0 or not capture.result_path.exists():
        raise RuntimeError(f"{capture.name} failed; inspect {capture.stdout_path}")
    result = json.loads(capture.result_path.read_text())
    normalized = normalized_result(result)
    normalized_digest = digest(normalized)
    if result["status"] != "passed":
        raise RuntimeError(f"{capture.name} returned {result['status']}")
    receipts = sum(
        len(scenario["receipts"]) for scenario in normalized["scenarios"]
    )
    if receipts != trajectories * 3:
        raise RuntimeError(f"{capture.name} retained {receipts} receipts, expected {trajectories * 3}")
    if expected_digest is not None and normalized_digest != expected_digest:
        raise RuntimeError(f"{capture.name} differs from isolated trace {expected_digest}")
    if capture.run_directory is None:
        raise RuntimeError(f"{capture.name} did not report its run directory")
    retained = capture.result_path.parent / "run"
    shutil.copytree(capture.run_directory, retained)
    player_logs = sorted((retained / "logs").glob("player-*.log"))
    log = player_logs[0].read_text(errors="replace") if player_logs else ""
    lines = log.splitlines()
    first_receipt = next((index for index, line in enumerate(lines) if "pointer-receipt" in line), -1)
    background_before_input = any(
        "application-focus focus=False" in line for line in lines[:first_receipt]
    )
    held = False
    focus_while_held = 0
    for line in lines:
        capture_match = POINTER_CAPTURE.search(line)
        if capture_match is not None:
            held = capture_match.group(1) != "none"
        if held and "application-focus focus=False" in line and "ditto=True" in line:
            focus_while_held += 1
    return {
        "name": capture.name,
        "run_id": result["run_id"],
        "native_execution_id": result["player_sessions"][0]["startup_report"][
            "native_execution_id"
        ],
        "player_pid": capture.player_pid,
        "duration_ms": result["duration_ms"],
        "receipts": receipts,
        "trajectories": trajectories,
        "normalized_trace_sha256": normalized_digest,
        "background_before_input": background_before_input,
        "focus_losses_while_held_drag": focus_while_held,
        "terminal_screenshots": [
            screenshot
            for scenario in normalized["scenarios"]
            for screenshot in scenario["screenshots"]
        ],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, default=ROOT / "samples/reactant/ditto.toml")
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--launches", type=int, default=20)
    parser.add_argument("--players", type=int, default=2)
    parser.add_argument("--trajectories", type=int, default=50)
    parser.add_argument(
        "--desktop-perturbation-launches",
        type=int,
        default=0,
        help="explicitly allow focus and physical-pointer perturbation for this many launches",
    )
    parser.add_argument("--allow-noncertifying", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.desktop_perturbation_launches < 0:
        raise SystemExit("desktop perturbation launch count cannot be negative")
    if args.desktop_perturbation_launches > args.launches:
        raise SystemExit("desktop perturbation launches cannot exceed total launches")
    if not args.allow_noncertifying and (
        args.launches < 20
        or args.players < 2
        or args.desktop_perturbation_launches == 0
    ):
        raise SystemExit(
            "certification requires at least 20 launches, two concurrent players, "
            "and explicit desktop perturbation"
        )
    args.output_dir.mkdir(parents=True, exist_ok=False)
    with tempfile.TemporaryDirectory(prefix="battlement-pointer-stress.") as temporary:
        fragment_path = Path(temporary) / "stress.toml"
        fragment_path.write_text(fragment(args.trajectories))
        shutil.copy2(fragment_path, args.output_dir / "stress.toml")
        isolated = start_capture(
            "isolated",
            args.config.resolve(),
            fragment_path,
            args.output_dir / "isolated",
            no_build=False,
        )
        run_group([isolated], perturb=False)
        isolated_record = inspect_capture(isolated, None, args.trajectories)
        reference = str(isolated_record["normalized_trace_sha256"])
        launches: list[dict[str, object]] = []
        for launch in range(args.launches):
            group = [
                start_capture(
                    f"launch-{launch + 1:02d}-player-{player + 1}",
                    args.config.resolve(),
                    fragment_path,
                    args.output_dir / f"launch-{launch + 1:02d}" / f"player-{player + 1}",
                    no_build=True,
                )
                for player in range(args.players)
            ]
            perturb = launch < args.desktop_perturbation_launches
            perturbation = run_group(group, perturb=perturb)
            records = [
                inspect_capture(capture, reference, args.trajectories) for capture in group
            ]
            if not perturbation["overlap"]:
                raise RuntimeError(f"launch {launch + 1} players did not overlap")
            if perturb:
                if (
                    perturbation["pointer_attempts"] == 0
                    or perturbation["pointer_successes"] != perturbation["pointer_attempts"]
                ):
                    raise RuntimeError(
                        f"launch {launch + 1} did not complete every physical pointer perturbation"
                    )
                if any(not record["background_before_input"] for record in records):
                    raise RuntimeError(
                        f"launch {launch + 1} did not begin controlled input in background"
                    )
                if any(record["focus_losses_while_held_drag"] == 0 for record in records):
                    raise RuntimeError(
                        f"launch {launch + 1} lacked a focus loss during a held drag"
                    )
            launches.append({"index": launch + 1, "perturbation": perturbation, "players": records})
            print(f"launch {launch + 1}/{args.launches}: passed", flush=True)
    summary = {
        "schema": 1,
        "status": "passed",
        "players": args.players,
        "launches": args.launches,
        "desktop_perturbation_launches": args.desktop_perturbation_launches,
        "certifying": not args.allow_noncertifying,
        "trajectories_per_player_per_launch": args.trajectories,
        "trajectories_per_player": args.launches * args.trajectories,
        "isolated": isolated_record,
        "normalized_trace_sha256": reference,
        "concurrent_launches": launches,
    }
    (args.output_dir / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
