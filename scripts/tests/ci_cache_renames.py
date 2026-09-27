"""Exercise staged rename cache reuse with real Git status records."""

from pathlib import Path
import subprocess

from ci_cache import CiCache


def verify_renamed_inputs(root: Path) -> None:
    repository = root / "rename-repository"
    repository.mkdir()
    subprocess.run(["git", "init", "--quiet"], cwd=repository, check=True)
    selected = repository / "selected"
    selected.mkdir()
    original = selected / "original file.txt"
    original.write_text("original\n")
    subprocess.run(["git", "add", "."], cwd=repository, check=True)
    subprocess.run(
        ["git", "-c", "user.name=CI Fixture", "-c", "user.email=ci@example.invalid",
         "commit", "--quiet", "-m", "fixture"],
        cwd=repository, check=True,
    )
    cache = CiCache(repository, root / "rename-cache", {"toolchain": "fixture"})
    calls = []

    def run() -> bool:
        return cache.run("rename", ("selected",), lambda: calls.append("executed"))

    assert run()
    assert not run()
    renamed = selected / "renamed file.txt"
    subprocess.run(["git", "mv", str(original), str(renamed)], cwd=repository, check=True)
    assert run()
    assert not run(), "fully staged rename bypassed the cache"
    assert len(calls) == 2

    renamed.write_text("unstaged edit\n")
    assert run()
    assert run(), "unstaged rename destination was cached"
    subprocess.run(["git", "add", "."], cwd=repository, check=True)
    assert run()
    assert not run()

    untracked = selected / "untracked file.txt"
    untracked.write_text("untracked\n")
    assert run()
    assert run(), "untracked selected input was cached"
    untracked.unlink()
    (repository / "unrelated.txt").write_text("outside selection\n")
    assert not run(), "untracked unrelated input invalidated selection"

    renamed.unlink()
    assert run()
    assert run(), "unstaged deletion was cached"
