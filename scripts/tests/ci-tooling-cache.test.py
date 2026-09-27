#!/usr/bin/env python3
"""Exercise tooling result reuse through real child processes and staged inputs."""

import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
DRIVER = '''
import json, os, sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import ci_tooling
import ci_tooling_cache
from ci_cache import CiCache
fixture_environment = ci_tooling_cache.fixture_environment
ci_tooling_cache.fixture_environment = lambda: fixture_environment() | {
    key: value for key, value in os.environ.items() if key.startswith("FIXTURE_")
}
repo, cache = map(Path, sys.argv[2:4])
ci_tooling.CHECKS = (("Reusable", "scripts/tests/wire-contracts.test.py"),
                     ("Live", "scripts/tests/live.test.py"))
ci_tooling.PERFORMANCE_CHECKS = (("Extra", "scripts/tests/extra.test.py"),)
cache = CiCache(repo, cache, {"tool": sys.argv[4]},
                enabled=sys.argv[5] != "disabled",
                event=lambda name, attrs: print(json.dumps({"event": name, **attrs}), flush=True))
ci_tooling.run(repo, cache=cache, performance=sys.argv[5] == "performance")
'''
FIXTURE = '''
import os, time
from pathlib import Path
with Path(os.environ["FIXTURE_LOG"]).open("a") as f: f.write("reusable\\n")
assert Path("samples/fixture/ditto.toml").read_text() != "fail"
assert Path("Packages/fixture.txt").read_text() != "fail"
if os.environ.get("FIXTURE_HOLD"):
    Path(os.environ["FIXTURE_HOLD"]).touch()
    time.sleep(60)
'''


def git(repo, *args):
    return subprocess.check_output(["git", "-c", "core.hooksPath=/dev/null", *args], cwd=repo)


