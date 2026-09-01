#!/usr/bin/env python3

"""Verify content-addressed CI Cache reuse."""

from __future__ import annotations

from pathlib import Path
import os
import subprocess
import sys
import tempfile
from unittest.mock import patch


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

import ci_cache  # noqa: E402
from ci_cache import CiCache, charged_size, prune_chrome_code_sign_clones  # noqa: E402


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

        if hasattr(os, "symlink"):
            linked = cache_root / "linked"
            linked.mkdir(parents=True)
            (linked / "target").write_bytes(b"target")
            os.symlink(linked / "target", linked / "link")
            charged_size(linked)
            try:
                charged_size(linked / "link")
            except RuntimeError:
                pass
            else:
                raise AssertionError("symbolic-link cache root was accepted")

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

        legacy = cache_root / "cargo-targets/old-checkout/target"
        legacy.mkdir(parents=True)
        (legacy / "artifact").write_bytes(b"legacy")
        shared_old = cache_root / "cargo-targets/shared/old"
        shared_new = cache_root / "cargo-targets/shared/new"
        shared_old.mkdir(parents=True)
        shared_new.mkdir(parents=True)
        (shared_old / "artifact").write_bytes(b"old")
        (shared_new / "artifact").write_bytes(b"new")
        old_time = 1_000_000_000
        new_time = 2_000_000_000
        shared_old.touch()
        shared_new.touch()
        os.utime(shared_old, ns=(old_time, old_time))
        os.utime(shared_new, ns=(new_time, new_time))
        result = cache.prune(target_bytes=0, high_water_bytes=0)
        assert legacy.parent in result.removed
        assert shared_old in result.removed
        assert shared_new in result.removed
        assert charged_size(cache_root) == result.after_bytes
        _verify_maintenance_cadence(cache)
        _verify_targeted_chrome_scan(root)
        _verify_chrome_clone_pruning(root)
        print("CI Cache tests passed.")


def _verify_chrome_clone_pruning(root: Path) -> None:
    clone_root = root / "com.google.Chrome.code_sign_clone"
    clone_root.mkdir()
    old_unused = clone_root / "code_sign_clone.OLD001"
    old_open = clone_root / "code_sign_clone.OPEN01"
    recent = clone_root / "code_sign_clone.NEW001"
    unrelated = clone_root / "unrelated"
    for path in (old_unused, old_open, recent, unrelated):
        path.mkdir()
        (path / "payload").write_bytes(b"clone")
    now_ns = 10_000_000_000
    old_ns = 1_000_000_000
    for path in (old_unused, old_open):
        os.utime(path, ns=(old_ns, old_ns))
    result = prune_chrome_code_sign_clones(
        clone_root,
        {old_open.name},
        now_ns,
        minimum_age_seconds=5,
    )
    assert result.removed == (old_unused,)
    assert not old_unused.exists()
    assert old_open.is_dir()
    assert recent.is_dir()
    assert unrelated.is_dir()


def _verify_maintenance_cadence(cache: CiCache) -> None:
    now_ns = 10_000_000_000
    with patch.object(ci_cache, "prune_chrome_code_sign_clones") as chrome:
        chrome.return_value = ci_cache.CachePruneResult(0, 0, ())
        assert cache.maintain(now_ns, interval_seconds=5)
        assert not cache.maintain(now_ns + 4_000_000_000, interval_seconds=5)
        assert cache.maintain(now_ns + 5_000_000_000, interval_seconds=5)
        assert chrome.call_count == 2


def _verify_targeted_chrome_scan(root: Path) -> None:
    clone_root = root / "targeted-code-sign-clones"
    clone = clone_root / "code_sign_clone.OPEN01"
    clone.mkdir(parents=True)
    completed = subprocess.CompletedProcess(
        ["lsof"],
        0,
        stdout=f"p123\nfcwd\nn{clone}/payload\n",
        stderr="",
    )
    with patch.object(ci_cache.subprocess, "run", return_value=completed) as run:
        assert ci_cache._open_chrome_clones(clone_root) == {clone.name}
        command = run.call_args.args[0]
        assert command[1:] == ["-Fn", "+D", str(clone_root)]
        assert run.call_args.kwargs["timeout"] == ci_cache.CHROME_LSOF_TIMEOUT_SECONDS


if __name__ == "__main__":
    main()
