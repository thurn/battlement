"""Select declared browser risks without making public demos a validation dependency."""

from __future__ import annotations

from dataclasses import dataclass, field
from fnmatch import fnmatchcase
import gzip
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

from web_compatibility import check_site


@dataclass
class Selection:
    """Browser work selected by changed risk, separated by build requirement."""

    players: dict[str, list[str]] = field(default_factory=dict)
    fixtures: list[str] = field(default_factory=list)

    def report(self) -> dict[str, object]:
        return {"fixtures": self.fixtures, "players": self.players}

    def __bool__(self) -> bool:
        return bool(self.players or self.fixtures)


def select(repository: Path, paths: list[str], names: list[str]) -> Selection:
    """Map changed paths to browser contracts and concrete platform risks."""
    risks = tomllib.loads((repository / "web/contracts.toml").read_text())["risks"]
    for risk in risks:
        reason = risk["reason"].strip()
        kind = risk.get("kind", "player")
        samples = risk.get("samples", [])
        declared = names if samples == ["*"] else samples
        if kind not in {"fixture", "player"} or not reason:
            raise RuntimeError("Browser risk declares an unknown kind or empty reason")
        if kind == "fixture" and samples:
            raise RuntimeError("Fixture browser risks cannot select Unity samples")
        if kind == "player" and (not declared or not set(declared).issubset(names)):
            raise RuntimeError("Browser risk declares an unknown sample or empty reason")
    selected = Selection()
    for path in paths:
        player_reasons = []
        fixture_reasons = []
        if path in {"web/checks/shared.js", "scripts/web_compatibility.py", "scripts/playwright_mcp.py"}:
            fixture_reasons.append("Browser contract execution")
        if path == "web/checks/fixture.js":
            fixture_reasons.append("Small browser harness fixture")
        for sample in names:
            if path == f"web/checks/{sample}.js":
                player_reasons.append(([sample], "Declared sample interaction"))
        for risk in risks:
            if any(fnmatchcase(path, pattern) for pattern in risk["paths"]):
                reason = risk["reason"].strip()
                kind = risk.get("kind", "player")
                samples = risk.get("samples", [])
                samples = names if samples == ["*"] else samples
                if kind == "fixture":
                    fixture_reasons.append(reason)
                    continue
                player_reasons.append((samples, reason))
        source = repository / path
        if path.startswith("Packages/com.battlement.client/Runtime/") and path.endswith(".cs"):
            if not source.exists() or "UNITY_WEBGL" in source.read_text():
                representative = "basic"
                if representative not in names:
                    raise RuntimeError(f"Missing browser representative sample: {representative}")
                player_reasons.append(([representative], "Shared runtime contains WebGL-specific behavior"))
        selected.fixtures.extend(f"{path}: {reason}" for reason in fixture_reasons)
        for samples, reason in player_reasons:
            for sample in samples:
                selected.players.setdefault(sample, []).append(f"{path}: {reason}")
    return selected


def changed_paths(repository: Path) -> tuple[str, list[str]]:
    """Include committed task changes and the staged/working diff without rename hiding."""
    def git(*args: str) -> str:
        return subprocess.check_output(["git", *args], cwd=repository, text=True).strip()
    revision = git("rev-parse", "HEAD")
    if os.environ.get("TOLLGATE_TESTED_OID") == revision:
        base = git("rev-parse", "HEAD^")
    else:
        base = git("merge-base", "HEAD", "release")
    raw = subprocess.check_output(
        ["git", "diff", "--name-only", "--no-renames", "-z", base, "--"], cwd=repository,
    )
    return revision, [path.decode() for path in raw.split(b"\0") if path]


def validate_affected(repository: Path) -> None:
    """Build and check only selected local browser contracts; retain native checks independently."""
    names = sorted(path.parent.name for path in (repository / "samples").glob("*/sample.toml"))
    revision, paths = changed_paths(repository)
    selected = select(repository, paths, names)
    print("Browser risk selection: " + json.dumps(selected.report(), sort_keys=True), flush=True)
    if not selected:
        return
    # Preparation owns its compiler cache and Unity project lease.
    import importlib.util
    spec = importlib.util.spec_from_file_location("prepare_web_demo", repository / "scripts/prepare-web-demo.py")
    assert spec and spec.loader
    prepare = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(prepare)
    with tempfile.TemporaryDirectory(prefix="battlement-web-contracts.") as temporary:
        site = Path(temporary)
        checked = []
        if selected.fixtures:
            fixture = site / "fixture"
            fixture.mkdir(parents=True)
            (fixture / "index.html").write_text("""<!doctype html><meta charset=utf-8><style>body{margin:0}</style>
<canvas id=unity-canvas width=64 height=64></canvas>
<script src=fixture.framework.js.unityweb></script><script>
const canvas = document.querySelector('canvas');
const context = canvas.getContext('2d'); context.fillStyle = '#123'; context.fillRect(0, 0, 64, 64);
console.log('battlement.host.connected');
</script>""")
            (fixture / "fixture.framework.js.unityweb").write_bytes(
                gzip.compress(b"window.fixtureAssetDecoded = 'ok';")
            )
            checked.append("fixture")
        for sample in selected.players:
            output = prepare.prepare(sample, True, prepare.DEFAULT_CACHE_ROOT)
            shutil.copytree(output, site / sample)
            checked.append(sample)
        evidence = check_site(repository, site, sorted(checked), revision)
        print(f"Affected browser evidence: {evidence}", flush=True)
