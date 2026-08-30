#!/usr/bin/env python3

"""Reject pre-Ditto tooling and verify the active CI contract."""

from __future__ import annotations

from pathlib import Path
import subprocess
import tomllib


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
NEGATIVE_FIXTURE = Path("scripts/tests/fixtures/ditto-cutover-rejected.txt")
ALLOWED_NEGATIVE_FIXTURES = frozenset({NEGATIVE_FIXTURE})
FORBIDDEN_MARKERS = tuple(
    marker.lower()
    for marker in (
        "visual" + "-capture",
        "visual" + "_capture",
        "visual " + "capture",
        "capture-" + "visual-evidence.py",
        "capture-sample-" + "visual-evidence.py",
        "scaffold-visual-" + "capture.py",
        "visual_" + "capture_lib",
        "visual_" + "capture_options",
        "visual_" + "capture_slots",
        "Visual" + "Capture",
        "SampleVisual" + "CaptureBuild",
        "Visual" + "Capture" + "Build",
        "Visual" + "Capture" + "Assets",
        "Visual" + "Capture" + "Scaffold",
        "Assets/Visual" + "Capture",
        "BattlementIntegration" + "CaptureScenario",
        "ditto_" + "shadow",
        "ditto-" + "shadow",
    )
)
DITTO_PATHS = (
    Path(".tollgate/config.toml"),
    Path("crates/battlement-ditto"),
    Path("crates/battlement-tooling"),
    Path("docs/ditto.md"),
    Path("docs/ditto-implementation-plan.md"),
    Path("docs/ditto-technical-design.md"),
    Path("Packages/com.battlement.client/Editor/BattlementDittoBuild.cs"),
    Path("scripts/ditto_ci.py"),
)
UNSUPPORTED_ARCHITECTURES = (
    "x86" + "_64",
    "x86" + "-64",
    "intel" + " mac",
    "amd" + "64",
    "universal" + "2",
)


def tracked_paths() -> list[Path]:
    """Return every path present in the staged repository tree."""
    output = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
    ).stdout
    return [Path(value.decode()) for value in output.split(b"\0") if value]


def read_text(path: Path) -> str | None:
    data = (REPOSITORY_ROOT / path).read_bytes()
    if b"\0" in data:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return None


def check_removed_references(paths: list[Path]) -> None:
    fixture = read_text(NEGATIVE_FIXTURE)
    assert fixture is not None
    assert all(marker in fixture.lower() for marker in FORBIDDEN_MARKERS)
    violations = []
    for path in paths:
        if path in ALLOWED_NEGATIVE_FIXTURES:
            continue
        text = read_text(path)
        if text is None:
            continue
        for marker in FORBIDDEN_MARKERS:
            if marker in text.lower():
                violations.append(f"{path}: rejected marker {marker!r}")
    assert not violations, "\n".join(violations)


def in_ditto_scope(path: Path) -> bool:
    return any(path == root or root in path.parents for root in DITTO_PATHS)


def check_apple_silicon_only(paths: list[Path]) -> None:
    violations = []
    for path in paths:
        if not in_ditto_scope(path):
            continue
        text = read_text(path)
        if text is None:
            continue
        for architecture in UNSUPPORTED_ARCHITECTURES:
            if architecture in text.lower():
                violations.append(f"{path}: unsupported architecture {architecture!r}")
    assert not violations, "\n".join(violations)


def check_ci_matrix() -> None:
    config = tomllib.loads(
        (REPOSITORY_ROOT / ".tollgate/config.toml").read_text(encoding="utf-8")
    )
    steps = {step["name"]: step for step in config["step"]}
    required = {
        "ditto-cold-prepare",
        "ditto-warm-prepare",
        "ditto-basic",
        "ditto-tictactoe",
        "ditto-reactant",
        "ditto-chess",
        "ditto-ui",
        "ditto-webgl-adapter",
        "ditto-ios-adapter",
        "ditto-performance",
        "ditto-publish-baselines",
    }
    assert required <= steps.keys()
    assert steps["ditto-warm-prepare"]["needs"] == ["ditto-cold-prepare"]
    publication = steps["ditto-publish-baselines"]
    assert publication.get("final") is True
    assert required - {"ditto-cold-prepare", "ditto-warm-prepare", publication["name"]} <= set(
        publication["needs"]
    ) | {"ditto-performance"}
    runner = "python3 scripts/" + "ditto_ci.py "
    assert all(steps[name]["run"].startswith(runner) for name in required if name != publication["name"])
    assert publication["run"].startswith(runner)


def check_handoff_guidance() -> None:
    design = (REPOSITORY_ROOT / "docs/ditto-technical-design.md").read_text()
    implementation = (REPOSITORY_ROOT / "docs/implementation-plan.md").read_text()
    for marker in ("Ditto: passed", "Ditto: not applicable", "player-visible runtime result"):
        assert marker in design
    assert "Ditto: passed" in implementation
    assert "Ditto: not applicable" in implementation


def main() -> None:
    paths = tracked_paths()
    assert ALLOWED_NEGATIVE_FIXTURES <= set(paths)
    check_removed_references(paths)
    check_apple_silicon_only(paths)
    check_ci_matrix()
    check_handoff_guidance()
    print("Ditto cutover tests passed.")


if __name__ == "__main__":
    main()
