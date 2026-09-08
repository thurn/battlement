#!/usr/bin/env python3
"""Exercise isolated MCP session ownership and both response encodings over HTTP."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import sys
from threading import Event, Thread

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from playwright_mcp import PlaywrightMcp


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


def main():
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
        assert Fixture.events == ["initialize", "notifications/initialized", "tools/list", "tools/call", "tools/call", "tools/call", "deleted"]
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
    print("Playwright MCP transport checks passed.")


if __name__ == "__main__":
    main()
