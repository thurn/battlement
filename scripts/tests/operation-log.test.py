#!/usr/bin/env python3
"""Verify real subprocess correlation, resource intervals, and private retention."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ci_steps
import operation_log
import perf_log
import process_identity
from resource_slots import SlotLease


class OperationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.logs = self.root / 'logs'
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        subprocess.run(['git', '-c', 'user.name=Test', '-c', 'user.email=test@example.com',
                        'commit', '--allow-empty', '-qm', 'fixture'], cwd=self.root, check=True)
        self.environment = patch.dict(os.environ, {'BATTLEMENT_LOG_ROOT': str(self.logs)})
        self.environment.start()
        os.environ.pop("BATTLEMENT_PARENT_OPERATION_ID", None)
        os.environ.pop("BATTLEMENT_ROOT_OPERATION_ID", None)
        self.addCleanup(self.environment.stop)

    def test_ci_child_keeps_parent_identity_and_private_index(self):
        output = self.root / 'child.json'
        trace = perf_log.CiTrace(self.root, {'full': False}, log_root=self.logs)
        ci_steps.configure(self.root, trace)
        self.addCleanup(ci_steps.configure, self.root, None)
        index = str(self.root / 'private-index')
        with patch.dict(os.environ, {'GIT_INDEX_FILE': index}):
            ci_steps.run_step('child', [sys.executable, '-c',
                "import json,os,sys;json.dump({k:os.environ.get(k) for k in "
                "['BATTLEMENT_PARENT_OPERATION_ID','BATTLEMENT_ROOT_OPERATION_ID','GIT_INDEX_FILE']},open(sys.argv[1],'w'))",
                str(output)])
        trace.finish('passed', 0)
        child = json.loads(output.read_text())
        events = [json.loads(line) for path in self.logs.glob('operations/**/*.jsonl')
                  for line in path.read_text().splitlines()]
        starts = [event for event in events if event['event'] == 'operation.started']
        self.assertEqual(len(starts), 2)
        step = next(event for event in starts if event['name'] == 'child')
        self.assertEqual(child['BATTLEMENT_PARENT_OPERATION_ID'], step['operation_id'])
        self.assertEqual(child['BATTLEMENT_ROOT_OPERATION_ID'], trace.run_id)
        self.assertEqual(child['GIT_INDEX_FILE'], index)
        self.assertEqual(step['parent_operation_id'], trace.run_id)
        self.assertTrue(step['context']['head_oid'])
        self.assertTrue(process_identity.matches(step['process']))
        process = next(event for event in events if event['event'] == 'process.started')
        terminal = next(event for event in events if event['event'] == 'process.finished')
        self.assertEqual(process['process'], terminal['process'])
        self.assertNotEqual(process['process']['pid'], os.getpid())
        self.assertEqual(terminal['exit_code'], 0)
        self.assertFalse(process_identity.matches(process['process']))

    def test_nested_ci_retains_inherited_operation_family(self):
        with operation_log.Operation(self.root, 'outer') as outer:
            with patch.dict(os.environ, operation_log.child_environment()):
                trace = perf_log.CiTrace(self.root, {}, log_root=self.logs)
                with trace.span('nested'):
                    self.assertEqual(operation_log.current().context['root_operation_id'], outer.id)
                trace.finish('passed', 0)
        related = list(self.logs.glob('operations/**/*.jsonl'))
        self.assertEqual(len(related), 3)
        perf_log.enforce_retention(self.logs, 0, {trace.path})
        self.assertTrue(all(path.exists() for path in related))
        trace.path.unlink()
        perf_log.enforce_retention(self.logs, 0)
        self.assertFalse(any(path.exists() for path in related))

    def test_resource_acquisition_release_and_failed_outcome(self):
        with self.assertRaises(subprocess.CalledProcessError):
            with operation_log.Operation(self.root, 'fixture') as operation:
                with SlotLease(self.root / 'slots', 'fixture', 1):
                    raise subprocess.CalledProcessError(7, ['fixture'])
        events = [json.loads(line) for line in operation.path.read_text().splitlines()]
        self.assertEqual([event['event'] for event in events], [
            'operation.started', 'resource.queued', 'resource.acquired',
            'resource.released', 'operation.finished'])
        self.assertEqual(events[-1]['exit_code'], 7)
        self.assertEqual(events[-1]['outcome'], 'failed')
        self.assertEqual(events[-1]['failure_kind'], 'unknown')
        self.assertGreaterEqual(events[2]['queue_duration_ms'], 0)
        self.assertGreaterEqual(events[3]['held_duration_ms'], 0)
        with SlotLease(self.root / 'slots', 'fixture', 1):
            pass

    def test_explicit_exit_and_cancel_outcomes(self):
        for error, outcome in [(SystemExit(1), 'failed'), (SystemExit(0), 'passed'),
                               (KeyboardInterrupt(), 'canceled')]:
            with self.assertRaises(type(error)):
                with operation_log.Operation(self.root, 'exit') as operation:
                    raise error
            terminal = json.loads(operation.path.read_text().splitlines()[-1])
            self.assertEqual(terminal['outcome'], outcome)

    def test_process_birth_cannot_be_replaced_by_matching_pid(self):
        expected = process_identity.identity()
        self.assertTrue(process_identity.matches(expected))
        self.assertFalse(process_identity.matches({**expected, 'birth': 'different-birth'}))
        self.assertFalse(process_identity.matches({**expected, 'birth': None}))
        child = subprocess.Popen([sys.executable, '-c', 'import time;time.sleep(30)'])
        try:
            observed = process_identity.identity(child.pid)
            self.assertTrue(process_identity.matches(observed))
        finally:
            child.terminate()
            child.wait()
        self.assertFalse(process_identity.matches(observed))

    def test_redaction_permissions_and_referenced_retention(self):
        self.logs.mkdir(mode=0o755)
        os.chmod(self.logs, 0o755)
        with operation_log.Operation(self.root, 'private') as operation:
            operation.event('fixture', password='sensitive', nested={'api_token': 'sensitive'},
                            url='https://user:sensitive@example.test/?token=sensitive',
                            repository_url='user:sensitive@example.test/repository',
                            environment_fingerprint='retained-digest')
        contents = operation.path.read_text()
        self.assertNotIn('sensitive', contents)
        self.assertIn('retained-digest', contents)
        self.assertEqual(operation.path.stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.logs.stat().st_mode & 0o777, 0o755)
        trace = perf_log.CiTrace(self.root, {}, log_root=self.logs)
        with trace.span('referenced'):
            pass
        trace.finish('passed', 0)
        related = list(self.logs.glob('operations/**/*.jsonl'))
        perf_log.enforce_retention(self.logs, 0, {trace.path})
        retained = [path for path in related if path.exists()]
        self.assertEqual(len(retained), 2)
        trace.path.unlink()
        perf_log.enforce_retention(self.logs, 0)
        self.assertFalse(any(path.exists() for path in related))


if __name__ == '__main__':
    unittest.main()
