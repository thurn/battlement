#!/usr/bin/env python3

"""Verify content-addressed CI Cache reuse."""

from __future__ import annotations

from pathlib import Path
import subprocess
import sys
import tempfile


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

from ci_cache import CiCache  # noqa: E402


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-ci-cache-test.") as temporary:
        root = Path(temporary)
        repository = root / "repository"
        cache_root = root / "cache"
        repository.mkdir()
        subprocess.run(["git", "init", "--quiet"], cwd=repository, check=True)
        included = repository / "included.txt"
        unrelated = repository / "unrelated.txt"
        included.write_text("initial\n")
        unrelated.write_text("initial\n")
        subprocess.run(["git", "add", "."], cwd=repository, check=True)
        subprocess.run(
            [
                "git", "-c", "user.name=CI Fixture", "-c", "user.email=ci@example.invalid",
                "commit", "--quiet", "-m", "fixture",
            ],
            cwd=repository,
            check=True,
        )
        cache = CiCache(repository, cache_root, {"toolchain": "fixture"})
        calls: list[str] = []

        assert cache.run("fixture", ("included.txt",), lambda: calls.append("first"))
        assert not cache.run("fixture", ("included.txt",), lambda: calls.append("cached"))
        assert calls == ["first"]

        replica = root / "replica"
        subprocess.run(["git", "clone", "--quiet", str(repository), str(replica)], check=True)
        replica_cache = CiCache(replica, cache_root, {"toolchain": "fixture"})
        assert not replica_cache.run(
            "fixture",
            ("included.txt",),
            lambda: calls.append("replica"),
        )

        disabled = CiCache(
            repository,
            cache_root,
            {"toolchain": "fixture"},
            enabled=False,
        )
        assert disabled.run("fixture", ("included.txt",), lambda: calls.append("disabled-1"))
        assert disabled.run("fixture", ("included.txt",), lambda: calls.append("disabled-2"))

        unrelated.write_text("staged unrelated change\n")
        subprocess.run(["git", "add", "unrelated.txt"], cwd=repository, check=True)
        assert not cache.run(
            "fixture",
            ("included.txt",),
            lambda: calls.append("unrelated"),
        )

        included.write_text("staged included change\n")
        subprocess.run(["git", "add", "included.txt"], cwd=repository, check=True)
        assert cache.run("fixture", ("included.txt",), lambda: calls.append("changed"))

        included.write_text("unstaged included change\n")
        assert cache.run("fixture", ("included.txt",), lambda: calls.append("unstaged-1"))
        assert cache.run("fixture", ("included.txt",), lambda: calls.append("unstaged-2"))

        failures = 0

        def fail() -> None:
            nonlocal failures
            failures += 1
            raise RuntimeError("expected failure")

        for _ in range(2):
            try:
                cache.run("failure", ("unrelated.txt",), fail)
            except RuntimeError:
                pass
            else:
                raise AssertionError("failed cached step unexpectedly passed")
        assert failures == 2
        print("CI Cache tests passed.")


if __name__ == "__main__":
    main()
