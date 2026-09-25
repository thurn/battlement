"""Exercise root test partitioning through staged inputs and the real CI cache."""

from contextlib import nullcontext
from pathlib import Path
import subprocess
from unittest.mock import patch

from ci_cache import CiCache
from ci_selection import RustSelection


def verify_root_rust_cache(ci, root: Path) -> None:
    repository = root / "rust-cache-repository"
    repository.mkdir()
    subprocess.run(["git", "init", "--quiet"], cwd=repository, check=True)
    dependencies = (
        "Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
        "crates/reactant-core/src/audio.rs",
        "crates/reactant-core/tests/fixtures/data.json",
        "samples/reactant/rules/src/lifetime_proof.rs",
        "samples/chess/rules/Cargo.toml",
        "samples/ui/Assets/Original/Signal Sprite.png",
        "samples/chess/rules/tests/fixtures/ditto.lock",
        *ci.ci_selection.TOOLING_RUST_INPUTS,
    )
    baseline = "samples/chess/ditto.lock"
    for relative in (*dependencies, baseline):
        path = repository / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("initial\n")
    subprocess.run(["git", "add", "."], cwd=repository, check=True)
    cache = CiCache(repository, root / "rust-cache", {"toolchain": "fixture"})
    calls = []
    selection = RustSelection(root=True, samples=(), reasons=("fixture",))
    commands = [
        ["cargo", "test", "--workspace", "--exclude", "battlement-ditto"],
        ["cargo", "test", "--package", "battlement-ditto"],
    ]

    def observe():
        calls.clear()
        ci.test_rust_workspaces(selection, cache)
        return calls.copy()

    def change(relative):
        path = repository / relative
        path.write_text(path.read_text() + "changed\n")
        subprocess.run(["git", "add", relative], cwd=repository, check=True)

    with (
        patch.object(ci, "REPOSITORY_ROOT", repository),
        patch.object(ci, "cargo_environment", return_value={}),
        patch.object(ci.process_priority, "run", side_effect=lambda args, **_: calls.append(args)),
        patch.object(cache, "invocation", side_effect=nullcontext),
    ):
        assert observe() == commands
        assert observe() == []
        change(baseline)
        assert observe() == commands[1:]
        assert observe() == []
        for relative in dependencies:
            change(relative)
            assert observe() == commands, relative
            assert observe() == [], relative
        # A dirty baseline bypasses only the tests that read it at runtime.
        (repository / baseline).write_text("unstaged baseline\n")
        assert observe() == commands[1:]
        assert observe() == commands[1:]
        (repository / baseline).unlink()
        subprocess.run(["git", "add", baseline], cwd=repository, check=True)
        assert observe() == commands[1:]
        assert observe() == []
