#!/usr/bin/env python3
"""Serve a Unity Web build with the isolation headers required by pthreads."""

import argparse
import http.server


class IsolatedHandler(http.server.SimpleHTTPRequestHandler):
    def guess_type(self, path):
        if path.endswith(".wasm.unityweb"):
            return "application/wasm"
        if path.endswith((".framework.js.unityweb", ".js.unityweb")):
            return "application/javascript"
        if path.endswith(".unityweb"):
            return "application/octet-stream"
        return super().guess_type(path)

    def end_headers(self):
        if self.path.partition("?")[0].endswith(".unityweb"):
            self.send_header("Content-Encoding", "gzip")
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cross-Origin-Resource-Policy", "same-origin")
        super().end_headers()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--directory", required=True)
    parser.add_argument("--port", required=True, type=int)
    args = parser.parse_args()
    handler = lambda *handler_args, **kwargs: IsolatedHandler(
        *handler_args, directory=args.directory, **kwargs
    )
    server = http.server.ThreadingHTTPServer(("127.0.0.1", args.port), handler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
