"""Publish and verify invocation-owned Ditto evidence without latest-report aliases."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
import uuid


def invocation_root(repository: Path, invocation_id: str) -> Path:
    """Resolve the exact invocation directory, including an explicit artifact root."""
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]{0,127}", invocation_id):
        raise ValueError("Invalid Ditto invocation ID")
    root = Path(os.environ.get(
        "DITTO_CI_ARTIFACT_ROOT", repository / "artifacts/ditto-ci/executions" / invocation_id,
    )).absolute()
    if root.name != invocation_id:
        raise ValueError("DITTO_CI_ARTIFACT_ROOT must end with the invocation ID")
    return root


def _git(repository: Path, *arguments: str) -> str | None:
    result = subprocess.run(["git", "--no-optional-locks", *arguments], cwd=repository, text=True, capture_output=True)
    return result.stdout.strip() if result.returncode == 0 else None


def _index_digest(repository: Path) -> str | None:
    entries = _git(repository, "ls-files", "--stage", "-z")
    return hashlib.sha256(entries.encode()).hexdigest() if entries is not None else None


def begin(root: Path, invocation_id: str, repository: Path, command: str) -> dict:
    """Claim one invocation before any producer writes its artifacts."""
    root.mkdir(parents=True)
    head = _git(repository, "rev-parse", "HEAD")
    clean = _git(repository, "status", "--porcelain", "--untracked-files=no") == ""
    identity = {
        "schema": 1,
        "invocation_id": invocation_id,
        "artifact_root": str(root),
        "command": command,
        "started_at": time.time(),
        "head_oid": head,
        "tested_oid": head if clean else None,
        "source_index_digest": _index_digest(repository),
        "task_id": os.environ.get("CODEX_THREAD_ID"),
        "candidate_id": os.environ.get("TOLLGATE_ITEM_ID"),
        "validation_generation_id": os.environ.get("TOLLGATE_VALIDATION_GENERATION_ID"),
    }
    # Exclusive creation rejects duplicate invocation ownership, including retries.
    with (root / "invocation.json").open("x", encoding="utf-8") as output:
        json.dump(identity, output, sort_keys=True)
        output.write("\n")
    return identity


def digest(path: Path) -> str:
    """Hash a retained file without loading archives into memory."""
    checksum = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            checksum.update(chunk)
    return checksum.hexdigest()


def _owned_file(root: Path, relative: str) -> Path:
    path = Path(relative)
    if path.is_absolute() or ".." in path.parts:
        raise ValueError(f"Evidence path escapes invocation: {relative}")
    current = root
    for part in path.parts:
        current = current / part
        if current.is_symlink():
            raise ValueError(f"Evidence path is a symbolic link: {relative}")
    if not current.is_file():
        raise ValueError(f"Missing evidence: {relative}")
    return current


def finish(root: Path, identity: dict, status: str) -> Path:
    """Atomically publish a terminal inventory of this invocation's retained files."""
    files = []
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"Evidence path is a symbolic link: {path}")
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix()
        if relative == "evidence.json":
            raise ValueError("This invocation already has terminal evidence")
        files.append({"path": relative, "sha256": digest(path), "bytes": path.stat().st_size})
    document = {**identity, "finished_at": time.time(), "status": status, "files": files}
    temporary = root / f".evidence-{uuid.uuid4()}.tmp"
    temporary.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n")
    temporary.replace(root / "evidence.json")
    return root / "evidence.json"


def read(path: Path, expected_invocation: str, *, expected_tested_oid: str | None = None) -> dict:
    """Verify exact invocation identity and every retained member before discovery."""
    if path.name != "evidence.json":
        raise ValueError("Select an invocation evidence.json, not a latest-report alias")
    document = json.loads(path.read_text())
    if document.get("schema") != 1 or document.get("invocation_id") != expected_invocation:
        raise ValueError("Evidence invocation identity mismatch")
    if expected_tested_oid is not None and document.get("tested_oid") != expected_tested_oid:
        raise ValueError("Evidence tested revision mismatch")
    names = set()
    for member in document["files"]:
        relative = member["path"]
        if relative in names:
            raise ValueError(f"Duplicate evidence member: {relative}")
        names.add(relative)
        owned = _owned_file(path.parent, relative)
        if owned.stat().st_size != member["bytes"] or digest(owned) != member["sha256"]:
            raise ValueError(f"Evidence content changed: {relative}")
    if "invocation.json" not in names:
        raise ValueError("Evidence omitted its invocation identity")
    identity = json.loads((path.parent / "invocation.json").read_text())
    for field in ("invocation_id", "artifact_root", "tested_oid", "source_index_digest", "command"):
        if identity.get(field) != document.get(field):
            raise ValueError(f"Evidence identity differs from its invocation: {field}")
    if document["command"] == "gate":
        if "gate.json" not in names:
            raise ValueError("Missing evidence: gate.json")
        gate = json.loads((path.parent / "gate.json").read_text())
        if gate.get("invocation_id") != expected_invocation:
            raise ValueError("Gate report belongs to another invocation")
        if gate.get("artifact_root") != document["artifact_root"]:
            raise ValueError("Gate report artifact root mismatch")
        if document["status"] == "passed":
            if gate["status"] != "passed" or gate["failures"]:
                raise ValueError("Passed evidence has a failed gate report")
            expected = set(gate["expected_samples"])
            observed = [sample["sample"] for sample in gate["samples"]]
            if set(observed) != expected or len(observed) != len(expected):
                raise ValueError("Gate omitted expected sample evidence")
        for sample in gate["samples"]:
            if f"{sample['sample']}/result.json" not in names:
                raise ValueError(f"Missing sample evidence: {sample['sample']}")
    return document


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path, help="Exact invocation evidence.json")
    parser.add_argument("--invocation", required=True, help="Expected invocation ID")
    parser.add_argument("--tested-oid", help="Expected tested revision")
    args = parser.parse_args()
    print(json.dumps(read(args.manifest, args.invocation, expected_tested_oid=args.tested_oid), indent=2))


if __name__ == "__main__":
    main()
