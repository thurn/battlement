#!/usr/bin/env python3

"""Hold Ditto build-cache leases across separate CI subprocesses."""

from __future__ import annotations

from dataclasses import dataclass
import json
import os
from pathlib import Path
import signal
import subprocess
from threading import Lock
import time


@dataclass
class RetainedBuild:
    """One prepared build protected by its still-running producer."""

    sample: str
    process: subprocess.Popen[str]
    release_fd: int
    output: Path
    stdout: Path
    stderr: Path


class DittoBuildLeases:
    """Own exact build subprocesses until every no-build consumer finishes."""

    def __init__(
        self, repository: Path, binary: Path, cache_root: Path, evidence_root: Path,
    ) -> None:
        self.repository = repository
        self.binary = binary
        self.cache_root = cache_root.resolve()
        self.evidence_root = evidence_root
        self._builds: list[RetainedBuild] = []
        self._lock = Lock()

    def prepare(self, sample: str) -> dict[str, object]:
        """Prepare one player and retain its active lock in the producer process."""
        self.evidence_root.mkdir(parents=True, exist_ok=True)
        output = self.evidence_root / f"{sample}.json"
        stdout = self.evidence_root / f"{sample}.stdout.log"
        stderr = self.evidence_root / f"{sample}.stderr.log"
        if output.exists():
            raise RuntimeError(f"Ditto build evidence already exists: {output}")
        read_fd, write_fd = os.pipe()
        environment = os.environ.copy()
        environment["DITTO_CACHE_ROOT"] = str(self.cache_root)
        try:
            process = subprocess.Popen(
                [
                    str(self.binary),
                    "--config", f"samples/{sample}/ditto.toml",
                    "build", "--profile", "macos", "--json",
                    "--output", str(output),
                    "--retain-until-fd-closed", str(read_fd),
                ],
                cwd=self.repository,
                env=environment,
                pass_fds=(read_fd,),
                process_group=0,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
        except BaseException:
            os.close(write_fd)
            raise
        finally:
            os.close(read_fd)
        retained = RetainedBuild(sample, process, write_fd, output, stdout, stderr)
        try:
            result = self._wait_until_ready(retained)
            self._validate_result(result)
            if process.poll() is not None:
                raise RuntimeError(f"{sample} build exited without retaining its cache lease")
        except BaseException:
            self._release(retained, require_success=False)
            raise
        with self._lock:
            self._builds.append(retained)
        return result

    def assert_healthy(self) -> None:
        """Reject execution if any producer stopped protecting its prepared build."""
        with self._lock:
            stopped = [build.sample for build in self._builds if build.process.poll() is not None]
        if stopped:
            raise RuntimeError(
                "prepared Ditto build lease ended before execution: " + ", ".join(stopped)
            )

    def close(self) -> None:
        """Release only this invocation's leases and reap their exact processes."""
        with self._lock:
            builds = self._builds
            self._builds = []
        failures = []
        for build in builds:
            try:
                self._release(build, require_success=True)
            except Exception as error:  # noqa: BLE001 - release every owned lease
                failures.append(str(error))
        if failures:
            raise RuntimeError("; ".join(failures))

    def _wait_until_ready(self, build: RetainedBuild) -> dict[str, object]:
        while True:
            if build.output.is_file():
                try:
                    return json.loads(build.output.read_text(encoding="utf-8"))
                except json.JSONDecodeError:
                    pass
            if build.process.poll() is not None:
                captured_stdout, captured_stderr = build.process.communicate()
                build.stdout.write_text(captured_stdout, encoding="utf-8")
                build.stderr.write_text(captured_stderr, encoding="utf-8")
                raise subprocess.CalledProcessError(
                    build.process.returncode,
                    build.process.args,
                    output=captured_stdout,
                    stderr=captured_stderr,
                )
            time.sleep(0.05)

    def _validate_result(self, result: dict[str, object]) -> None:
        fingerprint = result.get("build_fingerprint")
        if not isinstance(fingerprint, str) or len(fingerprint) != 64:
            raise RuntimeError("Ditto build result omitted its exact fingerprint")
        expected = self.cache_root / "builds" / "entries" / fingerprint
        player = result.get("player_path")
        if not isinstance(player, str) or Path(player).resolve() != expected:
            raise RuntimeError("Ditto build result is outside the selected cache")
        if not expected.is_dir():
            raise RuntimeError("Ditto build result was published without its cache entry")

    def _release(self, build: RetainedBuild, *, require_success: bool) -> None:
        try:
            os.close(build.release_fd)
        except OSError:
            pass
        try:
            captured_stdout, captured_stderr = build.process.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            self._signal(build.process, signal.SIGTERM)
            try:
                captured_stdout, captured_stderr = build.process.communicate(timeout=3)
            except subprocess.TimeoutExpired:
                self._signal(build.process, signal.SIGKILL)
                captured_stdout, captured_stderr = build.process.communicate()
        build.stdout.write_text(captured_stdout, encoding="utf-8")
        build.stderr.write_text(captured_stderr, encoding="utf-8")
        if require_success and build.process.returncode != 0:
            raise RuntimeError(
                f"{build.sample} retained build process exited with "
                f"{build.process.returncode}: {captured_stderr.strip()}"
            )

    @staticmethod
    def _signal(process: subprocess.Popen[str], selected_signal: signal.Signals) -> None:
        try:
            os.killpg(process.pid, selected_signal)
        except ProcessLookupError:
            pass
