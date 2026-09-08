"""Validate a complete prepared site locally through the configured Playwright MCP."""

from __future__ import annotations

from functools import partial
import hashlib
import http.server
import json
from pathlib import Path
from threading import Thread
import time
import uuid
import subprocess

from platform_support import user_cache_path

from playwright_mcp import PlaywrightMcp
from serve_web import IsolatedHandler


def contracts(repository: Path, samples: list[str]) -> dict[str, str]:
    """Require a declared browser interaction for every shipped sample."""
    selected = {}
    for sample in samples:
        path = repository / "web/checks" / f"{sample}.js"
        if not path.is_file():
            raise RuntimeError(f"Missing Web compatibility contract for {sample}: {path}")
        selected[sample] = path.read_text()
    return selected


def site_digest(site: Path) -> str:
    """Bind browser results to the complete prepared site's exact bytes."""
    checksum = hashlib.sha256()
    for path in sorted(site.rglob("*")):
        if path.is_symlink():
            raise RuntimeError(f"Prepared site contains a symbolic link: {path}")
        if not path.is_file():
            continue
        checksum.update(path.relative_to(site).as_posix().encode() + b"\0")
        with path.open("rb") as source:
            checksum.update(hashlib.file_digest(source, "sha256").digest())
    return checksum.hexdigest()


def check_site(
    repository: Path, site: Path, samples: list[str], revision: str,
    *, artifact_root: Path | None = None,
) -> Path:
    """Run each declared interaction, retain failures, and stop owned services."""
    selected = contracts(repository, samples)
    shared = (repository / "web/checks/shared.js").read_text()
    identity = site_digest(site)
    artifact_root = artifact_root or Path(user_cache_path("Battlement", "web-compatibility")) / str(uuid.uuid4())
    artifact_root.mkdir(parents=True, mode=0o700)
    report = {"schema": 1, "revision": revision, "site_sha256": identity,
              "started_at": time.time(), "status": "failed", "samples": [],
              "source_index_sha256": hashlib.sha256(subprocess.check_output(
                  ["git", "--no-optional-locks", "ls-files", "--stage", "-z"], cwd=repository,
              )).hexdigest(),
              "shared_contract_sha256": hashlib.sha256(
                  shared.encode()).hexdigest()}
    handler = partial(IsolatedHandler, directory=str(site))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = Thread(target=server.serve_forever, name="web-compatibility-server")
    thread.start()
    try:
        with PlaywrightMcp(timeout=360) as client:
            for sample, source in selected.items():
                url = f"http://127.0.0.1:{server.server_port}/{sample}/"
                record = {"sample": sample, "url": url, "status": "failed",
                          "contract_sha256": hashlib.sha256(source.encode()).hexdigest()}
                report["samples"].append(record)
                try:
                    record["evidence"] = client.call("browser_run_code_unsafe", {
                        "code": _check_code(source, url, str(artifact_root / sample),
                                            shared),
                    })
                    record["status"] = "passed"
                except Exception as error:
                    record["error"] = str(error)
                    raise
        if site_digest(site) != identity:
            raise RuntimeError("Prepared site changed during browser validation")
        report["status"] = "passed"
    except BaseException as error:
        report["error"] = str(error)
        raise
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
        report["finished_at"] = time.time()
        (artifact_root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return artifact_root / "result.json"


def _check_code(contract: str, url: str, evidence_prefix: str, shared: str) -> str:
    return """async (page) => {
      const logs = [], failures = [], canceledRequests = [], startupProgress = [];
      let initialized = false;
      const onConsole = message => {
        if (!message.text().includes("battlement.frame.slow")) logs.push(message.text().slice(0, 2000));
        if (logs.length > 200) logs.shift();
        const text = message.text();
        if (text.includes('battlement.host.connected')) initialized = true;
        const loading = !initialized && /^(still waiting on run dependencies:|dependency: (?:dataUrl|loading-workers)|\\(end of list\\))$/.test(text.trim());
        if (loading) startupProgress.push(text);
        else if (message.type() === 'error') failures.push(text.slice(0, 2000));
      };
      const onError = error => failures.push(String(error));
      const onDialog = dialog => {
        failures.push(`Browser dialog: ${dialog.message()}`);
        void dialog.dismiss();
      };
      const onRequest = request => {
        const detail = `${request.url()}: ${request.failure()?.errorText}`;
        // Unity cancels duplicate Addressables fetches when its cache wins.
        if (request.failure()?.errorText === 'net::ERR_ABORTED') canceledRequests.push(detail);
        else failures.push(detail);
      };
      page.on('console', onConsole);
      page.on('pageerror', onError);
      page.on('dialog', onDialog);
      page.on('requestfailed', onRequest);
      try {
        page.setDefaultTimeout(30000);
        const result = await (CONTRACT)(page, {
          url: URL_VALUE, evidencePrefix: EVIDENCE_VALUE, logs, start: SHARED_VALUE,
          async waitForLog(pattern) {
            if (logs.some(line => line.includes(pattern))) return;
            await page.waitForEvent('console', {
              predicate: message => message.text().includes(pattern), timeout: 90000
            });
          }
        });
        if (!result || !result.interaction || result.assertions < 1) {
          throw new Error('The sample contract omitted its interaction/assertion result');
        }
        if (failures.length) throw new Error(failures.join('\\n'));
        return { ...result, status: 'passed', console: logs, canceledRequests, startupProgress };
      } catch (error) {
        await page.screenshot({ path: EVIDENCE_VALUE + '-failed.png', fullPage: true, timeout: 10000 }).catch(() => {});
        throw new Error(`${error}\\nConsole: ${logs.join('\\n')}\\nNetwork/errors: ${failures.join('\\n')}`);
      } finally {
        page.off('console', onConsole);
        page.off('pageerror', onError);
        page.off('dialog', onDialog);
        page.off('requestfailed', onRequest);
      }
    }""".replace("SHARED_VALUE", shared).replace("CONTRACT", contract).replace("URL_VALUE", json.dumps(url)).replace("EVIDENCE_VALUE", json.dumps(evidence_prefix))
