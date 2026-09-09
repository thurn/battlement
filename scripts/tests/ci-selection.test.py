#!/usr/bin/env python3

"""Verify dependency-scoped Rust and Reactant validation decisions."""

from pathlib import Path
import sys
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import ci_selection


workspaces = [
    Path("samples/basic/rules/Cargo.toml"),
    Path("samples/chess-ui/rules/Cargo.toml"),
    Path("samples/reactant/rules/Cargo.toml"),
]
dependencies = {
    "basic": {"battlement"},
    "chess-ui": {"battlement", "battlement-reactant"},
    "reactant": {"battlement", "battlement-reactant"},
}

with patch.object(
    ci_selection.native_validation_selection,
    "sample_crates",
    side_effect=lambda _root, sample: dependencies[sample],
):
    all_workspaces = ci_selection.select_rust(ROOT, [], workspaces)
    assert all_workspaces.root and list(all_workspaces.samples) == workspaces

    chess_ui = ci_selection.select_rust(
        ROOT, ["samples/chess-ui/rules/src/lib.rs"], workspaces
    )
    assert not chess_ui.root
    assert chess_ui.samples == (workspaces[1],)

    reactant = ci_selection.select_rust(
        ROOT, ["crates/battlement-reactant/src/lib.rs"], workspaces
    )
    assert reactant.root
    assert reactant.samples == (workspaces[1], workspaces[2])

    unrelated = ci_selection.select_rust(
        ROOT, ["scripts/perf_report.py", "README.md"], workspaces
    )
    assert not unrelated.root and not unrelated.samples

    global_change = ci_selection.select_rust(ROOT, ["Cargo.lock"], workspaces)
    assert global_change.root and list(global_change.samples) == workspaces

selected, _reasons = ci_selection.select_reactant_assets(
    ["crates/battlement-reactant-assets/src/lib.rs"]
)
assert selected
selected, reasons = ci_selection.select_reactant_assets(
    ["samples/chess-ui/rules/src/lib.rs"]
)
assert not selected and "no changed path" in reasons[0]
selected, _reasons = ci_selection.select_reactant_assets([])
assert selected

print("CI selection tests passed")
