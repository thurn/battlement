#!/usr/bin/env python3

"""Verify Web demo cache identity and materialization helpers."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))
SPEC = importlib.util.spec_from_file_location(
    "prepare_web_demo",
    REPOSITORY_ROOT / "scripts/prepare-web-demo.py",
)
assert SPEC and SPEC.loader
prepare_web_demo = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(prepare_web_demo)


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-web-demo-test.") as temporary:
        root = Path(temporary)
        source = root / "source"
        destination = root / "destination"
        (source / "Build").mkdir(parents=True)
        (source / "index.html").write_text("fixture\n")
        (source / "Build/sample.wasm.unityweb").write_bytes(b"wasm")

        assert prepare_web_demo.valid_web_build(source)
        prepare_web_demo.materialize_directory(source, destination)
        assert prepare_web_demo.valid_web_build(destination)

        (source / "index.html").write_text("updated\n")
        prepare_web_demo.materialize_directory(source, destination)
        assert (destination / "index.html").read_text() == "updated\n"

        command = prepare_web_demo.build_command("tictactoe", True, True)
        assert command[-3:] == ["--web", "--web-unthreaded", "--release"]

        repository = root / "repository"
        create_repository(repository)
        original_root = prepare_web_demo.REPOSITORY_ROOT
        original_editor = prepare_web_demo.unity_editor
        try:
            prepare_web_demo.REPOSITORY_ROOT = repository
            prepare_web_demo.unity_editor = lambda _sample: Path("/bin/sh")
            initial = prepare_web_demo.staged_fingerprint("fixture", False, False)
            tracked = repository / "samples/fixture/rules/src/lib.rs"
            tracked.write_text("pub fn value() -> u8 { 2 }\n")
            subprocess.run(["git", "add", str(tracked)], cwd=repository, check=True)
            changed = prepare_web_demo.staged_fingerprint("fixture", False, False)
        finally:
            prepare_web_demo.REPOSITORY_ROOT = original_root
            prepare_web_demo.unity_editor = original_editor
        assert initial != changed

    print("Web demo cache tests passed.")


def create_repository(root: Path) -> None:
    for directory in (
        "Packages/com.battlement.client",
        "crates/example",
        "samples/fixture/ProjectSettings",
        "samples/fixture/rules/src",
        "web",
    ):
        (root / directory).mkdir(parents=True, exist_ok=True)
    for path, contents in (
        ("Cargo.toml", "[workspace]\nmembers = []\n"),
        ("Cargo.lock", ""),
        ("rust-toolchain.toml", "[toolchain]\nchannel = \"stable\"\n"),
        ("Packages/com.battlement.client/package.json", "{}\n"),
        ("crates/example/lib.rs", ""),
        ("web/init.js", ""),
        ("samples/fixture/sample.toml", "application = \"Fixture.app\"\n"),
        ("samples/fixture/ProjectSettings/ProjectVersion.txt", "m_EditorVersion: fixture\n"),
        ("samples/fixture/rules/src/lib.rs", "pub fn value() -> u8 { 1 }\n"),
    ):
        (root / path).write_text(contents)
    subprocess.run(["git", "init", "--quiet"], cwd=root, check=True)
    subprocess.run(["git", "add", "."], cwd=root, check=True)
    subprocess.run(
        [
            "git", "-c", "user.name=CI Fixture", "-c", "user.email=ci@example.invalid",
            "commit", "--quiet", "-m", "fixture",
        ],
        cwd=root,
        check=True,
    )


if __name__ == "__main__":
    main()
