#!/usr/bin/env python3

"""Verify that the local server labels compressed Unity artifacts correctly."""

from __future__ import annotations

import functools
from http.server import ThreadingHTTPServer
from pathlib import Path
import sys
import tempfile
import threading
import urllib.request


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

import serve_web  # noqa: E402


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-web-server-test.") as temporary:
        root = Path(temporary)
        artifacts = {
            "sample.data.unityweb": "application/octet-stream",
            "sample.framework.js.unityweb": "application/javascript",
            "sample.wasm.unityweb": "application/wasm",
        }
        for name in artifacts:
            (root / name).write_bytes(b"\x1f\x8bfixture")

        handler = functools.partial(serve_web.IsolatedHandler, directory=root)
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


if __name__ == "__main__":
    main()
