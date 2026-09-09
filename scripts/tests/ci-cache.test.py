#!/usr/bin/env python3

"""Verify content-addressed CI Cache reuse."""

from __future__ import annotations

from pathlib import Path
import multiprocessing
import os
from queue import Empty
import subprocess
import sys
import tempfile
import threading
import time
from unittest.mock import patch


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

import ci_cache  # noqa: E402
from ci_cache import CiCache, charged_size, prune_chrome_code_sign_clones  # noqa: E402
from resource_slots import SlotLease  # noqa: E402


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-ci-cache-test.") as temporary:
        root = Path(temporary)
        repository = root / "repository"
        cache_root = root / "cache"
        slots = root / "resource-slots"
        capacity = _isolated_compiler_capacity(slots)
        capacity.start()
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
        events: list[tuple[str, dict[str, object]]] = []
        cache = CiCache(
            repository,
            cache_root,
            {"toolchain": "fixture"},
            event=lambda event, attributes: events.append((event, attributes)),
        )
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
        assert [
            attributes["result"]
            for event, attributes in events
            if event == "ci.cache_lookup"
        ] == ["miss", "hit"]
        _verify_distinct_misses_execute_concurrently(repository, cache_root)
        _verify_concurrent_miss_publishes_once(repository, cache_root)
        _verify_maintenance_and_execution_do_not_deadlock(repository, cache_root)
        _verify_compiler_capacity_is_bounded(repository, cache_root, slots)
        _verify_failure_releases_capacity(repository, cache_root)

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
        assert any(
            event == "ci.cache_lookup" and attributes["result"] == "bypassed"
            for event, attributes in events
        )

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
        with cache.invocation():
            pass
        assert any(event == "ci.cache_wait" for event, _attributes in events)
        _verify_targeted_chrome_scan(root)
        _verify_chrome_clone_pruning(root)
        capacity.stop()
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


def _isolated_compiler_capacity(slots: Path):
    return patch.multiple(
        ci_cache,
        compiler_capacity_lease=lambda: SlotLease(slots, "machine-heavy", 6, 3),
        compiler_maintenance_lease=lambda: SlotLease(slots, "machine-heavy", 6, 6),
    )


def _thread(function, errors: list[BaseException]) -> threading.Thread:
    def guarded() -> None:
        try:
            function()
        except BaseException as error:
            errors.append(error)

    result = threading.Thread(target=guarded)
    result.start()
    return result


def _join(threads: list[threading.Thread], errors: list[BaseException]) -> None:
    for thread in threads:
        thread.join(timeout=2)
    assert not any(thread.is_alive() for thread in threads), "concurrent cache test timed out"
    if errors:
        raise errors[0]


def _cache_process(
    repository: str,
    cache_root: str,
    slots: str,
    step: str,
    toolchain: str,
    ready,
    messages,
    release,
) -> None:
    with _isolated_compiler_capacity(Path(slots)):
        cache = CiCache(
            Path(repository),
            Path(cache_root),
            {"toolchain": toolchain},
            event=lambda event, attributes: messages.put(
                ("event", step, event, attributes)
            ),
        )
        ready.put(step)

        def execute() -> None:
            messages.put(("execute", step))
            if not release.wait(2):
                raise TimeoutError("cache process was not released")

        cache.run(step, ("included.txt",), execute)
        messages.put(("finished", step))


def _cache_processes(
    repository: Path,
    cache_root: Path,
    slots: Path,
    steps: tuple[str, str],
    toolchain: str,
):
    context = multiprocessing.get_context("spawn")
    ready = context.Queue()
    messages = context.Queue()
    release = context.Event()
    processes = [
        context.Process(
            target=_cache_process,
            args=(
                str(repository),
                str(cache_root),
                str(slots),
                step,
                toolchain,
                ready,
                messages,
                release,
            ),
        )
        for step in steps
    ]
    for process in processes:
        process.start()
    assert {ready.get(timeout=1), ready.get(timeout=1)} == set(steps)
    return processes, messages, release


