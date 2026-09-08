#!/usr/bin/env python3
"""Serve an immutable Unity Web build and manage its durable review handle."""

from __future__ import annotations

import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import time

import process_identity
import operation_log


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


def build_digest(directory: Path) -> str:
    """Identify the exact immutable build tree served by a review process."""
    digest = hashlib.sha256()
    for path in sorted(directory.rglob("*")):
        if path.is_symlink():
            raise RuntimeError(f"Review builds may not contain symlinks: {path}")
        if not path.is_file():
            continue
        relative = path.relative_to(directory).as_posix().encode()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
    return digest.hexdigest()


def read_handle(path: Path) -> dict:
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(f"Could not read review handle {path}: {error}") from error
    if value.get("schema") != 1 or not isinstance(value.get("process"), dict):
        raise RuntimeError(f"Unsupported review handle: {path}")
    return value


def write_handle(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w") as output:
            json.dump(value, output, indent=2, sort_keys=True)
            output.write("\n")
        os.replace(temporary, path)
    finally:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass


def handle_status(path: Path) -> dict:
    value = read_handle(path)
    running = process_identity.matches(value["process"])
    state = "server-ready" if running else "stopped"
    directory = Path(value["build"]["directory"])
    if running:
        try:
            if build_digest(directory) != value["build"]["sha256"]:
                state = "inputs-invalidated"
        except OSError:
            state = "inputs-invalidated"
    return value | {"state": state, "running": running}


def stop_handle(path: Path) -> dict:
    value = read_handle(path)
    expected = value["process"]
    if process_identity.matches(expected):
        os.kill(expected["pid"], signal.SIGTERM)
        deadline = time.monotonic() + 5
        while process_identity.matches(expected) and time.monotonic() < deadline:
            time.sleep(0.05)
        if process_identity.matches(expected):
            os.kill(expected["pid"], signal.SIGKILL)
            deadline = time.monotonic() + 2
            while process_identity.matches(expected) and time.monotonic() < deadline:
                time.sleep(0.05)
        if process_identity.matches(expected):
            raise RuntimeError("Review server did not stop")
    value.update(state="stopped", running=False, stopped_at=time.time())
    write_handle(path, value)
    return value


def serve(directory: Path, port: int, handle_path: Path | None) -> None:
    directory = directory.resolve()
    if not directory.is_dir():
        raise RuntimeError(f"Review build is not a directory: {directory}")
    with operation_log.Operation(
        Path(__file__).resolve().parent.parent,
        "Web review service",
        metadata={"build_directory": str(directory), "requested_port": port},
    ) as operation:
        _serve(directory, port, handle_path, operation)


def _serve(
    directory: Path, port: int, handle_path: Path | None,
    operation: operation_log.Operation,
) -> None:
    digest_started = time.monotonic_ns()
    digest = build_digest(directory)
    operation.event(
        "review.build_identified", source_manifest_sha256=digest,
        duration_ms=round((time.monotonic_ns() - digest_started) / 1_000_000),
    )
    handler = lambda *handler_args, **kwargs: IsolatedHandler(
        *handler_args, directory=directory, **kwargs
    )
    server = http.server.ThreadingHTTPServer(("127.0.0.1", port), handler)
    server.timeout = 0.25
    stopping = False

    def request_stop(_number, _frame) -> None:
        nonlocal stopping
        stopping = True

    previous = {
        number: signal.signal(number, request_stop)
        for number in (signal.SIGINT, signal.SIGTERM)
    }
    handle = {
        "schema": 1,
        "state": "server-ready",
        "running": True,
        "service_label": "battlement-web-review",
        "process": process_identity.identity(),
        "operation_id": operation.id,
        "build": {"directory": str(directory), "sha256": digest},
        "url": f"http://127.0.0.1:{server.server_port}/",
        "port": server.server_port,
        "public_url": None,
        "log_path": None,
        "readiness": {
            "server": "ready",
            "assets": "not-checked",
            "player": "not-checked",
            "interaction": "not-requested",
        },
        "cleanup_action": [
            sys.executable,
            str(Path(__file__).resolve()),
            "--stop-handle",
            str(handle_path.resolve()) if handle_path else None,
        ],
        "started_at": time.time(),
    }
    if handle_path:
        write_handle(handle_path, handle)
    else:
        handle["cleanup_action"] = None
        print(handle["url"], flush=True)
    operation.event(
        "review.ready", handle_path=str(handle_path.resolve()) if handle_path else None,
        port=server.server_port, source_manifest_sha256=digest,
        cleanup_action=handle["cleanup_action"], readiness=handle["readiness"],
    )
    try:
        while not stopping:
            server.handle_request()
    finally:
        server.server_close()
        for number, old_handler in previous.items():
            signal.signal(number, old_handler)
        if handle_path:
            current_digest = None
            try:
                current_digest = build_digest(directory)
            except OSError:
                pass
            if current_digest != digest:
                operation.event(
                    "review.inputs_invalidated", expected_sha256=digest,
                    actual_sha256=current_digest,
                )
                operation.finish(
                    "inputs-invalidated", 0, expected_sha256=digest,
                    actual_sha256=current_digest,
                )
            handle.update(state="stopped", running=False, stopped_at=time.time())
            write_handle(handle_path, handle)


def parse_arguments(arguments: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--status-handle", type=Path)
    action.add_argument("--stop-handle", type=Path)
    parser.add_argument("--directory", type=Path)
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--handle", type=Path)
    return parser.parse_args(arguments)


def main() -> None:
    args = parse_arguments()
    try:
        if args.status_handle:
            print(json.dumps(handle_status(args.status_handle), sort_keys=True))
        elif args.stop_handle:
            print(json.dumps(stop_handle(args.stop_handle), sort_keys=True))
        elif args.directory:
            serve(args.directory, args.port, args.handle)
        else:
            raise RuntimeError("--directory is required when starting a review server")
    except (OSError, RuntimeError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error


if __name__ == "__main__":
    main()
