#!/usr/bin/env python3
"""Exercise Cloudflare sample-site assembly without building or deploying."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
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
    with tempfile.TemporaryDirectory(prefix="masonry-deploy-test.") as temporary:
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


if __name__ == "__main__":
    main()
