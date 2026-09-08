#!/usr/bin/env python3

"""Validate the narrow, trusted plan-only lane and publish its selection evidence."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from urllib.parse import unquote, urlsplit
import uuid


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
TRUSTED_PATHS = frozenset({"plans/workflow-performance.md"})
LINK = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")


def changed_paths(repository: Path, *, tollgate: bool = False) -> list[str]:
    """Return the exact candidate diff, or the staged local diff before submission."""
    revision = os.environ.get("TOLLGATE_TESTED_OID") if tollgate else None
    if revision:
        head = _git(repository, "rev-parse", "HEAD").decode().strip()
        if head != revision:
            raise RuntimeError("Tollgate tested revision differs from the execution checkout")
        return _diff_paths(repository, f"{head}^")
    staged = _diff_paths(repository, "--cached")
    if staged:
        return staged
    return _diff_paths(repository)


def selected(paths: list[str]) -> bool:
    """Select only a non-empty change wholly contained by the immutable allowlist."""
    return bool(paths) and all(path in TRUSTED_PATHS for path in paths)


def validate(repository: Path, paths: list[str]) -> None:
    """Validate formatting and local links for an already-selected prose change."""
    if not selected(paths):
        raise RuntimeError(
            "prose validation only accepts: " + ", ".join(sorted(TRUSTED_PATHS))
        )
    errors: list[str] = []
    root = repository.resolve()
    for relative in paths:
        path = repository / relative
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            errors.append(f"{relative}: is not readable UTF-8: {error}")
            continue
        if text and not text.endswith("\n"):
            errors.append(f"{relative}: must end with a newline")
        for line_number, line in enumerate(text.splitlines(), 1):
            if line.rstrip() != line:
                errors.append(f"{relative}:{line_number}: trailing whitespace")
            if line.startswith(("<<<<<<<", "=======", ">>>>>>>")):
                errors.append(f"{relative}:{line_number}: unresolved merge marker")
        for raw_target in LINK.findall(text):
            target = _link_target(raw_target)
            if target is None:
                continue
            destination = (path.parent / target).resolve()
            if destination != root and root not in destination.parents:
                errors.append(f"{relative}: local link escapes the repository: {raw_target}")
            elif not destination.exists():
                errors.append(f"{relative}: local link does not exist: {raw_target}")
    if errors:
        raise RuntimeError("Prose validation failed:\n- " + "\n- ".join(errors))


def publish_evidence(repository: Path, paths: list[str]) -> Path:
    """Publish an exact buildset receipt that explicitly records omitted native checks."""
    identities: dict[str, str] = {}
    for key in ("BUILDSET_ID", "ITEM_ID", "VALIDATION_GENERATION_ID"):
        value = os.environ.get(f"TOLLGATE_{key}", "")
        if str(uuid.UUID(value)) != value:
            raise RuntimeError(f"TOLLGATE_{key} must be a canonical execution identity")
        identities[key] = value
    tested_oid = os.environ.get("TOLLGATE_TESTED_OID", "")
    if not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", tested_oid):
        raise RuntimeError("Missing or invalid Tollgate tested revision")
    root = repository / "artifacts/tollgate-prose" / identities["BUILDSET_ID"]
    for parent in (root.parent.parent, root.parent, root):
        if parent.is_symlink():
            raise RuntimeError("Tollgate prose evidence directory contains a symbolic link")
    root.mkdir(parents=True, mode=0o700)
    destination = root / "evidence.json"
    if destination.exists():
        raise RuntimeError("This buildset already published prose evidence")
    document = {
        "schema": 1,
        "status": "passed",
        "buildset_id": identities["BUILDSET_ID"],
        "candidate_id": identities["ITEM_ID"],
        "validation_generation_id": identities["VALIDATION_GENERATION_ID"],
        "tested_oid": tested_oid,
        "changed_paths": sorted(paths),
        "checks": ["utf-8", "markdown-format", "local-links"],
        "native_checks": {
            "selected": False,
            "reason": "all changed paths matched the trusted plan-only allowlist",
        },
    }
    descriptor, temporary_name = tempfile.mkstemp(prefix=".evidence-", dir=root)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as output:
            json.dump(document, output, indent=2, sort_keys=True)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)
    return destination


def run(repository: Path, *, tollgate_evidence: bool = False) -> list[str]:
    paths = changed_paths(repository, tollgate=tollgate_evidence)
    validate(repository, paths)
    if tollgate_evidence:
        publish_evidence(repository, paths)
    print("Trusted prose validation passed: " + ", ".join(paths), flush=True)
    return paths


def _diff_paths(repository: Path, base: str | None = None) -> list[str]:
    arguments = ["diff", "--name-only", "-z"]
    if base == "--cached":
        arguments.append(base)
    elif base is not None:
        arguments.extend([base, "HEAD"])
    output = _git(repository, *arguments)
    return sorted(value.decode() for value in output.split(b"\0") if value)


def _git(repository: Path, *arguments: str) -> bytes:
    return subprocess.run(
        ["git", *arguments], cwd=repository, check=True, capture_output=True
    ).stdout


def _link_target(raw: str) -> Path | None:
    value = raw.strip()
    if value.startswith("<") and ">" in value:
        value = value[1 : value.index(">")]
    else:
        value = value.split(maxsplit=1)[0]
    parsed = urlsplit(value)
    if parsed.scheme or parsed.netloc or not parsed.path:
        return None
    return Path(unquote(parsed.path))


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tollgate-evidence", action="store_true")
    return parser.parse_args()


if __name__ == "__main__":
    arguments = parse_arguments()
    try:
        run(REPOSITORY_ROOT, tollgate_evidence=arguments.tollgate_evidence)
    except (OSError, RuntimeError, subprocess.CalledProcessError, ValueError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
