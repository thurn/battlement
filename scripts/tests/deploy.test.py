#!/usr/bin/env python3
"""Exercise Cloudflare sample-site assembly without building or deploying."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import json
import sys


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))
SPEC = importlib.util.spec_from_file_location("deploy", REPOSITORY_ROOT / "scripts/deploy.py")
assert SPEC and SPEC.loader
deploy = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(deploy)


def create_sample(root: Path, name: str) -> None:
    sample = root / "samples" / name
    sample.mkdir(parents=True)
    (sample / "sample.toml").write_text(f'executable = "{name}"\n')
    output = sample / "Build/release/WebThreads"
    (output / "Build").mkdir(parents=True)
    (output / "StreamingAssets").mkdir()
    (output / "index.html").write_text("<html></html>")
    (output / "Build/Web.wasm.unityweb").write_bytes(b"\x1f\x8bfixture")
    (output / "StreamingAssets/assets_0123456789abcdef0123456789abcdef.bundle").write_bytes(b"asset")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-deploy-test.") as temporary:
        root = Path(temporary)
        for name in ("tictactoe", "basic", "chess"):
            create_sample(root, name)

        deploy.REPOSITORY_ROOT = root
        deploy.STAGING_ROOT = root / "Build/cloudflare"
        names = deploy.sample_names()
        assert names == ["basic", "chess", "tictactoe"]

        commands: list[list[str]] = []
        original_run = deploy.run
        deploy.run = lambda command, **_kwargs: commands.append(command) or ""
        try:
            deploy.build_samples("chess", names)
        finally:
            deploy.run = original_run
        assert [command[-3] for command in commands] == ["chess", "basic", "tictactoe"]

        deploy.assemble_site(names, "0123456789abcdef")
        deploy.validate_site(names)
        root_index = (deploy.STAGING_ROOT / "index.html").read_text()
        assert all(f'href="/{name}/"' in root_index for name in names)
        headers = (deploy.STAGING_ROOT / "_headers").read_text()
        assert "Cross-Origin-Embedder-Policy: require-corp" in headers
        assert "Content-Type: application/wasm" in headers
        assert "Cache-Control: public, max-age=31536000, immutable" in headers

        revision = "0123456789abcdef"
        deploy.require_deployable_checkout = lambda: revision
        deploy.run = lambda command, **_kwargs: commands.append(command) or ""
        evidence = root / "browser-result.json"
        def passing_check(_repository, site, selected, source):
            assert selected == names and source == revision
            evidence.write_text(json.dumps({"status": "passed", "site_sha256": deploy.site_digest(site)}))
            return evidence
        deploy.check_site = passing_check
        commands.clear()
        deploy.publish_site(names, revision)
        assert len(commands) == 1 and commands[0][1] == "deploy"

        for failure in ("browser", "site-drift", "source-drift"):
            commands.clear()
            deploy.require_deployable_checkout = lambda: revision
            def failed_check(repository, site, selected, source):
                result = passing_check(repository, site, selected, source)
                if failure == "browser":
                    raise RuntimeError("The declared sample interaction failed")
                if failure == "site-drift":
                    (site / "index.html").write_text("changed after validation")
                if failure == "source-drift":
                    deploy.require_deployable_checkout = lambda: "different-revision"
                return result
            deploy.check_site = failed_check
            try:
                deploy.publish_site(names, revision)
                raise AssertionError(f"Publication accepted {failure}")
            except RuntimeError:
                pass
            assert commands == [], f"Publishing command ran despite {failure}"


if __name__ == "__main__":
    main()
