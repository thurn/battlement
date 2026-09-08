"""Export only one verified Ditto invocation for Tollgate's exact buildset collector."""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import io
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tarfile
import uuid

import ditto_evidence


@dataclass(frozen=True)
class Export:
    """Collector identity supplied by Tollgate, captured before CI starts."""

    root: Path
    buildset_id: str
    candidate_id: str
    generation_id: str
    tested_oid: str

    @classmethod
    def begin(cls, repository: Path) -> Export:
        values = {}
        for key in ("BUILDSET_ID", "ITEM_ID", "VALIDATION_GENERATION_ID"):
            value = os.environ.get(f"TOLLGATE_{key}", "")
            if str(uuid.UUID(value)) != value:
                raise ValueError(f"TOLLGATE_{key} must be a canonical execution identity")
            values[key] = value
        tested_oid = os.environ.get("TOLLGATE_TESTED_OID", "")
        if not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", tested_oid):
            raise ValueError("Missing or invalid Tollgate tested revision")
        head = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=repository, text=True,
        ).strip()
        if head != tested_oid:
            raise ValueError("Tollgate tested revision differs from the execution checkout")
        root = repository / "artifacts/tollgate-ditto" / values["BUILDSET_ID"]
        for parent in (root.parent.parent, root.parent, root):
            if parent.is_symlink():
                raise ValueError("Tollgate evidence directory contains a symbolic link")
        root.mkdir(parents=True, mode=0o700)
        return cls(root, values["BUILDSET_ID"], values["ITEM_ID"], values["VALIDATION_GENERATION_ID"], tested_oid)

    def publish(self, manifest: Path, invocation_id: str) -> Path:
        """Seal exactly the manifest's members; unrelated or stale executions are never scanned."""
        document = ditto_evidence.read(manifest, invocation_id, expected_tested_oid=self.tested_oid)
        for field, expected in (
            ("candidate_id", self.candidate_id),
            ("buildset_id", self.buildset_id),
            ("validation_generation_id", self.generation_id),
        ):
            if document.get(field) != expected:
                raise ValueError(f"Ditto evidence belongs to another Tollgate {field}")
        if document["status"] not in {"passed", "failed", "canceled"}:
            raise ValueError("Ditto evidence has no terminal outcome")
        if document["command"] != "gate":
            raise ValueError("Tollgate requires the canonical Ditto gate invocation")
        manifest_bytes = manifest.read_bytes()
        if json.loads(manifest_bytes) != document:
            raise ValueError("Ditto manifest changed during collection")
        receipt = {
            "schema": 1, "buildset_id": self.buildset_id,
            "candidate_id": self.candidate_id, "validation_generation_id": self.generation_id,
            "tested_oid": self.tested_oid, "invocation_id": invocation_id,
            "manifest_sha256": hashlib.sha256(manifest_bytes).hexdigest(),
            "status": document["status"],
        }
        destination = self.root / "evidence.tar.gz"
        if destination.exists():
            raise ValueError("This buildset already published its Ditto evidence")
        temporary = self.root / f".evidence-{uuid.uuid4()}.tmp"
        try:
            with tarfile.open(temporary, "w:gz") as archive:
                _add_bytes(archive, "receipt.json", json.dumps(receipt, sort_keys=True).encode())
                _add_bytes(archive, "invocation/evidence.json", manifest_bytes)
                for member in document["files"]:
                    path = ditto_evidence._owned_file(manifest.parent, member["path"])
                    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
                    with os.fdopen(descriptor, "rb") as source:
                        if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
                            raise ValueError("Evidence member changed its file type")
                        verified = _VerifiedReader(source)
                        info = tarfile.TarInfo("invocation/" + member["path"])
                        info.size = member["bytes"]
                        info.mode = 0o644
                        archive.addfile(info, verified)
                        if verified.digest.hexdigest() != member["sha256"]:
                            raise ValueError(f"Evidence changed during archival: {member['path']}")
            temporary.replace(destination)
        finally:
            temporary.unlink(missing_ok=True)
        return destination


def _add_bytes(archive: tarfile.TarFile, name: str, value: bytes) -> None:
    info = tarfile.TarInfo(name)
    info.size = len(value)
    info.mode = 0o644
    archive.addfile(info, io.BytesIO(value))


class _VerifiedReader:
    def __init__(self, source) -> None:
        self.source = source
        self.digest = hashlib.sha256()

    def read(self, size: int) -> bytes:
        value = self.source.read(size)
        self.digest.update(value)
        return value
