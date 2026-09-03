#!/usr/bin/env python3
"""Exercise the supported replay command and its prerequisite failures."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ditto_replay


def main() -> None:
    scripts = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="ditto-replay-test.") as temporary:
        root = Path(temporary)
        (root / "scripts").mkdir()
        for name in ("ditto_ci.py", "ditto_replay.py"):
            shutil.copy2(scripts / name, root / "scripts" / name)
        config = root / "samples/chess-ui/ditto.toml"
        config.parent.mkdir(parents=True)
        configuration = """name = 'fixture'
[[scenarios]]
name = 'gallery reset'
steps = [{screenshot = {name = 'initial'}}]
[[scenarios]]
name = 'collection accessibility'
steps = []
"""
        config.write_text(configuration)
        config.with_name("ditto.lock").write_text("original baselines")
        binary = root / "ditto"
        binary.write_text(f"#!{sys.executable}\n" + '''
import json, os, pathlib, sys
if "--version" in sys.argv:
    print("4.5.0")
    raise SystemExit(0)
assert os.environ["DITTO_REPLAY_BUILD_FINGERPRINT"] == "a" * 64
assert "--no-build" in sys.argv
assert sys.argv[sys.argv.index("--profile") + 1] == "macos"
if sys.argv[-3] == "gallery reset":
    assert pathlib.Path(os.environ["DITTO_ODIFF_PATH"]).is_file()
else:
    assert sys.argv[-3] == "collection accessibility"
    assert "DITTO_ODIFF_PATH" not in os.environ
pathlib.Path(os.environ["PLAYER_MARKER"]).write_text("executed")
output = pathlib.Path(sys.argv[sys.argv.index("--output") + 1])
output.write_text(json.dumps({"status":"passed", "errors":[]}))
''')
        binary.chmod(0o755)
        if os.name == "nt":
            script = binary
            binary = root / "ditto.cmd"
            binary.write_text(f'@"{sys.executable}" "{script}" %*\n')
        environment = {"DITTO_CACHE_ROOT": str(root / "cache"), "DITTO_ODIFF_PATH": str(binary)}
        recipe = ditto_replay.record(root, binary, root / "cache", "chess-ui",
                                     ["gallery reset", "collection accessibility"], environment)
        result = {"run_id": "original-failure", "status": "failed",
                  "build": {"fingerprint": "a" * 64}}
        retained = root / "replay.json"
        ditto_replay.save(recipe, retained, result)
        original = retained.read_bytes()
        marker = root / "player-started"
        env = os.environ.copy()
        env["PLAYER_MARKER"] = str(marker)
        env["DITTO_ODIFF_PATH"] = "/incorrect/ambient/tool"

        def invoke(recipe_path=retained, scenario="gallery reset"):
            return subprocess.run([sys.executable, str(root / "scripts/ditto_ci.py"),
                                   "replay", str(recipe_path), scenario],
                                  env=env, capture_output=True, text=True)

        binary.write_text("a rebuilt runner must not be used")
        passed = invoke()
        assert passed.returncode == 0, passed.stderr + passed.stdout
        assert marker.read_text() == "executed"
        assert "Original original-failure: failed" in passed.stdout
        assert retained.read_bytes() == original
        marker.unlink()

        no_comparison = ditto_replay.record(
            root, Path(recipe["tools"]["runner"]["path"]), root / "cache", "chess-ui",
            ["collection accessibility"], {"DITTO_CACHE_ROOT": str(root / "cache")},
        )
        no_comparison_path = root / "no-comparison.json"
        ditto_replay.save(no_comparison, no_comparison_path, result)
        no_odiff = invoke(no_comparison_path, "collection accessibility")
        assert no_odiff.returncode == 0, no_odiff.stderr + no_odiff.stdout
        assert marker.read_text() == "executed"
        marker.unlink()

        config.write_text("changed scenario")
        changed = invoke()
        assert changed.returncode == 1
        assert "Replay configuration changed" in changed.stderr
        assert not marker.exists()
        config.write_text(configuration)
        pinned = Path(recipe["tools"]["DITTO_ODIFF_PATH"]["path"])
        pinned.unlink()
        missing = invoke()
        assert missing.returncode == 1
        assert "dependency is missing or changed" in missing.stderr
        assert not marker.exists()
        assert retained.read_bytes() == original
    print("Ditto replay tests passed.")


if __name__ == "__main__":
    main()
