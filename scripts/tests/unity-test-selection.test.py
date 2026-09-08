#!/usr/bin/env python3

"""Verify conservative Unity Edit Mode affected-test selection."""

from pathlib import Path
import sys


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

from unity_test_selection import (  # noqa: E402
    FULL_ASSEMBLIES,
    INTEGRATION_ASSEMBLIES,
    NATIVE_ASSEMBLIES,
    Scope,
    fixture_dependency_roots,
    select,
)


def main() -> None:
    roots = fixture_dependency_roots(REPOSITORY_ROOT)
    for root in (
        "crates/battlement",
        "crates/battlement-cloud",
        "crates/battlement-native",
        "crates/battlement-types",
        "crates/battlement-ui",
    ):
        assert root in roots
    for root in (
        "crates/battlement-cli",
        "crates/battlement-ditto",
        "crates/battlement-reactant",
        "crates/battlement-tooling",
    ):
        assert root not in roots

    skipped = select(
        REPOSITORY_ROOT,
        [
            "docs/guide.md",
            "samples/chess-ui/rules/src/lib.rs",
            "crates/battlement-reactant/src/lib.rs",
            "crates/battlement-ditto/src/lib.rs",
        ],
    )
    assert skipped.scope == Scope.NONE
    assert skipped.assemblies == ()
    assert not skipped.dotnet_diagnostics
    assert "crates" not in skipped.cache_inputs

    for path in (
        "Cargo.lock",
        "rust-toolchain.toml",
        "crates/battlement/src/lib.rs",
        "crates/battlement-native/src/lib.rs",
        "crates/battlement-ui/src/lib.rs",
    ):
        native = select(REPOSITORY_ROOT, [path])
        assert native.scope == Scope.NATIVE, path
        assert native.assemblies == NATIVE_ASSEMBLIES
        assert not native.dotnet_diagnostics

    for path in (
        "Assets/BattlementIntegration/IntegrationFixturePayload.cs",
        "Packages/com.battlement.client/Runtime/Json/BattlementJson.cs",
        "ProjectSettings/ProjectVersion.txt",
        "scripts/ci.py",
        "scripts/unity_test_selection.py",
        "scripts/web_selection.py",
    ):
        full = select(REPOSITORY_ROOT, [path])
        assert full.scope == Scope.ALL, path
        assert full.assemblies == FULL_ASSEMBLIES
        assert full.dotnet_diagnostics == (path != "scripts/web_selection.py")

    for path in (
        ".config/dotnet-tools.json",
        ".editorconfig",
        "Directory.Build.targets",
        "battlement-ci.slnx",
    ):
        diagnostics = select(REPOSITORY_ROOT, [path])
        assert diagnostics.scope == Scope.NONE, path
        assert diagnostics.dotnet_diagnostics, path
    for path in (
        "scripts/ci_cache.py",
        "scripts/perf_log.py",
        "scripts/resource_slots.py",
    ):
        assert select(REPOSITORY_ROOT, [path]).dotnet_diagnostics, path

    integration = select(
        REPOSITORY_ROOT,
        ["samples/ui/Assets/Original/Signal Texture.png"],
    )
    assert integration.scope == Scope.INTEGRATION
    assert integration.assemblies == INTEGRATION_ASSEMBLIES
    assert not integration.dotnet_diagnostics

    mixed = select(
        REPOSITORY_ROOT,
        ["crates/battlement-ui/src/lib.rs", "Packages/manifest.json"],
    )
    assert mixed.scope == Scope.ALL
    assert select(REPOSITORY_ROOT, []).scope == Scope.ALL
    print("Unity affected-test selection passed")


if __name__ == "__main__":
    main()
