#!/usr/bin/env python3

"""Verify that the local server labels compressed Unity artifacts correctly."""

from __future__ import annotations

import functools
from http.server import ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

import serve_web  # noqa: E402


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-web-server-test.") as temporary:
        root = Path(temporary)
        build = root / "build"
        build.mkdir()
        artifacts = {
            "sample.data.unityweb": "application/octet-stream",
            "sample.framework.js.unityweb": "application/javascript",
            "sample.wasm.unityweb": "application/wasm",
        }
        for name in artifacts:
            (build / name).write_bytes(b"\x1f\x8bfixture")

        handler = functools.partial(serve_web.IsolatedHandler, directory=build)
        server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
        thread = threading.Thread(target=server.serve_forever)
        thread.start()
        try:
            for name, content_type in artifacts.items():
                with urllib.request.urlopen(
                    f"http://127.0.0.1:{server.server_port}/{name}"
                ) as response:
                    assert response.headers["Content-Type"] == content_type
                    assert response.headers["Content-Encoding"] == "gzip"
                    assert response.headers["Cross-Origin-Opener-Policy"] == "same-origin"
                    assert response.headers["Cross-Origin-Embedder-Policy"] == "require-corp"
        finally:
            server.shutdown()
            thread.join()
            server.server_close()

        handle = root / "review.json"
        process = subprocess.Popen(
            [
                sys.executable,
                str(REPOSITORY_ROOT / "scripts/serve_web.py"),
                "--directory",
                str(build),
                "--handle",
                str(handle),
            ],
            env={**os.environ, "BATTLEMENT_LOG_ROOT": str(root / "logs")},
        )
        try:
            deadline = time.monotonic() + 5
            while not handle.exists() and time.monotonic() < deadline:
                time.sleep(0.02)
            assert handle.exists(), "review handle was not published"
            started = json.loads(handle.read_text())
            assert started["state"] == "server-ready"
            assert started["readiness"]["server"] == "ready"
            assert started["operation_id"]
            assert started["build"]["directory"] == str(build.resolve())
            assert len(started["build"]["sha256"]) == 64
            assert started["cleanup_action"][-1] == str(handle.resolve())
            with urllib.request.urlopen(started["url"]) as response:
                assert response.status == 200

            status = subprocess.run(
                [sys.executable, str(REPOSITORY_ROOT / "scripts/serve_web.py"),
                 "--status-handle", str(handle)],
                check=True,
                capture_output=True,
                text=True,
            )
            assert json.loads(status.stdout)["running"] is True

            (build / "late-file").write_text("invalidates immutable identity")
            invalidated = subprocess.run(
                [sys.executable, str(REPOSITORY_ROOT / "scripts/serve_web.py"),
                 "--status-handle", str(handle)],
                check=True,
                capture_output=True,
                text=True,
            )
            assert json.loads(invalidated.stdout)["state"] == "inputs-invalidated"

            subprocess.run(started["cleanup_action"], check=True, capture_output=True)
            process.wait(timeout=5)
            assert json.loads(handle.read_text())["state"] == "stopped"
            operation_records = [
                json.loads(line)
                for path in (root / "logs/operations").glob("**/*.jsonl")
                for line in path.read_text().splitlines()
                if f'"operation_id": "{started["operation_id"]}"' in line
            ]
            assert operation_records[-1]["event"] == "operation.finished"
            assert operation_records[-1]["outcome"] == "inputs-invalidated"
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


if __name__ == "__main__":
    main()
