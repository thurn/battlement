#!/usr/bin/env python3
"""Build and verify the release Rust cancellation fixture in threaded WebGL."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time

from platform_support import user_cache_path
from playwright_mcp import PlaywrightMcp
from serve_web import handle_status


REPOSITORY = Path(__file__).resolve().parent.parent
CONFIG = REPOSITORY / "fixtures/webgl-worker-proof/ditto.toml"


def run(output: Path | None = None) -> Path:
    """Build the exact release player and retain its browser proof report."""
    artifact = output or (
        Path(user_cache_path("Battlement", "engine-evidence"))
        / "engine-04-threaded-webgl.json"
    )
    artifact.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="battlement-webgl-worker-") as temporary:
        temporary_path = Path(temporary)
        build_result = temporary_path / "build.json"
        read_fd, release_fd = os.pipe()
        try:
            build_process = subprocess.Popen(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "-p",
                    "rt",
                    "--",
                    "ditto",
                    "--config",
                    str(CONFIG),
                    "build",
                    "--profile",
                    "webgl",
                    "--json",
                    "--output",
                    str(build_result),
                    "--retain-until-fd-closed",
                    str(read_fd),
                ],
                cwd=REPOSITORY,
                pass_fds=(read_fd,),
            )
        except BaseException:
            os.close(release_fd)
            raise
        finally:
            os.close(read_fd)
        try:
            build = wait_for_build(build_result, build_process)
            player = Path(build["application_path"])
            strict_pool = assert_strict_thread_pool(player)
            handle = temporary_path / "server.json"
            server = subprocess.Popen(
                [
                    sys.executable,
                    str(REPOSITORY / "scripts/serve_web.py"),
                    "--directory",
                    str(player),
                    "--handle",
                    str(handle),
                ],
                cwd=REPOSITORY,
            )
            try:
                status = wait_for_server(handle, server)
                evidence_prefix = artifact.with_suffix("")
                with PlaywrightMcp(timeout=360) as client:
                    evidence = client.call(
                        "browser_run_code_unsafe",
                        {
                            "code": browser_contract(
                                status["url"], str(evidence_prefix) + "-browser"
                            )
                        },
                    )
                report = {
                    "schema": 1,
                    "status": "passed",
                    "build": build,
                    "strict_pool": strict_pool,
                    "player_sha256": status["build"]["sha256"],
                    "source_index_sha256": hashlib.sha256(
                        subprocess.check_output(
                            ["git", "--no-optional-locks", "ls-files", "--stage", "-z"],
                            cwd=REPOSITORY,
                        )
                    ).hexdigest(),
                    "browser": evidence,
                    "finished_at": time.time(),
                }
                artifact.write_text(json.dumps(report, indent=2) + "\n")
            finally:
                if handle.is_file():
                    subprocess.run(
                        [
                            sys.executable,
                            str(REPOSITORY / "scripts/serve_web.py"),
                            "--stop-handle",
                            str(handle),
                        ],
                        cwd=REPOSITORY,
                        check=True,
                    )
                server.wait(timeout=10)
        finally:
            os.close(release_fd)
            return_code = build_process.wait(timeout=10)
            if return_code != 0 and sys.exc_info()[0] is None:
                raise subprocess.CalledProcessError(return_code, build_process.args)
    print(artifact)
    return artifact


def wait_for_build(
    output: Path, process: subprocess.Popen, timeout: float = 3600
) -> dict:
    """Wait until the retained build producer publishes its result."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if output.is_file():
            result = json.loads(output.read_text())
            if process.poll() is not None:
                raise RuntimeError("Web proof build exited without retaining its cache lease")
            return result
        if process.poll() is not None:
            raise subprocess.CalledProcessError(process.returncode, process.args)
        time.sleep(0.05)
    raise RuntimeError("Web proof build did not publish its result")


def assert_strict_thread_pool(player: Path) -> dict:
    """Verify the linked player refuses to grow past its prestarted pool."""
    frameworks = list((player / "Build").glob("*.framework.js.unityweb"))
    if len(frameworks) != 1:
        raise RuntimeError(f"Expected one generated framework, found {len(frameworks)}")
    with gzip.open(frameworks[0], "rb") as compressed:
        framework = compressed.read()
    strict_get_worker = re.search(
        rb"getNewWorker\(\)\{if\(PThread\.unusedWorkers\.length==0\)\{return\}",
        framework,
    )
    if strict_get_worker is None:
        raise RuntimeError("Generated WebGL player can allocate beyond its worker pool")
    return {"verified": True, "framework": frameworks[0].name}


