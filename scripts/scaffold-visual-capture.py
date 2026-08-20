#!/usr/bin/env python3

"""Create a formatted Masonry visual-capture scenario and authored scene."""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import tempfile


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Create a formatted scenario component, its .meta, and an authored scene "
            "containing one matching scenario plus the reusable Masonry capture shell."
        )
    )
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--type", dest="type_name", required=True)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def fail(message: str, status: int = 2) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(status)


def tail(path: Path, count: int) -> None:
    lines = path.read_text(errors="replace").splitlines()
    print("\n".join(lines[-count:]), file=sys.stderr)


def main() -> None:
    args = parse_arguments()
    if re.fullmatch(r"[A-Za-z0-9._-]+", args.scenario) is None:
        fail("A safe --scenario name is required.")
    if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", args.type_name) is None:
        fail("A C# --type name is required.")
    if len(args.output.parts) < 2 or args.output.parts[0] != "Assets":
        fail("--output must be a directory under Assets.")
    if ".." in args.output.parts:
        fail("--output may not traverse parents.")

    version_file = REPOSITORY_ROOT / "ProjectSettings/ProjectVersion.txt"
    unity_version = next(
        line.removeprefix("m_EditorVersion: ")
        for line in version_file.read_text().splitlines()
        if line.startswith("m_EditorVersion: ")
    )
    unity_editor = Path(
        os.environ.get(
            "UNITY_EDITOR",
            f"/Applications/Unity/Hub/Editor/{unity_version}/Unity.app/Contents/MacOS/Unity",
        )
    )
    if not os.access(unity_editor, os.X_OK):
        fail(f"Unity {unity_version} was not found at {unity_editor}.", 1)

    script_path = args.output / f"{args.type_name}.cs"
    scene_path = args.output / f"{args.type_name}.unity"
    if (REPOSITORY_ROOT / script_path).exists() or (REPOSITORY_ROOT / scene_path).exists():
        fail("Refusing to overwrite existing scenario output.", 1)
    (REPOSITORY_ROOT / args.output).mkdir(parents=True, exist_ok=True)
    (REPOSITORY_ROOT / script_path).write_text(
        f'''#nullable enable

using Masonry.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;

public sealed class {args.type_name} : MasonryCaptureScenario
{{
    private bool awaitingPress;
    private bool awaitingRelease;

    public override string ScenarioName => "{args.scenario}";

    protected override void BeginCapture()
    {{
        Object.FindAnyObjectByType<MasonryCaptureShell>().SetPhase("Before interaction");
        awaitingPress = true;
        RequestPointerInput(
            new[] {{ "initial-state-rendered" }},
            CapturePointerAction.LeftButtonDown,
            new Vector2(0.5f, 0.5f)
        );
    }}

    private void Update()
    {{
        if (awaitingPress && Mouse.current.leftButton.wasPressedThisFrame)
        {{
            awaitingPress = false;
            awaitingRelease = true;
            RequestPointerInput(
                new[] {{ "initial-state-rendered", "requested-press-observed" }},
                CapturePointerAction.LeftButtonUp,
                new Vector2(0.5f, 0.5f)
            );
            return;
        }}

        if (!awaitingRelease || !Mouse.current.leftButton.wasReleasedThisFrame)
        {{
            return;
        }}

        awaitingRelease = false;
        Object.FindAnyObjectByType<MasonryCaptureShell>().SetPhase("Interaction passed");
        SignalPassed(
            new[]
            {{
                "initial-state-rendered",
                "requested-press-observed",
                "requested-release-observed",
            }}
        );
    }}
}}
'''
    )
    guid = hashlib.sha256(f"{script_path.as_posix()}:{args.scenario}".encode()).hexdigest()[:32]
    (REPOSITORY_ROOT / f"{script_path}.meta").write_text(f"fileFormatVersion: 2\nguid: {guid}\n")

    with tempfile.NamedTemporaryFile(prefix="masonry-scaffold.", delete=False) as log_file:
        unity_log = Path(log_file.name)
    try:
        environment = os.environ.copy()
        environment.update(
            MASONRY_CAPTURE_SCAFFOLD_SCENE=scene_path.as_posix(),
            MASONRY_CAPTURE_SCAFFOLD_SCRIPT=script_path.as_posix(),
            MASONRY_CAPTURE_SCAFFOLD_TYPE=args.type_name,
        )
        result = subprocess.run(
            [
                str(unity_editor), "-batchmode", "-nographics", "--burst-disable-compilation",
                "-quit", "-projectPath", str(REPOSITORY_ROOT), "-executeMethod",
                "Masonry.Editor.VisualCaptureScaffold.CreateScene", "-logFile", str(unity_log),
            ],
            env=environment,
        )
        if result.returncode != 0:
            tail(unity_log, 120)
            raise SystemExit(1)
        if (
            f"MASONRY_CAPTURE_SCAFFOLD_OK:{scene_path.as_posix()}"
            not in unity_log.read_text(errors="replace")
        ):
            tail(unity_log, 120)
            fail("Unity exited without completing scenario scaffolding.", 1)
    finally:
        unity_log.unlink(missing_ok=True)
    print(f"Created {script_path}, {scene_path}, and matching metadata.")


def interrupted(_signal_number, _frame) -> None:
    raise KeyboardInterrupt


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    try:
        main()
    except KeyboardInterrupt:
        raise SystemExit(130) from None
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
