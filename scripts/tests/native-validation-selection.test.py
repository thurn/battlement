#!/usr/bin/env python3

"""Exercise native sample selection without invoking build tools."""

from pathlib import Path
import sys
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import native_validation_selection as selection


SAMPLES = ["basic", "reactant", "chess-ui"]


assert selection.select(ROOT, ["README.md"], SAMPLES) == []
assert selection.select(ROOT, ["samples/basic/rules/src/lib.rs"], SAMPLES) == ["basic"]
assert selection.select(ROOT, ["Packages/com.battlement.client/Runtime/Host.cs"], SAMPLES) == SAMPLES
assert selection.select(ROOT, ["contracts/native-abi.json"], SAMPLES) == SAMPLES
assert selection.select(ROOT, ["ProjectSettings/GraphicsSettings.asset"], SAMPLES) == SAMPLES
assert selection.select(ROOT, ["samples/basic/ProjectSettings/ProjectSettings.asset"], SAMPLES) == SAMPLES
assert selection.select(ROOT, ["samples/chess/Assets/Shaders/LegalSquare.shader"], SAMPLES) == SAMPLES
assert selection.select(ROOT, ["samples/ui/Assets/Resources/BattlementTextSettings.asset"], SAMPLES) == SAMPLES

dependencies = {
    "basic": {"battlement"},
    "reactant": {"battlement", "reactant", "reactant-core", "reactant-ui"},
    "chess-ui": {"battlement", "reactant", "reactant-core", "reactant-ui"},
}
with patch.object(selection, "sample_crates", side_effect=lambda _root, sample: dependencies[sample]):
    assert selection.select(ROOT, ["crates/reactant-core/src/lib.rs"], SAMPLES) == [
        "reactant",
        "chess-ui",
    ]

print("native validation selection tests passed")