def wait_for_server(handle: Path, server: subprocess.Popen, timeout: float = 20) -> dict:
    """Wait for the isolated local server's durable ready handle."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if handle.is_file():
            status = handle_status(handle)
            if status["running"]:
                return status
        if server.poll() is not None:
            raise RuntimeError(f"Web proof server exited with {server.returncode}")
        time.sleep(0.05)
    raise RuntimeError("Web proof server did not become ready")


def browser_contract(url: str, evidence_prefix: str) -> str:
    """Return the Playwright interaction for the actual Unity release player."""
    return f"""async (page) => {{
      const logs = [], failures = [];
      let step = 'page startup';
      page.on('console', message => {{
        logs.push(message.text());
        if (message.type() === 'error') failures.push(message.text());
      }});
      page.on('pageerror', error => failures.push(String(error)));
      page.setDefaultTimeout(120000);
      try {{
        await page.setViewportSize({{ width: 1280, height: 720 }});
        await page.goto({json.dumps(url)});
        const waitForLog = async value => {{
          if (logs.some(line => line.includes(value))) return;
          await page.waitForEvent('console', {{
            predicate: message => message.text().includes(value), timeout: 120000
          }});
        }};
        step = 'fixture ready';
        await waitForLog('BATTLEMENT_WEBGL_WORKER_READY');
        step = 'host connected';
        await waitForLog('battlement.host.connected');
        const threading = await page.evaluate(() => ({{
        isolated: crossOriginIsolated,
        sharedArrayBuffer: typeof SharedArrayBuffer === 'function',
        configuredThreads: window.battlementWebThreads?.threadCount || 0,
        sharedMemory: new WebAssembly.Memory({{
          initial: 1, maximum: 1, shared: true
        }}).buffer instanceof SharedArrayBuffer,
      }}));
      if (!threading.isolated || !threading.sharedArrayBuffer ||
          !threading.sharedMemory || threading.configuredThreads < 1) {{
        throw new Error('Threaded WebGL runtime facts are missing: ' + JSON.stringify(threading));
      }}
      const canvas = page.locator('#unity-canvas');
      await canvas.waitFor({{ state: 'visible' }});
      const box = await canvas.boundingBox();
      if (!box) throw new Error('Unity canvas has no bounds');
      const clickCanvas = async (x, y) => {{
        await page.mouse.click(box.x + x, box.y + y, {{ delay: 120 }});
      }};
      step = 'worker started';
      await clickCanvas(400, 360);
      await waitForLog('BATTLEMENT_WEBGL_WORKER_STARTED_OFF_UI');
      step = 'cancellation cleanup';
      await clickCanvas(65, 60);
      await waitForLog('BATTLEMENT_WEBGL_MENU_OPENED_DURING_CANCEL');
      await waitForLog('BATTLEMENT_WEBGL_WORKER_CANCELLED_CLEANED_STOPPED');
      const cancellationPanics = failures.filter(line =>
        line.includes("thread 'reactant-rules'") ||
        line.includes('panicked at')
      );
      if (cancellationPanics.length) {{
        throw new Error('Cancellation reached the panic hook: ' + cancellationPanics.join('\\n'));
      }}
      await page.screenshot({{ path: {json.dumps(evidence_prefix + '-responsive.png')} }});
      step = 'finite-pool replacement';
      await clickCanvas(65, 60);
      await waitForLog('BATTLEMENT_WEBGL_WORKER_LATEST_REPLACEMENT_ONLY');
      step = 'real panic';
      await clickCanvas(65, 60);
      await waitForLog('BATTLEMENT_WEBGL_WORKER_REAL_PANIC_FAILED');
      step = 'panic replacement';
      await clickCanvas(65, 60);
      await waitForLog('BATTLEMENT_WEBGL_WORKER_RECOVERED');
      step = 'non-joining exit';
      await clickCanvas(65, 60);
      await waitForLog('BATTLEMENT_WEBGL_WORKER_NONJOINING_EXIT_CLEANED');
      step = 'completion screenshot';
      await page.screenshot({{ path: {json.dumps(evidence_prefix + '-complete.png')} }});
      step = 'expected panic classification';
        const realPanic = failures.some(line =>
          line.includes('fixture genuine rules panic') ||
          line.includes("thread 'reactant-rules'")
        );
        if (!realPanic) throw new Error('The genuine Rust panic was not visible to the browser');
        const unexpected = failures.filter(line =>
          line && line !== 'Error' &&
          !line.includes('fixture genuine rules panic') &&
          !line.includes("thread 'reactant-rules'") &&
          !line.includes('RUST_BACKTRACE') &&
          !line.includes('BATTLEMENT_WEBGL_WORKER_REAL_PANIC_FAILED')
        );
        if (unexpected.length) throw new Error(unexpected.join('\\n'));
        return {{
          status: 'passed', threading, realPanic,
          milestones: logs.filter(line => line.includes('BATTLEMENT_WEBGL_')),
        }};
      }} catch (error) {{
        await page.screenshot({{
          path: {json.dumps(evidence_prefix + '-failed.png')}, fullPage: true
        }}).catch(() => {{}});
        const relevant = logs.filter(line =>
          line.includes('BATTLEMENT_WEBGL_') ||
          line.includes('fixture.') ||
          line.includes('battlement.session') ||
          line.includes('panic')
        );
        throw new Error(`Step: ${{step}}\\n${{error}}\\nConsole:\\n${{relevant.join('\\n')}}\\nFailures:\\n${{failures.join('\\n')}}`);
      }}
    }}"""


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


if __name__ == "__main__":
    arguments = parse_arguments()
    run(arguments.output)