def main():
    with tempfile.TemporaryDirectory(prefix="tooling-cache-test.") as temporary:
        root = Path(temporary)
        repo = root / "repo"
        (repo / "scripts/tests").mkdir(parents=True)
        (repo / "samples/fixture").mkdir(parents=True)
        (repo / "Packages").mkdir()
        (repo / "samples/fixture/ditto.toml").write_text("sample")
        (repo / "Packages/fixture.txt").write_text("unity")
        fixture = repo / "scripts/tests/wire-contracts.test.py"
        fixture.write_text(FIXTURE)
        for name in ("live", "extra"):
            (repo / f"scripts/tests/{name}.test.py").write_text(
                f'import os; from pathlib import Path\n'
                f'with Path(os.environ["FIXTURE_LOG"]).open("a") as f: f.write("{name}\\n")\n')
        git(repo, "init", "-q")
        git(repo, "add", ".")
        cache = root / "cache"
        cache.mkdir()
        # Keep the actual cache maintenance mechanism outside this focused test.
        (cache / "maintenance.json").write_text(json.dumps({"schema": 1, "completedAt": time.time_ns()}))
        log = root / "executions"
        environment = os.environ | {"FIXTURE_LOG": str(log), "BATTLEMENT_LOG_ROOT": str(root / "logs")}

        def run(checkout=repo, *, tool="one", mode="normal", extra=None, success=True):
            before = log.read_text().splitlines() if log.exists() else []
            result = subprocess.run([sys.executable, "-c", DRIVER, str(ROOT / "scripts"),
                str(checkout), str(cache), tool, mode], env=environment | (extra or {}),
                capture_output=True, text=True)
            assert (result.returncode == 0) == success, result.stdout + result.stderr
            after = log.read_text().splitlines()
            return after[len(before):], result.stdout

        def expect_replay(**options):
            ran, output = run(**options)
            assert sorted(ran) == ["live", "reusable"], (ran, output)
            return output

        expect_replay()
        ran, output = run()
        assert ran == ["live"], output
        hit = next(json.loads(line) for line in output.splitlines()
                   if line.startswith('{') and json.loads(line).get("result") == "hit")
        assert Path(hit["provenance"]["fixture_logs"]).is_dir()
        assert hit["provenance"]["source"]["worktree_path"] == str(repo.resolve())
        # An equivalent independent index reuses staged content without sharing .git.
        replica = root / "replica"
        import shutil
        shutil.copytree(repo, replica, ignore=shutil.ignore_patterns('.git'))
        git(replica, "init", "-q")
        git(replica, "add", ".")
        assert run(replica)[0] == ["live"]
        assert run(extra={"TERM_PROGRAM": "different-caller", "CODEX_TURN_ID": "other",
                          "BATTLEMENT_CI_JOB_ID": "another-job"})[0] == ["live"]
        assert run(extra={"PATH": str(root / "unrelated-bin") + os.pathsep + environment["PATH"]})[0] == ["live"]
        for relative in ("scripts/tests/wire-contracts.test.py", "samples/fixture/ditto.toml", "Packages/fixture.txt"):
            path = repo / relative
            original = path.read_text()
            path.write_text(original + ("\n# changed\n" if relative.endswith('.py') else "changed"))
            git(repo, "add", "--renormalize", ".")
            expect_replay()
        expect_replay(tool="two")
        expect_replay(extra={"FIXTURE_RELEVANT_SETTING": "changed"})
        binaries = root / "bin"
        binaries.mkdir()
        node = binaries / ("node.exe" if os.name == "nt" else "node")
        node.write_bytes(b"tool version one")
        node.chmod(0o755)
        tool_path = {"PATH": str(binaries) + os.pathsep + environment["PATH"]}
        expect_replay(extra=tool_path)
        assert run(extra=tool_path)[0] == ["live"]
        node.write_bytes(b"tool version two")
        expect_replay(extra=tool_path)
        ran, _ = run(mode="performance")
        assert sorted(ran) == ["extra", "live", "reusable"]
        assert sorted(run(mode="performance")[0]) == ["extra", "live"]
        for mode in ("disabled", "disabled"):
            expect_replay(mode=mode)
        dirty = repo / "Packages/fixture.txt"
        dirty.write_text("unstaged")
        assert "bypassed" in expect_replay()
        assert "bypassed" in expect_replay()
        git(repo, "add", "--renormalize", ".")
        untracked = repo / "Packages/untracked.txt"
        untracked.write_text("untracked")
        assert "bypassed" in expect_replay()
        untracked.unlink()
        if os.name != "nt":
            link = repo / "external"
            link.symlink_to(root / "executions")
            git(repo, "add", "external")
            assert "external input boundary" in expect_replay()
            link.unlink()
            git(repo, "rm", "--cached", "external")
        # Failed fixtures cannot publish successful reuse for their new staged key.
        dirty.write_text("fail")
        git(repo, "add", "--renormalize", ".")
        for _ in range(2):
            ran, output = run(success=False)
            assert "reusable" in ran and "CI Cache miss" in output
        dirty.write_text("restored")
        git(repo, "add", "--renormalize", ".")
        # Interrupted execution owns its children and never reaches marker publication.
        if os.name != "nt":
            marker = root / "holding"
            command = [sys.executable, "-c", DRIVER, str(ROOT / "scripts"), str(repo), str(cache), "cancel", "normal"]
            before = set((cache / "entries/repository-tooling").glob('*.json'))
            with (root / "cancellation.log").open('w') as output:
                child = subprocess.Popen(command, env=environment | {"FIXTURE_HOLD": str(marker)},
                                         stdout=output, stderr=output, process_group=0)
                try:
                    deadline = time.monotonic() + 10
                    while not marker.exists():
                        assert child.poll() is None
                        assert time.monotonic() < deadline
                        time.sleep(.02)
                    os.killpg(child.pid, signal.SIGTERM)
                    child.wait(timeout=5)
                finally:
                    if child.poll() is None:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait()
            assert set((cache / "entries/repository-tooling").glob('*.json')) == before
        print("Tooling result reuse: staged closure, replica, selections, environment, bypass, failure and cancellation passed")


if __name__ == '__main__':
    main()
