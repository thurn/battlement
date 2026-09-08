"""Select declared browser risks without making public demos a validation dependency."""

from __future__ import annotations

from fnmatch import fnmatchcase
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

from web_compatibility import check_site


def select(repository: Path, paths: list[str], names: list[str]) -> dict[str, list[str]]:
    """Map changed paths to browser contracts and concrete platform risks."""
    risks = tomllib.loads((repository / "web/contracts.toml").read_text())["risks"]
    selected: dict[str, list[str]] = {}
    for path in paths:
        reasons = []
        if path in {"web/contracts.toml", "web/checks/shared.js", "scripts/web_selection.py", "scripts/web_compatibility.py", "scripts/playwright_mcp.py"}:
            reasons.append((names, "Browser contract selection or execution"))
        for sample in names:
            if path == f"web/checks/{sample}.js":
                reasons.append(([sample], "Declared sample interaction"))
        for risk in risks:
            if any(fnmatchcase(path, pattern) for pattern in risk["paths"]):
                samples = names if risk["samples"] == ["*"] else risk["samples"]
                if not set(samples).issubset(names) or not risk["reason"].strip():
                    raise RuntimeError("Browser risk declares an unknown sample or empty reason")
                reasons.append((samples, risk["reason"]))
        source = repository / path
        if path.startswith("Packages/com.battlement.client/Runtime/") and path.endswith(".cs"):
            if not source.exists() or "UNITY_WEBGL" in source.read_text():
                reasons.append((names, "Shared runtime contains WebGL-specific behavior"))
        for samples, reason in reasons:
            for sample in samples:
                selected.setdefault(sample, []).append(f"{path}: {reason}")
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
    print("Browser risk selection: " + json.dumps(selected, sort_keys=True), flush=True)
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
        for sample in selected:
            output = prepare.prepare(sample, True, prepare.DEFAULT_CACHE_ROOT)
            shutil.copytree(output, site / sample)
        evidence = check_site(repository, site, sorted(selected), revision)
        print(f"Affected browser evidence: {evidence}", flush=True)
