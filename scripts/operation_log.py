"""Private operation events and explicit subprocess correlation for local tooling."""

from __future__ import annotations

from contextvars import ContextVar
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import subprocess
from threading import Lock
import time
import uuid

import process_identity


_current: ContextVar[Operation | None] = ContextVar('battlement_operation', default=None)
_CONTEXT_VARIABLES = {
    'task_id': 'CODEX_THREAD_ID', 'turn_id': 'CODEX_TURN_ID',
    'candidate_id': 'TOLLGATE_ITEM_ID', 'buildset_id': 'TOLLGATE_BUILDSET_ID',
    'generation_id': 'TOLLGATE_VALIDATION_GENERATION_ID',
}


def current() -> Operation | None:
    """Return this thread's innermost operation."""
    return _current.get()


def child_environment(environment: dict[str, str] | None = None) -> dict[str, str]:
    """Preserve the caller environment, including Unity's private Git index."""
    import resource_slots
    result = resource_slots.capacity_environment(environment)
    operation = current()
    if operation is not None:
        result['BATTLEMENT_PARENT_OPERATION_ID'] = operation.id
        result['BATTLEMENT_LOG_ROOT'] = str(operation.log_root)
        result['BATTLEMENT_ROOT_OPERATION_ID'] = operation.context['root_operation_id']
    return result


def run(command: list[str], *, cwd: Path, environment: dict[str, str] | None = None) -> None:
    """Observe one real child process while preserving subprocess.run failure semantics."""
    operation = current()
    started = time.monotonic_ns()
    with subprocess.Popen(command, cwd=cwd, env=child_environment(environment)) as child:
        observed = process_identity.identity(child.pid)
        if operation:
            operation.event('process.started', process=observed,
                            executable=Path(command[0]).name,
                            containment={'kind': 'direct-child', 'controller': operation.process})
        try:
            result = child.wait()
        except BaseException:
            child.kill()
            child.wait()
            raise
        finally:
            if operation:
                operation.event('process.finished', process=observed, exit_code=child.returncode,
                                duration_ms=round((time.monotonic_ns() - started) / 1_000_000))
        if result:
            raise subprocess.CalledProcessError(result, command)


def _safe(value, key=''):
    if re.search(r'password|secret|(?:^|_)token$|cookie|authorization|^(argv|environment)$', key, re.I):
        return '[redacted]'
    if isinstance(value, dict):
        return {str(k): _safe(v, str(k)) for k, v in list(value.items())[:40]}
    if isinstance(value, (list, tuple)):
        return [_safe(item) for item in value[:30]]
    if isinstance(value, str):
        if key == 'repository_url':
            value = re.sub(r'^[^/@]+:[^/@]+@', '[redacted]@', value)
        value = re.sub(r'(https?://)[^/@\s]+:[^/@\s]+@', r'\1[redacted]@', value)
        value = re.sub(r'(?i)([?&](?:token|key|password|secret)=)[^&\s]+', r'\1[redacted]', value)
        return value[:1500]
    return value if value is None or isinstance(value, (int, float, bool)) else type(value).__name__


class Operation:
    """One observed operation with a stable identity and an explicit terminal event."""

    def __init__(self, repository: Path, name: str, *, operation_id: str | None = None,
                 parent_id: str | None = None, context: dict | None = None,
                 metadata: dict | None = None, log_root: Path | None = None) -> None:
        self.id = operation_id or str(uuid.uuid4())
        uuid.UUID(self.id)
        parent = current()
        self.parent_id = parent_id or (parent.id if parent else os.environ.get('BATTLEMENT_PARENT_OPERATION_ID'))
        self.log_root = log_root or (parent.log_root if parent else Path(os.environ.get('BATTLEMENT_LOG_ROOT', Path.home() / 'battlement/.logs')))
        self.started_ns = time.monotonic_ns()
        self.process = process_identity.identity()
        self.context = dict(context if context is not None else self._context(repository))
        self.context.setdefault('root_operation_id', os.environ.get('BATTLEMENT_ROOT_OPERATION_ID', self.id))
        self.name = name
        self.path = None
        self.file = None
        self.lock = Lock()
        self.closed = False
        self.token = None
        try:
            directory = self.log_root / 'operations' / datetime.now(timezone.utc).date().isoformat()
            directory.mkdir(mode=0o700, parents=True, exist_ok=True)
            self.path = directory / f'{self.id}.jsonl'
            descriptor = os.open(self.path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            self.file = os.fdopen(descriptor, 'w', encoding='utf-8')
        except OSError:
            self.path = None
        self.event('operation.started', name=name, context=self.context, metadata=metadata or {},
                   process=self.process, clock_domain={'host': self.process['host'], 'boot_process': process_identity.identity(1)['birth']},
                   parent_operation_id=self.parent_id)

    @staticmethod
    def _context(repository: Path) -> dict:
        from perf_log import git_metadata
        source = git_metadata(repository)
        return {**source, 'tested_oid': None if source['dirty'] else source['head_oid'],
                **{key: os.environ.get(variable) for key, variable in _CONTEXT_VARIABLES.items()}}

    def event(self, event: str, **attributes) -> None:
        """Emit bounded diagnostic metadata; logging failure cannot change a test result."""
        record = _safe(attributes)
        record.update(schema=1, event=event, operation_id=self.id,
                      timestamp=datetime.now(timezone.utc).isoformat(), monotonic_ns=time.monotonic_ns())
        encoded = json.dumps(record, sort_keys=True)
        if len(encoded) > 12000:
            essentials = {key: record[key] for key in ('schema', 'event', 'operation_id', 'timestamp', 'monotonic_ns')}
            essentials['context'] = _safe({key: self.context.get(key) for key in (
                'repository_url', 'worktree_path', 'head_oid', 'staged_tree_oid', 'tested_oid',
                'root_operation_id', 'task_id', 'turn_id', 'candidate_id', 'buildset_id', 'generation_id')})
            encoded = json.dumps(essentials | {'details_omitted': True})
        with self.lock:
            if self.file is None:
                return
            try:
                self.file.write(encoded + '\n')
                self.file.flush()
            except OSError:
                try:
                    self.file.close()
                except OSError:
                    pass
                self.file = None

    def finish(self, outcome: str, exit_code: int | None = None, **attributes) -> None:
        """Close once; disappearance without this event remains an unknown terminal state."""
        if self.closed:
            return
        self.event('operation.finished', name=self.name, outcome=outcome, exit_code=exit_code,
                   duration_ms=round((time.monotonic_ns() - self.started_ns) / 1_000_000), **attributes)
        self.closed = True
        with self.lock:
            if self.file:
                try:
                    self.file.close()
                except OSError:
                    pass
                self.file = None

    def __enter__(self) -> Operation:
        self.token = _current.set(self)
        return self

    def __exit__(self, kind, error, traceback) -> None:
        outcome = 'passed'
        code = 0
        if isinstance(error, KeyboardInterrupt):
            code, outcome = 130, 'canceled'
        elif isinstance(error, SystemExit):
            code = error.code if isinstance(error.code, int) else 0 if error.code is None else 1
            outcome = 'passed' if code == 0 else 'failed'
        elif error is not None:
            outcome = 'failed'
            code = getattr(error, 'returncode', None)
        if isinstance(error, (OSError, subprocess.TimeoutExpired)):
            failure_kind = 'infrastructure'
        elif error is not None:
            failure_kind = 'product'
        else:
            failure_kind = None
        self.finish(outcome, code, error_type=type(error).__name__ if error else None,
                    failure_kind=failure_kind)
        if self.token is not None:
            _current.reset(self.token)
