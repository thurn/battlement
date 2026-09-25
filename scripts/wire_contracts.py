"""Render wire manifests and their Rust/Unity digest declarations together."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re


def rendered_files(root: Path) -> dict[Path, bytes]:
    """Compute the complete wire fingerprint update without changing source files."""
    wire = root / "contracts/wire-contract.json"
    fixture = root / "crates/battlement-native/tests/fixtures/exported-engine"
    fixture_wire = fixture / "schema/wire-contract.json"
    manifest = json.loads(wire.read_text())
    specification = json.loads((root / "schemas/flatbuffers-toolchain.json").read_text())
    manifest["flatbuffers"]["schema_closure"] = schema_closure(root / "schemas/flatbuffers")
    manifest["flatbuffers"]["version"] = specification["version"]
    manifest["flatbuffers"]["generation_options"] = specification["generation_options"]
    wire_bytes = manifest_bytes(manifest)
    wire_digest = hashlib.sha256(wire_bytes).hexdigest()

    extension = json.loads(fixture_wire.read_text())
    extension["base_wire_contract_digest"] = wire_digest
    extension["schema_closure"] = schema_closure(fixture / "schema")
    extension["generation_options"] = specification["generation_options"]
    fixture_bytes = manifest_bytes(extension)
    fixture_digest = hashlib.sha256(fixture_bytes).hexdigest()
    outputs = {wire: wire_bytes, fixture_wire: fixture_bytes}
    declarations = (
        ("crates/battlement-native/src/lib.rs", "WIRE_CONTRACT_DIGEST", wire_digest),
        ("crates/battlement-native/src/lib.rs", "WIRE_DIGEST_C", wire_digest),
        ("Packages/com.battlement.client/Runtime/Host/Native/BattlementNativeContract.cs",
         "WireContractDigest", wire_digest),
        ("crates/battlement-native/tests/fixtures/exported-engine/src/fixture_response.rs",
         "WIRE_DIGEST_C", fixture_digest),
        ("Packages/com.battlement.client/Tests/Editor/CustomFixtures/FixtureFlatBufferResponseSchema.cs",
         "ContractDigest", fixture_digest),
        ("Assets/BattlementIntegration/FlatBuffers/FixtureFlatBufferResponseSchema.cs",
         "ContractDigest", fixture_digest),
    )
    for relative, name, digest in declarations:
        path = root / relative
        source = outputs[path] if path in outputs else path.read_bytes()
        pattern = rf'(\b{re.escape(name)}\s*(?::[^=]+)?=\s*b?")[0-9a-f]{{64}}(?="|\\0")'
        updated, count = re.subn(pattern, lambda match: match[1] + digest, source.decode())
        if count != 1:
            raise ValueError(f"expected one {name} digest declaration in {relative}, found {count}")
        outputs[path] = updated.encode()
    return outputs


def schema_closure(directory: Path) -> dict[str, str]:
    """Hash each schema in deterministic filename order."""
    return {
        path.name: hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(directory.glob("*.fbs"))
    }


def manifest_bytes(manifest: dict) -> bytes:
    return (json.dumps(manifest, indent=2) + "\n").encode()


def matches(outputs: dict[Path, bytes]) -> bool:
    """Report every stale fingerprint artifact without modifying it."""
    stale = [path for path, expected in outputs.items() if path.read_bytes() != expected]
    for path in stale:
        print(f"Stale wire contract: {path}")
    return not stale
