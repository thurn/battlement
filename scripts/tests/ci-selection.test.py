#!/usr/bin/env python3

"""Verify dependency-scoped Rust and Reactant validation decisions."""

from pathlib import Path
import sys
import tempfile
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import ci_selection


workspaces = [
    Path("samples/basic/rules/Cargo.toml"),
    Path("samples/chess/rules/Cargo.toml"),
    Path("samples/reactant/rules/Cargo.toml"),
]
dependencies = {
    "basic": {"battlement"},
    "chess": {"battlement", "reactant", "reactant-core", "reactant-ui"},
    "reactant": {"battlement", "reactant", "reactant-core", "reactant-ui"},
}

with patch.object(
    ci_selection.native_validation_selection,
    "sample_crates",
    side_effect=lambda _root, sample: dependencies[sample],
):
    all_workspaces = ci_selection.select_rust(ROOT, [], workspaces)
    assert all_workspaces.root and list(all_workspaces.samples) == workspaces

    chess_ui = ci_selection.select_rust(
        ROOT, ["samples/chess/rules/src/lib.rs"], workspaces
    )
    assert not chess_ui.root
    assert chess_ui.samples == (workspaces[1],)

    reactant = ci_selection.select_rust(
        ROOT, ["crates/reactant-core/src/lib.rs"], workspaces
    )
    assert reactant.root
    assert reactant.samples == (workspaces[1], workspaces[2])

    unrelated = ci_selection.select_rust(
        ROOT, ["scripts/perf_report.py", "README.md"], workspaces
    )
    assert not unrelated.root and not unrelated.samples

    for dependency in ci_selection.TOOLING_RUST_INPUTS:
        tooling = ci_selection.select_rust(ROOT, [dependency], workspaces)
        assert tooling.root and not tooling.samples
        assert "battlement-tooling" in tooling.packages

    global_change = ci_selection.select_rust(ROOT, ["Cargo.lock"], workspaces)
    assert global_change.root and list(global_change.samples) == workspaces

selected, _reasons = ci_selection.select_reactant_assets(
    ["crates/battlement-reactant-assets/src/lib.rs"]
)
assert selected
selected, reasons = ci_selection.select_reactant_assets(
    ["samples/chess/rules/src/lib.rs"]
)
assert not selected and "no changed path" in reasons[0]
selected, _reasons = ci_selection.select_reactant_assets([])
assert selected

with tempfile.TemporaryDirectory() as directory:
    repository = Path(directory)
    (repository / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["crates/*"]\nresolver = "2"\n'
    )
    manifests = {
        "rules": "",
        "display": '[dependencies]\nrules = { path = "../rules" }\n',
        "app": '[dev-dependencies]\ndisplay = { path = "../display" }\n',
        "builder": '[build-dependencies]\nrules = { path = "../rules" }\n',
        "optional": '[dependencies]\nrules = { path = "../rules", optional = true }\n',
        "targeted": '[target.\'cfg(target_os = "windows")\'.dependencies]\nrules = { path = "../rules" }\n',
        "unrelated": "",
    }
    for name, dependencies in manifests.items():
        crate = repository / "crates" / name
        (crate / "src").mkdir(parents=True)
        (crate / "src/lib.rs").write_text("")
        (crate / "Cargo.toml").write_text(
            f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n'
            + dependencies
        )
    selection = ci_selection.select_rust(repository, ["crates/rules/src/lib.rs"], [])
    assert selection.packages == ("app", "builder", "display", "optional", "rules", "targeted")
    assert ci_selection.root_arguments(selection) == [
        argument for package in selection.packages for argument in ("-p", package)
    ]
    assert ci_selection.select_rust(repository, ["crates/unrelated/src/lib.rs"], []).packages == ("unrelated",)
    unknown = ci_selection.select_rust(repository, ["crates/deleted/src/lib.rs"], [])
    assert ci_selection.root_arguments(unknown) == ["--workspace"]
    assert ci_selection.select_rust(repository, ["Cargo.lock", "crates/rules/src/lib.rs"], []).packages is None

print("CI selection tests passed")
