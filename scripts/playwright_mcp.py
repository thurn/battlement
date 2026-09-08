"""An isolated client of the configured singleton Playwright MCP service."""

from __future__ import annotations

import json
import http.client
import socket
import subprocess
from threading import Event, Thread
import urllib.error
import urllib.parse
import urllib.request


ENDPOINT = "http://localhost:8931/mcp"


class PlaywrightMcp:
    """Own one MCP session without launching or sharing a browser context."""

    def __init__(self, endpoint: str = ENDPOINT, timeout: float = 60) -> None:
        self.endpoint = endpoint
        self.timeout = timeout
        self.session_id: str | None = None
        self.protocol = "2025-03-26"
        self.sequence = 0
        self.tools: dict[str, dict] = {}
        self.stream: http.client.HTTPConnection | None = None
        self.listener: Thread | None = None
        self.stopping = Event()
        self.stream_error: Exception | None = None

    def __enter__(self) -> PlaywrightMcp:
        try:
            arguments = {
                "protocolVersion": self.protocol,
                "capabilities": {},
                "clientInfo": {"name": "battlement-web-validation", "version": "1"},
            }
            try:
                initialized = self.request("initialize", arguments)
            except urllib.error.URLError as error:
                if self.endpoint != ENDPOINT or not isinstance(error.reason, ConnectionRefusedError):
                    raise
                subprocess.run(["playwright-mcp-service", "start"], check=True)
                initialized = self.request("initialize", arguments)
            if not self.session_id:
                raise RuntimeError("Playwright MCP did not establish an isolated session")
            self.protocol = initialized["protocolVersion"]
            self.request("notifications/initialized", {}, notification=True)
            self._listen()
            listing = self.request("tools/list", {})
            self.tools = {tool["name"]: tool for tool in listing["tools"]}
            return self
        except BaseException as error:
            self._close_preserving(error)
            raise

    def request(self, method: str, params: dict, *, notification: bool = False) -> dict:
        """Read the response for this request; an observation timeout does not retry it."""
        self.sequence += 1
        message = {"jsonrpc": "2.0", "method": method, "params": params}
        if not notification:
            message["id"] = self.sequence
        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream",
            "MCP-Protocol-Version": self.protocol,
        }
        if self.session_id:
            headers["Mcp-Session-Id"] = self.session_id
        request = urllib.request.Request(self.endpoint, json.dumps(message).encode(), headers)
        with urllib.request.urlopen(request, timeout=self.timeout) as response:
            self.session_id = response.headers.get("Mcp-Session-Id", self.session_id)
            if notification:
                return {}
            if response.headers.get_content_type() == "text/event-stream":
                result = self._read_event(response, self.sequence)
            else:
                result = json.load(response)
        if result.get("id") != self.sequence:
            raise RuntimeError("Playwright MCP response has the wrong request identity")
        if "error" in result:
            raise RuntimeError(f"Playwright MCP: {str(result['error'])[:4000]}")
        return result["result"]

    def _listen(self) -> None:
        # Playwright's session heartbeat is a server request on the GET stream.
        endpoint = urllib.parse.urlsplit(self.endpoint)
        connection_type = http.client.HTTPSConnection if endpoint.scheme == "https" else http.client.HTTPConnection
        self.stream = connection_type(endpoint.hostname, endpoint.port, timeout=self.timeout)
        self.stream.request("GET", endpoint.path or "/", headers={
            "Accept": "text/event-stream", "Mcp-Session-Id": self.session_id,
            "MCP-Protocol-Version": self.protocol,
        })
        response = self.stream.getresponse()
        if response.status != 200:
            raise RuntimeError(f"Playwright event stream returned HTTP {response.status}")

        def consume() -> None:
            data = []
            try:
                for raw in response:
                    if self.stopping.is_set():
                        break
                    line = raw.decode("utf-8").rstrip("\r\n")
                    if line.startswith("data:"):
                        data.append(line[5:].lstrip())
                    elif not line and data:
                        message = json.loads("\n".join(data))
                        data = []
                        if "method" in message and "id" in message:
                            self._answer(message)
            except Exception as error:
                if not self.stopping.is_set():
                    self.stream_error = error

        self.listener = Thread(target=consume, name="playwright-mcp-events", daemon=True)
        self.listener.start()

    def _answer(self, message: dict) -> None:
        reply = {"jsonrpc": "2.0", "id": message["id"]}
        if message["method"] == "ping":
            reply["result"] = {}
        else:
            reply["error"] = {"code": -32601, "message": "Unsupported client request"}
        request = urllib.request.Request(self.endpoint, json.dumps(reply).encode(), {
            "Content-Type": "application/json", "Accept": "application/json, text/event-stream",
            "Mcp-Session-Id": self.session_id, "MCP-Protocol-Version": self.protocol,
        })
        with urllib.request.urlopen(request, timeout=min(5, self.timeout)):
            pass

    @staticmethod
    def _read_event(response, request_id: int) -> dict:
        data = []
        for raw in response:
            line = raw.decode("utf-8").rstrip("\r\n")
            if line.startswith("data:"):
                data.append(line[5:].lstrip())
            elif not line and data:
                message = json.loads("\n".join(data))
                data = []
                if message.get("id") == request_id:
                    return message
        raise RuntimeError("Playwright MCP response ended before the requested result")

    def call(self, name: str, arguments: dict) -> dict:
        """Invoke an advertised tool, preserving structured failure outcomes."""
        if self.stream_error:
            raise RuntimeError(f"Playwright event stream failed: {self.stream_error}")
        if name not in self.tools:
            raise RuntimeError(f"Configured Playwright MCP does not provide {name}")
        result = self.request("tools/call", {"name": name, "arguments": arguments})
        if result.get("isError"):
            detail = "\n".join(item.get("text", "") for item in result.get("content", []))
            if len(detail) > 16000:
                detail = detail[:2000] + "\n[earlier console output omitted]\n" + detail[-14000:]
            raise RuntimeError(f"Playwright {name}: {detail}")
        return result

    def close(self) -> None:
        """Close only this client's page and MCP session; leave the singleton running."""
        try:
            if "browser_close" in self.tools:
                self.call("browser_close", {})
        finally:
            try:
                self.stopping.set()
                if self.session_id:
                    request = urllib.request.Request(self.endpoint, method="DELETE", headers={
                        "Mcp-Session-Id": self.session_id, "MCP-Protocol-Version": self.protocol,
                    })
                    try:
                        with urllib.request.urlopen(request, timeout=min(5, self.timeout)):
                            pass
                    except urllib.error.HTTPError as error:
                        if error.code != 404:
                            raise
            finally:
                self.session_id = None
                if self.stream:
                    if self.stream.sock:
                        try:
                            self.stream.sock.shutdown(socket.SHUT_RDWR)
                        except OSError:
                            pass
                    self.stream.close()
                if self.listener:
                    self.listener.join(timeout=6)
                    if self.listener.is_alive():
                        raise RuntimeError("Playwright event listener did not stop")

    def _close_preserving(self, error: BaseException | None) -> None:
        try:
            self.close()
        except Exception as cleanup_error:
            if error is None:
                raise
            error.add_note(f"Playwright session cleanup also failed: {cleanup_error}")

    def __exit__(self, _kind, error, _traceback) -> None:
        self._close_preserving(error)
