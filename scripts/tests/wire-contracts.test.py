#!/usr/bin/env python3

"""Exercise schema edits, stale declarations, and idempotent wire refreshes."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import wire_contracts  # noqa: E402


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-wire-contracts-") as temporary:
        root = Path(temporary)
        fixture = Path("crates/battlement-native/tests/fixtures/exported-engine")
        sources = [
            *wire_contracts.rendered_files(ROOT),
            *ROOT.joinpath("schemas/flatbuffers").glob("*.fbs"),
            ROOT / "schemas/flatbuffers-toolchain.json",
            ROOT / fixture / "schema/fixture_response.fbs",
        ]
        for source in sources:
            destination = root / source.relative_to(ROOT)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)
        assert wire_contracts.matches(wire_contracts.rendered_files(root))
        for relative in ("schemas/flatbuffers/common.fbs", fixture / "schema/fixture_response.fbs"):
            schema = root / relative
            schema.write_text(schema.read_text() + "\n// fingerprint probe\n")
        before = {path: path.read_bytes() for path in root.rglob("*") if path.is_file()}
        rendered = wire_contracts.rendered_files(root)
        assert not wire_contracts.matches(rendered)
        assert all(path.read_bytes() == contents for path, contents in before.items())
        for path, contents in rendered.items():
            path.write_bytes(contents)
        assert wire_contracts.matches(wire_contracts.rendered_files(root))
        assert wire_contracts.rendered_files(root) == rendered
        wire_digest = hashlib.sha256((root / "contracts/wire-contract.json").read_bytes()).hexdigest()
        extension = root / fixture / "schema/wire-contract.json"
        fixture_digest = hashlib.sha256(extension.read_bytes()).hexdigest()
        assert json.loads(extension.read_bytes())["base_wire_contract_digest"] == wire_digest
        native = root / "crates/battlement-native/src/lib.rs"
        assert native.read_text().count(wire_digest) == 2
        assert fixture_digest in (root / fixture / "src/fixture_response.rs").read_text()
        csharp = root / "Packages/com.battlement.client/Runtime/Host/Native/BattlementNativeContract.cs"
        csharp.write_text(csharp.read_text().replace(wire_digest, "0" * 64))
        assert not wire_contracts.matches(wire_contracts.rendered_files(root))
        native.write_text(native.read_text().replace("WIRE_DIGEST_C", "RENAMED_DIGEST"))
        try:
            wire_contracts.rendered_files(root)
        except ValueError as error:
            assert "WIRE_DIGEST_C" in str(error)
        else:
            raise AssertionError("a missing digest declaration must not silently skip refresh")
    print("Wire contract refresh tests passed.")


if __name__ == "__main__":
    main()
