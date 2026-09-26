#!/usr/bin/env python3
"""Exercise isolated MCP session ownership and both response encodings over HTTP."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from threading import Event, Thread
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from playwright_mcp import PlaywrightMcp
import resource_slots


class Fixture(BaseHTTPRequestHandler):
    events = []
    heartbeat = Event()
    deleted = Event()

    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.wfile.write(b'data: {"jsonrpc":"2.0","id":"heartbeat","method":"ping"}\n\n')
        self.wfile.flush()
        self.deleted.wait(10)


    def log_message(self, *_args):
        pass

    def do_DELETE(self):
        assert self.headers["Mcp-Session-Id"] == "fixture-client"
        self.events.append("deleted")
        self.deleted.set()
        self.send_response(204)
        self.end_headers()

    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if request.get("id") == "heartbeat":
            assert request["result"] == {}
            self.heartbeat.set()
            self.send_response(202)
            self.end_headers()
            return
        method = request["method"]
        self.events.append(method)
        if method != "initialize":
            assert self.headers["Mcp-Session-Id"] == "fixture-client"
        if method == "notifications/initialized":
            self.send_response(202)
            self.end_headers()
            return
        if method == "initialize":
            result = {"protocolVersion": "2025-03-26", "capabilities": {}, "serverInfo": {"name": "fixture", "version": "1"}}
        elif method == "tools/list":
            result = {"tools": [{"name": name} for name in ("browser_close", "browser_evaluate", "failed")]}
        elif request["params"]["name"] == "failed":
            result = {"isError": True, "content": [{"type": "text", "text": "intentional fixture failure"}]}
        else:
            result = {"content": [{"type": "text", "text": "ok"}]}
        response = json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result})
        if method == "initialize" and self.path == "/initialize-fails":
            response = json.dumps({"jsonrpc": "2.0", "id": request["id"], "error": "intentional initialization failure"})
        self.send_response(200)
        self.send_header("Mcp-Session-Id", "fixture-client")
        if method == "tools/call":
            self.send_header("Content-Type", "text/event-stream")
            response = 'data: {"jsonrpc":"2.0","method":"notifications/progress"}\n\n' + "data: " + response + "\n\n"
        else:
            self.send_header("Content-Type", "application/json")
        payload = response.encode()
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)


def verify_admission(endpoint, locks):
    environment = {**os.environ, "BATTLEMENT_RESOURCE_SLOTS": str(locks),
                   "PYTHONPATH": str(Path(__file__).resolve().parents[1])}
    source = """
import sys
from playwright_mcp import PlaywrightMcp
try:
    with PlaywrightMcp(sys.argv[1]) as client:
        client.call('failed', {})
except RuntimeError as error:
    assert 'intentional' in str(error), str(error)
else:
    raise AssertionError('expected fixture failure')
"""
    held = resource_slots.playwright_capacity_lease().acquire()
    child = subprocess.Popen([sys.executable, "-c", source, endpoint], env=environment)
    try:
        deadline = time.monotonic() + 5
        while not list(locks.glob(".playwright-browser.queue.*.lock")):
            assert child.poll() is None, "waiting client exited unexpectedly"
            if time.monotonic() >= deadline:
                raise AssertionError("second client did not queue on browser admission")
            time.sleep(.01)
        assert Fixture.events.count("initialize") == 1, "waiting client initialized before admission"
        # Browser ownership and its waiters must leave the entire compiler budget usable.
        subprocess.run([
            sys.executable, "-c",
            "from resource_slots import compiler_maintenance_lease\nwith compiler_maintenance_lease(): pass",
        ], env=environment, check=True, timeout=5)
        assert child.poll() is None
        held.close()
        assert child.wait(timeout=10) == 0
    finally:
        held.close()
        if child.poll() is None:
            child.terminate()
            child.wait(timeout=5)
    # A failed body and a failed initialization must both allow the next client through.
    with resource_slots.SlotLease(locks, "browser", 2, 2):
        for address in (endpoint.replace("/mcp", "/initialize-fails"), endpoint):
            subprocess.run([sys.executable, "-c", source, address], env=environment, check=True, timeout=10)
    assert not list(locks.glob(".playwright-browser.queue.*.lock"))


def verify_transport(locks):
    resource_slots.GLOBAL_RESOURCE_ROOT = locks
    server = ThreadingHTTPServer(("127.0.0.1", 0), Fixture)
    thread = Thread(target=server.serve_forever)
    thread.start()
    try:
        with PlaywrightMcp(f"http://127.0.0.1:{server.server_port}/mcp") as client:
            assert Fixture.heartbeat.wait(2), "Client did not answer the server heartbeat"
            assert client.call("browser_evaluate", {"function": "() => true"})["content"][0]["text"] == "ok"
            for name in ("missing", "failed"):
                try:
                    client.call(name, {})
                    raise AssertionError("Invalid or failed tool was accepted")
                except RuntimeError:
                    pass
            client.close()
        assert Fixture.events == ["initialize", "notifications/initialized", "tools/list", "tools/call", "tools/call", "tools/call", "deleted"]
        verify_admission(f"http://127.0.0.1:{server.server_port}/mcp", locks)
    finally:
        server.shutdown()
        server.server_close()
        thread.join()

def main():
    with tempfile.TemporaryDirectory(prefix="battlement-playwright-admission-") as temporary:
        verify_transport(Path(temporary))
    print("Playwright MCP transport and cross-process admission checks passed.")


if __name__ == "__main__":
    main()
