#!/usr/bin/env python3

"""Verify trusted prose selection, validation, and Tollgate evidence."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from unittest.mock import patch


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))
import prose_validation


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-prose-validation.") as temporary:
        root = Path(temporary)
        _git(root, "init", "-q")
        _git(root, "config", "user.name", "Prose Test")
        _git(root, "config", "user.email", "prose@example.invalid")
        plan = root / "plans/workflow-performance.md"
        reference = root / "plans/reference.md"
        plan.parent.mkdir()
        plan.write_text("# Workflow\n\n[Reference](reference.md)\n", encoding="utf-8")
        reference.write_text("# Reference\n", encoding="utf-8")
        _git(root, "add", ".")
        _git(root, "commit", "-qm", "base")

        plan.write_text("# Workflow\n\n[Reference](reference.md)\n\nNext.\n", encoding="utf-8")
        _git(root, "add", str(plan.relative_to(root)))
        paths = prose_validation.changed_paths(root)
        assert paths == ["plans/workflow-performance.md"]
        assert prose_validation.selected(paths)
        prose_validation.validate(root, paths)
        assert not prose_validation.selected(paths + ["AGENTS.md"])

        plan.write_text("# Workflow\n\n[Missing](missing.md)\n", encoding="utf-8")
        try:
            prose_validation.validate(root, paths)
        except RuntimeError as error:
            assert "local link does not exist" in str(error)
        else:
            raise AssertionError("missing local link was accepted")

        plan.write_text("# Workflow\n\n![Missing image](missing.png)\n", encoding="utf-8")
        try:
            prose_validation.validate(root, paths)
        except RuntimeError as error:
            assert "local link does not exist" in str(error)
        else:
            raise AssertionError("missing local image was accepted")

        outside = root.parent / "outside-plan.md"
        outside.write_text("# Outside\n", encoding="utf-8")
        plan.unlink()
        plan.symlink_to(outside)
        try:
            prose_validation.validate(root, paths)
        except RuntimeError as error:
            assert "not a symbolic link" in str(error)
        else:
            raise AssertionError("symbolic-link plan was accepted")

        plan.unlink()
        plan.write_text("# Workflow\n\nEvidence.\n", encoding="utf-8")
        _git(root, "add", str(plan.relative_to(root)))
        _git(root, "commit", "-qm", "plan")
        tested_oid = _git(root, "rev-parse", "HEAD").strip()
        environment = {
            "TOLLGATE_BUILDSET_ID": "019ffe40-a60d-7722-a369-2635222d1203",
            "TOLLGATE_ITEM_ID": "019ffe40-a60d-7722-a369-2635222d1204",
            "TOLLGATE_VALIDATION_GENERATION_ID": "019ffe40-a60d-7722-a369-2635222d1205",
            "TOLLGATE_TESTED_OID": tested_oid,
        }
        with patch.dict(os.environ, environment, clear=False):
            assert prose_validation.changed_paths(root, tollgate=True) == paths
            evidence = prose_validation.publish_evidence(root, paths)
        document = json.loads(evidence.read_text(encoding="utf-8"))
        assert document["changed_paths"] == paths
        assert document["tested_oid"] == tested_oid
        assert document["native_checks"]["selected"] is False
    print("Trusted prose validation tests passed.")


def _git(root: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", *arguments],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout


if __name__ == "__main__":
    main()
