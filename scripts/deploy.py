#!/usr/bin/env python3
"""Build and deploy the Battlement sample site to Cloudflare Workers."""

from __future__ import annotations

import argparse
import html
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
STAGING_ROOT = REPOSITORY_ROOT / "Build/cloudflare"
WRANGLER = REPOSITORY_ROOT / "node_modules/.bin/wrangler"
PUBLIC_URL = "https://samples.battlement.workers.dev"
MAX_FILE_SIZE = 25 * 1024 * 1024
MAX_FILES = 20_000
HASHED_NAME = re.compile(r"[0-9a-f]{32}")


def run(command: list[str], *, capture: bool = False) -> str:
    result = subprocess.run(
        command,
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=capture,
        text=True,
    )
    return result.stdout.strip() if capture else ""


def sample_names() -> list[str]:
    """Return convention-based sample names in stable order."""
    return sorted(path.parent.name for path in (REPOSITORY_ROOT / "samples").glob("*/sample.toml"))


def require_deployable_checkout() -> str:
    branch = run(["git", "branch", "--show-current"], capture=True)
    if branch != "master":
        raise RuntimeError(f"deployments require branch master; current branch is {branch or 'detached HEAD'}")
    if run(["git", "status", "--porcelain", "--untracked-files=no"], capture=True):
        raise RuntimeError("deployments require a clean checkout with no tracked changes")
    return run(["git", "rev-parse", "HEAD"], capture=True)


def require_wrangler() -> None:
    if not WRANGLER.is_file():
        raise RuntimeError("Wrangler is not installed; run `npm ci` from the repository root")
    run([str(WRANGLER), "whoami"])


def build_samples(target: str, names: list[str]) -> None:
    order = names if target == "all" else [target, *(name for name in names if name != target)]
    if target != "all":
        print("A named deployment still rebuilds the complete sample site.", flush=True)
    for name in order:
        print(f"\n==> Build {name}", flush=True)
        run([
            "cargo", "run", "--quiet", "-p", "battlement-cli", "--", "sample", "build",
            name, "--web", "--release",
        ])


def build_root_index(names: list[str], revision: str) -> str:
    links = "\n".join(
        f'      <li><a href="/{html.escape(name)}/">{html.escape(name.title())}</a></li>'
        for name in names
    )
    return f"""<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Battlement samples</title>
  </head>
  <body>
    <main>
      <h1>Battlement samples</h1>
      <ul>
{links}
      </ul>
      <small>Revision {html.escape(revision[:12])}</small>
    </main>
  </body>
</html>
"""


def unity_headers(path: Path) -> list[str]:
    relative = "/" + path.relative_to(STAGING_ROOT).as_posix()
    headers = [relative, "  Content-Encoding: gzip"]
    if path.name.endswith(".wasm.unityweb"):
        headers.append("  Content-Type: application/wasm")
    elif path.name.endswith((".framework.js.unityweb", ".js.unityweb")):
        headers.append("  Content-Type: application/javascript")
    else:
        headers.append("  Content-Type: application/octet-stream")
    return headers


def build_headers(files: list[Path]) -> str:
    sections = [[
        "/*",
        "  Cross-Origin-Opener-Policy: same-origin",
        "  Cross-Origin-Embedder-Policy: require-corp",
        "  Cross-Origin-Resource-Policy: same-origin",
        "  X-Content-Type-Options: nosniff",
    ]]
    sections.extend(unity_headers(path) for path in files if path.name.endswith(".unityweb"))
    sections.extend(
        [
            "/" + path.relative_to(STAGING_ROOT).as_posix(),
            "  Cache-Control: public, max-age=31536000, immutable",
        ]
        for path in files
        if HASHED_NAME.search(path.name)
    )
    if len(sections) > 100:
        raise RuntimeError(f"generated {len(sections)} header rules; Cloudflare allows 100")
    return "\n\n".join("\n".join(section) for section in sections) + "\n"


def assemble_site(names: list[str], revision: str) -> None:
    shutil.rmtree(STAGING_ROOT, ignore_errors=True)
    STAGING_ROOT.mkdir(parents=True)
    for name in names:
        output = REPOSITORY_ROOT / "samples" / name / "Build/release/WebThreads"
        if not output.is_dir():
            raise RuntimeError(f"sample build output is missing: {output}")
        shutil.copytree(output, STAGING_ROOT / name)
    (STAGING_ROOT / "index.html").write_text(build_root_index(names, revision))
    files = [path for path in STAGING_ROOT.rglob("*") if path.is_file()]
    (STAGING_ROOT / "_headers").write_text(build_headers(files))


def validate_site(names: list[str]) -> None:
    files = [path for path in STAGING_ROOT.rglob("*") if path.is_file()]
    if len(files) > MAX_FILES:
        raise RuntimeError(f"site has {len(files)} files; Cloudflare allows {MAX_FILES}")
    for path in files:
        if path.stat().st_size > MAX_FILE_SIZE:
            raise RuntimeError(f"file exceeds Cloudflare's 25 MiB limit: {path}")
    for name in names:
        root = STAGING_ROOT / name
        if not (root / "index.html").is_file():
            raise RuntimeError(f"{name} build has no index.html")
        wasm = list(root.rglob("*.wasm.unityweb"))
        if not wasm:
            raise RuntimeError(f"{name} build has no compressed WebAssembly player")
        for path in wasm:
            if path.read_bytes()[:2] != b"\x1f\x8b":
                raise RuntimeError(f"WebAssembly player is not gzip-compressed: {path}")


def request(path: str) -> urllib.response.addinfourl:
    return urllib.request.urlopen(f"{PUBLIC_URL}{path}", timeout=20)


def smoke_test(names: list[str]) -> None:
    error: Exception | None = None
    for attempt in range(3):
        try:
            with request("/") as response:
                if response.status != 200:
                    raise RuntimeError(f"root returned HTTP {response.status}")
            for name in names:
                with request(f"/{name}/") as response:
                    if response.status != 200:
                        raise RuntimeError(f"/{name}/ returned HTTP {response.status}")
                wasm = next((STAGING_ROOT / name).rglob("*.wasm.unityweb"))
                wasm_path = f"/{name}/{wasm.relative_to(STAGING_ROOT / name).as_posix()}"
                with request(wasm_path) as response:
                    if response.headers.get_content_type() != "application/wasm":
                        raise RuntimeError(f"{wasm_path} has the wrong Content-Type")
                    if response.headers.get("Content-Encoding") != "gzip":
                        raise RuntimeError(f"{wasm_path} has the wrong Content-Encoding")
                    if response.headers.get("Cross-Origin-Embedder-Policy") != "require-corp":
                        raise RuntimeError(f"{wasm_path} is missing cross-origin isolation headers")
            return
        except (OSError, RuntimeError, urllib.error.HTTPError) as caught:
            error = caught
            if attempt < 2:
                time.sleep(2)
    raise RuntimeError(f"live smoke test failed: {error}")


def parse_arguments(names: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", choices=[*names, "all"])
    return parser.parse_args()


def main() -> None:
    names = sample_names()
    args = parse_arguments(names)
    revision = require_deployable_checkout()
    require_wrangler()
    build_samples(args.target, names)
    assemble_site(names, revision)
    validate_site(names)
    run([
        str(WRANGLER), "deploy", "--strict", "--message",
        f"Battlement samples at {revision}",
    ])
    try:
        smoke_test(names)
    except RuntimeError as error:
        print(error, file=sys.stderr)
        print("Rollback with: npm exec -- wrangler rollback --name samples", file=sys.stderr)
        raise
    print(f"Deployed {revision[:12]} to {PUBLIC_URL}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