def _join_processes(processes, release) -> None:
    release.set()
    for process in processes:
        process.join(timeout=2)
    assert all(not process.is_alive() for process in processes), (
        "concurrent cache process timed out"
    )
    assert [process.exitcode for process in processes] == [0, 0]


def _verify_distinct_misses_execute_concurrently(
    repository: Path, cache_root: Path
) -> None:
    processes, messages, release = _cache_processes(
        repository,
        cache_root,
        cache_root.parent / "resource-slots",
        ("distinct-a", "distinct-b"),
        "distinct",
    )
    try:
        executing = set()
        while len(executing) < 2:
            message = messages.get(timeout=1)
            if message[0] == "execute":
                executing.add(message[1])
        assert executing == {"distinct-a", "distinct-b"}
    finally:
        _join_processes(processes, release)


def _verify_concurrent_miss_publishes_once(
    repository: Path, cache_root: Path
) -> None:
    processes, messages, release = _cache_processes(
        repository,
        cache_root,
        cache_root.parent / "resource-slots",
        ("same-key", "same-key"),
        "deduplicate",
    )
    observed = []
    try:
        while not (
            any(message[0] == "execute" for message in observed)
            and any(
                message[0] == "event"
                and message[2] == "ci.cache_waiting"
                and message[3].get("resource") == "cache-key"
                for message in observed
            )
        ):
            observed.append(messages.get(timeout=1))
    finally:
        _join_processes(processes, release)
    while True:
        try:
            observed.append(messages.get_nowait())
        except Empty:
            break
    assert sum(message[0] == "execute" for message in observed) == 1


def _verify_maintenance_and_execution_do_not_deadlock(
    repository: Path, cache_root: Path
) -> None:
    cache = CiCache(repository, cache_root, {"toolchain": "maintenance-race"})
    start = threading.Barrier(3, timeout=1)
    errors: list[BaseException] = []

    def execute() -> None:
        start.wait()
        with cache.invocation():
            pass

    def maintain() -> None:
        start.wait()
        empty = ci_cache.CachePruneResult(0, 0, ())
        with (
            patch.object(cache, "prune", return_value=empty),
            patch.object(ci_cache, "prune_chrome_code_sign_clones", return_value=empty),
        ):
            cache.maintain(now_ns=time.time_ns(), interval_seconds=0)

    threads = [_thread(execute, errors), _thread(maintain, errors)]
    start.wait()
    _join(threads, errors)


def _verify_compiler_capacity_is_bounded(
    repository: Path, cache_root: Path, slots: Path
) -> None:
    cache = CiCache(repository, cache_root, {"toolchain": "bounded"})
    entered = 0
    maximum = 0
    condition = threading.Condition()
    release = threading.Event()
    errors: list[BaseException] = []

    def execute() -> None:
        nonlocal entered, maximum
        with condition:
            entered += 1
            maximum = max(maximum, entered)
            condition.notify_all()
        assert release.wait(1)
        with condition:
            entered -= 1

    threads = [
        _thread(lambda step=step: cache.run(step, ("included.txt",), execute), errors)
        for step in ("bounded-a", "bounded-b", "bounded-c")
    ]
    with condition:
        assert condition.wait_for(lambda: entered == 2, timeout=1)
    deadline = time.monotonic() + 1
    while not list(slots.glob(".machine-heavy.queue.*.lock")):
        assert time.monotonic() < deadline, "third compiler writer did not queue"
        time.sleep(0.01)
    assert maximum == 2
    release.set()
    _join(threads, errors)


def _verify_failure_releases_capacity(repository: Path, cache_root: Path) -> None:
    cache = CiCache(repository, cache_root, {"toolchain": "failure-release"})
    try:
        cache.run(
            "failure-release",
            ("included.txt",),
            lambda: (_ for _ in ()).throw(RuntimeError("expected failure")),
        )
    except RuntimeError:
        pass
    else:
        raise AssertionError("failed step unexpectedly passed")
    assert cache.run("after-failure", ("included.txt",), lambda: None)


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
